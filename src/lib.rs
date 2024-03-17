// SPDX-License-Identifier: MIT OR Apache-2.0
#![feature(ptr_metadata, layout_for_ptr, unsize)]
#![no_std]
//! [![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/miny)
//! [![Crates.io](https://img.shields.io/crates/v/miny)](https://crates.io/crates/miny)
//! [![docs.rs](https://img.shields.io/docsrs/miny)](https://docs.rs/miny)
//! [![MIT OR Apache-2.0](https://img.shields.io/crates/l/miny)](#LICENSE)
//!
//! A [`Miny<T>`][^1] is like a [`Box<T>`] with `T` stored inline for values
//! less than a pointer in size. Requires **nightly** Rust[^2] & [`alloc`]
//!
//! # Examples
//! ```
//! # use miny::Miny;
//! let small = Miny::new(1_u8);
//! let large = Miny::new([1_usize; 32]);
//! // small is stored inline on the stack
//! assert!(Miny::on_stack(&small));
//! // large is stored with an allocation
//! assert!(!Miny::on_stack(&large));
//! // consume the miny and get back a value
//! let original = Miny::into_inner(large);
//! assert_eq!(original, [1; 32]);
//! ```
//! To use unsized values, call [`unsize`] with a type or use the
//! [`new_unsized`] shorthand[^3]
//! ```
//! # use miny::Miny;
//! let value = Miny::new_unsized::<[usize]>([1_usize; 32]);
//! // it's usable as a [usize]
//! assert_eq!(value.len(), 32);
//! // and you can consume it to get a boxed value
//! let boxed = Miny::into_box(value);
//! assert_eq!(boxed, Box::new([1_usize; 32]) as Box<[usize]>);
//! ```
//! Or if you have a box you can directly convert it into a [`Miny`]
//! ```
//! # use miny::Miny;
//! let large = Miny::from(Box::new([1_usize; 32]) as Box<[usize]>);
//! assert_eq!(large.len(), 32);
//! // this is slightly inefficient as it boxes and then un-boxes the value,
//! // prefer using `new` / `new_unsized` for this
//! let small = Miny::from(Box::new([1_u8, 2]) as Box<[u8]>); assert_eq!(small.len(), 2);
//! ```
//!
//! [^1]:
//! The name is because it originally was just a "mini `Box<dyn Any>`", although
//! it supports any type
//!
//! [^2]:
//! Uses [`ptr_metadata`] (Reading the metadata pointer & storing it),
//! [`layout_for_ptr`] (Determining value size without reading the value), and
//! [`unsize`](https://github.com/rust-lang/rust/issues/18598) (`new_unsized` & `unsize` functions) features
//!
//! [^3]:
//! This is needed because the [`Miny`] layout is [too weird] for
//! [`CoerceUnsized`] to work properly
//!
//! [`new_unsized`]: Miny::new_unsized
//! [`unsize`]: Miny::unsize
//! [too weird]: ../src/miny/lib.rs.html#79-83
//! [`ptr_metadata`]: https://github.com/rust-lang/rust/issues/81513
//! [`layout_for_ptr`]: https://github.com/rust-lang/rust/issues/69835
//! [`CoerceUnsized`]: https://doc.rust-lang.org/nightly/core/ops/trait.CoerceUnsized.html

use core::alloc::Layout;
use core::marker::{PhantomData, Unsize};
use core::mem::{self, MaybeUninit};
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull, Pointee};

extern crate alloc;

use alloc::alloc::handle_alloc_error;
use alloc::boxed::Box;

#[cfg(test)]
mod tests;

mod r#impl;

/// [`Box<T>`] but with small data stored inline
///
/// See the [crate docs](crate) for more
pub struct Miny<T: ?Sized> {
	meta: <T as Pointee>::Metadata,
	data: MaybeUninit<*mut ()>,
	marker: PhantomData<T>,
}

const fn goes_on_stack(layout: Layout) -> bool {
	layout.size() <= mem::size_of::<*mut ()>() && layout.align() <= mem::align_of::<*mut ()>()
}

impl<T> Miny<T> {
	/// Construct a new instance from a sized value
	pub fn new(value: T) -> Self {
		Self {
			meta: (),
			data: if goes_on_stack(Layout::new::<T>()) {
				let mut data = MaybeUninit::<*mut ()>::uninit();
				// SAFETY: goes_on_stack determines that the value is small enough & aligned
				// enough to fit
				unsafe { ptr::write(data.as_mut_ptr().cast::<T>(), value) };
				data
			} else {
				MaybeUninit::new(Box::into_raw(Box::new(value)).cast::<()>())
			},
			marker: PhantomData,
		}
	}
	/// Attach the appropriate metadata to turn the value into an unsized value
	#[inline]
	#[must_use]
	pub fn unsize<S: ?Sized>(this: Self) -> Miny<S>
	where
		T: Unsize<S>,
	{
		// reason: this breaks if i use addr_of!
		#![allow(clippy::borrow_as_ptr)]
		let meta = ptr::metadata(&*this as *const S);
		let data = this.data;
		mem::forget(this);
		Miny {
			meta,
			data,
			marker: PhantomData,
		}
	}
	/// Shorthand for `Miny::unsize(Miny::new(v))`,
	/// or `Miny::from(Box::new(v) as S)`
	#[inline]
	#[must_use]
	pub fn new_unsized<S: ?Sized>(value: T) -> Miny<S>
	where
		T: Unsize<S>,
	{
		Self::unsize(Self::new(value))
	}
	/// Consume the `Miny` and take the value out,
	/// equivalent to box's deref move
	#[must_use]
	pub fn into_inner(this: Self) -> T {
		if goes_on_stack(Layout::new::<T>()) {
			let data = this.data;
			mem::forget(this);
			// SAFETY: stacked data is valid for a single read
			unsafe { ptr::read(data.as_ptr().cast()) }
		} else {
			// SAFETY: returned false, value is on heap
			*unsafe { Self::heap_into_box(this) }
		}
	}
}

impl<T: ?Sized> Miny<T> {
	/// # Safety
	/// must be on heap
	#[inline]
	unsafe fn heap_into_box(mut this: Self) -> Box<T> {
		// SAFETY: allocated data is the same as a box
		let res = unsafe { Box::from_raw(this.as_mut() as *mut T) };
		mem::forget(this);
		res
	}
	/// Get the layout of the inner value, not really useful for much except for
	/// maybe some unsafe things.
	#[inline]
	pub fn layout(this: &Self) -> Layout {
		// SAFETY: fine as long as `Layout::for_value_raw` never reads the value, which
		// it doesn't so far
		unsafe {
			Layout::for_value_raw(ptr::from_raw_parts::<T>(
				NonNull::dangling().as_ptr(),
				this.meta,
			))
		}
	}
	/// [`true`] if the value is stored inline on the stack instead of with a
	/// heap allocation, not really useful for much except for maybe some unsafe
	/// things or as a diagnostic tool.
	#[inline]
	pub fn on_stack(this: &Self) -> bool {
		goes_on_stack(Self::layout(this))
	}
	// we can't impl From<Miny<T>> for Box<T> for fun reasons so instead we get this
	/// Consume the `Miny` and take the value out as a [`Box`], as opposed to
	/// [`into_inner`] it also works on unsized values.
	///
	/// [`into_inner`]: Self::into_inner
	pub fn into_box(this: Self) -> Box<T> {
		if Self::on_stack(&this) {
			let layout = Self::layout(&this);
			let data = if layout.size() == 0 {
				NonNull::dangling()
			} else {
				// SAFETY: size is non-zero, also alloc alloc alloc :)
				NonNull::new(unsafe { alloc::alloc::alloc(layout) })
					.unwrap_or_else(|| handle_alloc_error(layout))
			};
			let src = this.data.as_ptr().cast::<u8>();
			// SAFETY: we just allocated the same layout for the value
			unsafe { ptr::copy_nonoverlapping(src, data.as_ptr(), layout.size()) };
			let meta = this.meta;
			mem::forget(this);
			// SAFETY: data was allocated with the global allocator
			unsafe { Box::from_raw(ptr::from_raw_parts_mut(data.as_ptr().cast(), meta)) }
		} else {
			// SAFETY: returned false, value is already on the heap
			unsafe { Self::heap_into_box(this) }
		}
	}
}

impl<T: ?Sized> Deref for Miny<T> {
	type Target = T;
	fn deref(&self) -> &Self::Target {
		let data = if Self::on_stack(self) {
			self.data.as_ptr().cast::<()>()
		} else {
			// SAFETY: on the heap, full ptr is used
			unsafe { self.data.assume_init() }
		};
		// SAFETY: valid data and meta
		unsafe { &*ptr::from_raw_parts(data, self.meta) }
	}
}
impl<T: ?Sized> DerefMut for Miny<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		let data = if Self::on_stack(self) {
			self.data.as_mut_ptr().cast::<()>()
		} else {
			// SAFETY: on the heap, full ptr is used
			unsafe { self.data.assume_init() }
		};
		// SAFETY: valid data and meta
		unsafe { &mut *ptr::from_raw_parts_mut(data, self.meta) }
	}
}

impl<T: ?Sized> From<Box<T>> for Miny<T> {
	fn from(value: Box<T>) -> Self {
		let layout = Layout::for_value(&*value);
		let (val, meta) = Box::into_raw(value).to_raw_parts();
		if goes_on_stack(layout) {
			let mut data = MaybeUninit::<*mut ()>::uninit();
			// using u8 as it's a one-byte value, maybe there's something better for this?
			let dst = data.as_mut_ptr().cast::<u8>();
			// SAFETY: we just created the data and the value is small enough to fit
			unsafe { ptr::copy_nonoverlapping(val.cast::<u8>(), dst, layout.size()) };
			// SAFETY: box has been consumed already
			unsafe { alloc::alloc::dealloc(val.cast::<u8>(), layout) };
			Self {
				meta,
				data,
				marker: PhantomData,
			}
		} else {
			Self {
				meta,
				data: MaybeUninit::new(val),
				marker: PhantomData,
			}
		}
	}
}

impl<T: ?Sized> Drop for Miny<T> {
	fn drop(&mut self) {
		if Self::on_stack(self) {
			// SAFETY: valid value and we don't use it again
			unsafe { ptr::drop_in_place(self.as_mut()) };
		} else {
			// SAFETY: heap value is equivalent to box
			drop(unsafe { Box::from_raw(self.as_mut()) });
		}
	}
}
