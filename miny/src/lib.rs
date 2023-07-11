// SPDX-License-Identifier: MIT
#![deny(missing_docs)]
//! [![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/miny)
//! [![Crates.io](https://img.shields.io/crates/v/miny)](https://crates.io/crates/miny)
//! [![docs.rs](https://img.shields.io/docsrs/miny)](https://docs.rs/miny)
//! [![MIT OR Apache-2.0](https://img.shields.io/crates/l/miny)](#LICENSE)
//!
//! a [`Miny<T>`] is like a [`Box<T>`] with `T` stored inline for values less than a pointer in size.
//! # Examples
//! ```
//!# use miny::Miny;
//! let small = Miny::new(1u8);
//! let large = Miny::new([1usize; 32]);
//! // small is stored inline on the stack
//! assert!(small.on_stack());
//! // large is stored with an allocation
//! assert!(!large.on_stack());
//! // consume the miny and get back a value
//! let original = large.into_inner();
//! assert_eq!(original, [1; 32]);
//! ```
//! to use unsized values, call [`.unsize`] with a type
//! ```
//!# use miny::Miny;
//! let value = Miny::new([1usize; 32]).unsize::<[usize]>();
//! // it's usable as a [usize]
//! assert_eq!(value.len(), 32);
//! // and you can consume it to get a boxed value
//! let boxed = value.into_box();
//! assert_eq!(boxed, Box::new([1usize; 32]) as Box<[usize]>);
//! ```
//! or you can create a box and convert the box to a [`Miny`]
//! ```
//!# use miny::Miny;
//! let large = Miny::from(Box::new([1usize; 32]) as Box<[usize]>);
//! assert_eq!(large.len(), 32);
//! // this is slightly inefficient as it boxes and then un-boxes the value
//! let small = Miny::from(Box::new([1u8, 2]) as Box<[u8]>);
//! assert_eq!(small.len(), 2);
//! ```
//!
//! # Other Info
//! - uses the nightly [`ptr_metadata`], [`layout_for_ptr`], and [`unsize`] features
//! - supports `#![no_std]` with `alloc`
//! - tested with miri, *should* be sound
//! - (the name is because it originally was just a "mini `Box<dyn Any>`")
//!
//! [`ptr_metadata`]: <https://github.com/rust-lang/rust/issues/81513>
//! [`layout_for_ptr`]: <https://github.com/rust-lang/rust/issues/69835>
//! [`unsize`]: <https://github.com/rust-lang/rust/issues/18598>
//! [`.unsize`]: Miny::unsize
#![feature(ptr_metadata, layout_for_ptr, unsize)]
#![no_std]

extern crate alloc;

use alloc::alloc::{handle_alloc_error, Layout};
use alloc::boxed::Box;
use alloc::fmt;
use core::any::{Any, TypeId};
use core::marker::{PhantomData, Unsize};
use core::mem::{self, MaybeUninit};
use core::ptr::{self, NonNull};

#[cfg(test)]
mod tests;

type VTable<T> = <T as ptr::Pointee>::Metadata;

/// `Box<T>` but with small data stored inline
///
/// see the [crate docs](crate) for more
pub struct Miny<T: ?Sized + 'static> {
	meta: VTable<T>,
	// either a pointer to data (allocated as a box) or an inline value small enough to fit
	// only init it it's a pointer
	data: MaybeUninit<*mut ()>,
	_marker: PhantomData<T>,
}

fn goes_on_stack(layout: Layout) -> bool {
	layout.size() <= mem::size_of::<*mut ()>() && layout.align() <= mem::align_of::<*mut ()>()
}

impl<T> Miny<T> {
	/// construct a new instance
	pub fn new(val: T) -> Self {
		unsafe { Self::new_raw(val, |v| v) }
	}
	/// manual [`CoerceUnsized`], convert into an unsized value
	///
	/// [`CoerceUnsized`]: core::ops::CoerceUnsized
	pub fn unsize<U: ?Sized>(self) -> Miny<U>
	where
		T: Unsize<U>,
	{
		let meta = (&*self as *const U).to_raw_parts().1;
		let data = self.data;
		mem::forget(self);
		Miny {
			meta,
			data,
			_marker: PhantomData,
		}
	}
	/// consume the value and return the value
	pub fn into_inner(mut self) -> T {
		let mut out = MaybeUninit::<T>::uninit();
		if self.on_stack() {
			// SAFETY: out is valid and data is small enough
			unsafe { ptr::copy_nonoverlapping(self.data.as_ptr().cast(), out.as_mut_ptr(), 1) };
		} else {
			out.write(*unsafe { Box::from_raw((self.as_mut() as *mut T).cast::<T>()) });
		}
		mem::forget(self);
		// SAFETY: we just initialized out in one of two ways
		unsafe { out.assume_init() }
	}
}

impl<T: ?Sized> Miny<T> {
	/// creates a new instance in fancy internal ways, generally you shouldn't use this
	/// # Safety
	/// `unsize` needs to return a reference which points to the same object as the one passed in, most of the time it can be `|v| v`
	pub unsafe fn new_raw<V>(val: V, unsize: impl FnOnce(&V) -> &T) -> Self {
		// we always need the metadata
		let meta = (unsize(&val) as *const T).to_raw_parts().1;
		if goes_on_stack(Layout::new::<V>()) {
			let mut data = MaybeUninit::<*mut ()>::uninit();
			// SAFETY: we just created the data and the value is small enough to fit
			unsafe { ptr::write(data.as_mut_ptr().cast::<V>(), val) };
			Self {
				meta,
				data,
				_marker: PhantomData,
			}
		} else {
			let data = MaybeUninit::new(Box::into_raw(Box::new(val)) as *mut ());
			Self {
				meta,
				data,
				_marker: PhantomData,
			}
		}
	}
	/// consume the value and return a box
	pub fn into_box(mut self) -> Box<T> {
		let out = if self.on_stack() {
			let layout = self.layout();
			let data = if layout.size() == 0 {
				NonNull::dangling()
			} else {
				// SAFETY: size is non-zero, also alloc alloc alloc :)
				NonNull::new(unsafe { alloc::alloc::alloc(self.layout()) })
					.unwrap_or_else(|| handle_alloc_error(layout))
			};
			// SAFETY: we just allocated the same layout for the value
			unsafe {
				ptr::copy_nonoverlapping(
					self.data.as_ptr().cast::<u8>(),
					data.as_ptr(),
					layout.size(),
				);
			}
			// SAFETY: data was allocated with the global allocator
			unsafe {
				Box::from_raw(ptr::from_raw_parts_mut(
					data.as_ptr().cast::<()>(),
					self.meta,
				))
			}
		} else {
			// SAFETY: it's on-heap and that always uses global box
			unsafe { Box::from_raw(self.as_mut()) }
		};
		mem::forget(self);
		out
	}
	/// Returns the layout of the contained value were it to be allocated
	pub fn layout(&self) -> Layout {
		// SAFETY: fine as long as `Layout::for_value_raw` never reads the value
		unsafe {
			Layout::for_value_raw(ptr::from_raw_parts::<T>(
				NonNull::dangling().as_ptr(),
				self.meta,
			))
		}
	}
	/// `true` if this value is stored inline instead of an allocation
	pub fn on_stack(&self) -> bool {
		goes_on_stack(self.layout())
	}
}
/// utilities for any types
impl<T: ?Sized + Any> Miny<T> {
	/// gets the [`TypeId`] of the inner type
	pub fn type_id(&self) -> TypeId {
		self.as_ref().type_id()
	}
	/// returns `true` if the inner type is the same as `T`.
	pub fn is<V: Any>(&self) -> bool {
		TypeId::of::<V>() == self.type_id()
	}
	/// returns a reference to the inner value
	/// # Safety
	/// The contained value must be of type `T`. Calling this method
	/// with the incorrect type is *undefined behavior*.
	pub unsafe fn downcast_ref_unchecked<V: Any>(&self) -> &V {
		debug_assert!(self.is::<V>());
		&*(self.as_ref() as *const T).cast::<V>()
	}
	/// returns a mutable reference to the inner value
	/// # Safety
	/// The contained value must be of type `T`. Calling this method
	/// with the incorrect type is *undefined behavior*.
	pub unsafe fn downcast_mut_unchecked<V: Any>(&mut self) -> &mut V {
		debug_assert!(self.is::<V>());
		&mut *(self.as_mut() as *mut T).cast::<V>()
	}
	/// downcasts the value to a concrete type
	/// # Safety
	/// The contained value must be of type `T`. Calling this method
	/// with the incorrect type is *undefined behavior*.
	pub unsafe fn downcast_unchecked<V: Any>(self) -> V {
		debug_assert!(self.is::<V>());
		// cursed as hell
		let data = self.data;
		mem::forget(self);
		Miny::<V> {
			data,
			meta: (),
			_marker: PhantomData,
		}
		.into_inner()
	}
	/// returns a reference to the inner value if it is of type `T`, or `None`
	/// if it isn't
	pub fn downcast_ref<V: Any>(&self) -> Option<&V> {
		self.is::<V>()
			.then(|| unsafe { self.downcast_ref_unchecked() })
	}
	/// returns a mutable reference to the inner value if it is of type `T`, or
	/// `None` if it isn't
	pub fn downcast_mut<V: Any>(&mut self) -> Option<&mut V> {
		self.is::<V>()
			.then(|| unsafe { self.downcast_mut_unchecked() })
	}
	/// attempts to downcast the value to a concrete type, returning the
	/// original instance if not
	pub fn downcast<V: Any>(self) -> Result<V, Self> {
		if self.is::<V>() {
			// SAFETY: we just checked that it's a `T
			Ok(unsafe { self.downcast_unchecked() })
		} else {
			Err(self)
		}
	}
}
impl<T: ?Sized> AsRef<T> for Miny<T> {
	fn as_ref(&self) -> &T {
		self
	}
}
impl<T: ?Sized> AsMut<T> for Miny<T> {
	fn as_mut(&mut self) -> &mut T {
		self
	}
}
impl<T: ?Sized> core::borrow::Borrow<T> for Miny<T> {
	fn borrow(&self) -> &T {
		self
	}
}
impl<T: ?Sized> core::borrow::BorrowMut<T> for Miny<T> {
	fn borrow_mut(&mut self) -> &mut T {
		self
	}
}
impl<T: ?Sized> core::ops::Deref for Miny<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		let data = if self.on_stack() {
			(&self.data as *const MaybeUninit<*mut ()>).cast::<()>()
		} else {
			// SAFETY: on the heap
			unsafe { self.data.assume_init() }
		};
		// SAFETY: slice and dyn have the same layout (for now at least)
		unsafe { &*ptr::from_raw_parts(data, self.meta) }
	}
}
impl<T: ?Sized> core::ops::DerefMut for Miny<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		let data = if self.on_stack() {
			(&mut self.data as *mut MaybeUninit<*mut ()>).cast::<()>()
		} else {
			// SAFETY: on the heap
			unsafe { self.data.assume_init() }
		};
		// SAFETY: slice and dyn have the same layout (for now at least)
		unsafe { &mut *ptr::from_raw_parts_mut(data, self.meta) }
	}
}
impl<T: ?Sized> From<Box<T>> for Miny<T> {
	fn from(val: Box<T>) -> Self {
		let layout = Layout::for_value(&*val);
		let (val, meta) = Box::into_raw(val).to_raw_parts();
		if goes_on_stack(layout) {
			let mut data = MaybeUninit::<*mut ()>::uninit();
			// SAFETY: we just created the data and the value is small enough to fit
			unsafe {
				// using u8 as it's a one-byte value, maybe there's something better for this?
				ptr::copy_nonoverlapping(
					val.cast::<u8>(),
					data.as_mut_ptr().cast::<u8>(),
					layout.size(),
				)
			};
			// SAFETY: box has been consumed already
			unsafe { alloc::alloc::dealloc(val.cast::<u8>(), layout) };
			Self {
				meta,
				data,
				_marker: PhantomData,
			}
		} else {
			let data = MaybeUninit::new(val);
			Self {
				meta,
				data,
				_marker: PhantomData,
			}
		}
	}
}
impl<T: ?Sized> Drop for Miny<T> {
	fn drop(&mut self) {
		if self.on_stack() {
			// SAFETY: valid value and we don't use it again
			unsafe { ptr::drop_in_place(self.as_mut()) };
		} else {
			// SAFETY: it's an on-heap value
			drop(unsafe { Box::from_raw(self.as_mut()) });
		}
	}
}

// trait forwards
impl<T: Clone> Clone for Miny<T> {
	fn clone(&self) -> Self {
		Self::new((**self).clone())
	}
	fn clone_from(&mut self, source: &Self) {
		**self = (**source).clone()
	}
}
impl<T: Default> Default for Miny<T> {
	fn default() -> Self {
		Self::new(T::default())
	}
}
impl<T: ?Sized + fmt::Debug> fmt::Debug for Miny<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&**self, f)
	}
}
impl<T: ?Sized + fmt::Display> fmt::Display for Miny<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Display::fmt(&**self, f)
	}
}
impl<T: ?Sized> fmt::Pointer for Miny<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if self.on_stack() {
			write!(f, "<stack>")
		} else {
			fmt::Pointer::fmt(&(&**self as *const T), f)
		}
	}
}
impl<T: ?Sized + PartialEq> PartialEq for Miny<T> {
	fn eq(&self, other: &Self) -> bool {
		**self == **other
	}
}
impl<T: ?Sized + Eq> Eq for Miny<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Miny<T> {
	fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
		(**self).partial_cmp(&**other)
	}
}
impl<T: ?Sized + Ord> Ord for Miny<T> {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		(**self).cmp(&**other)
	}
}
impl<T: ?Sized + core::hash::Hash> core::hash::Hash for Miny<T> {
	fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
		(**self).hash(state)
	}
}
// i have no idea if these are good but box has them and this is pretty much a box so it should be fine
unsafe impl<T: ?Sized + Sync> Sync for Miny<T> {}
unsafe impl<T: ?Sized + Send> Send for Miny<T> {}
