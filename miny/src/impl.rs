// SPDX-License-Identifier: MIT
//! trait forwards, in a module to not mess up the imports
//! because i need somewhere to put this, here's the readme:
#![doc = include_str!("../README.md")]
use core::any::{Any, TypeId};
use core::borrow::{Borrow, BorrowMut};
use core::cmp::Ordering;
use core::fmt::{Debug, Display, Formatter, Pointer, Result as FmtResult};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem;

use crate::Miny;
macro_rules! impl_refs {
	($method:ident $trait:ident $($kw:tt)?) => {
		impl<T: ?Sized> $trait<T> for Miny<T> {
			#[inline]
			fn $method(&$($kw)? self) -> &$($kw)? T {
				self
			}
		}
	};
}

impl_refs!(as_ref AsRef);
impl_refs!(as_mut AsMut mut);
impl_refs!(borrow Borrow);
impl_refs!(borrow_mut BorrowMut mut);

// TODO: once we get specialization I could probably
// impl<T> Clone for Miny<T> where Box<T>: Clone & then specialize for normal T
impl<T: Clone> Clone for Miny<T> {
	#[inline]
	fn clone(&self) -> Self {
		Self::new((**self).clone())
	}
	#[inline]
	fn clone_from(&mut self, source: &Self) {
		(**self).clone_from(source);
	}
}
impl<T: Default> Default for Miny<T> {
	#[inline]
	fn default() -> Self {
		Self::new(T::default())
	}
}
impl<T: ?Sized + Debug> Debug for Miny<T> {
	#[inline]
	fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
		Debug::fmt(&**self, fmt)
	}
}
impl<T: ?Sized + Display> Display for Miny<T> {
	#[inline]
	fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
		Display::fmt(&**self, fmt)
	}
}
impl<T: ?Sized> Pointer for Miny<T> {
	fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
		if Self::on_stack(self) {
			fmt.write_str("<stack>")
		} else {
			Pointer::fmt(&core::ptr::addr_of!(**self), fmt)
		}
	}
}
impl<T: ?Sized + PartialEq> PartialEq for Miny<T> {
	#[inline]
	fn eq(&self, other: &Self) -> bool {
		(**self).eq(&**other)
	}
}
impl<T: ?Sized + Eq> Eq for Miny<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Miny<T> {
	#[inline]
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		(**self).partial_cmp(&**other)
	}
}
impl<T: ?Sized + Ord> Ord for Miny<T> {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering {
		(**self).cmp(&**other)
	}
}
impl<T: ?Sized + Hash> Hash for Miny<T> {
	#[inline]
	fn hash<H: Hasher>(&self, state: &mut H) {
		(**self).hash(state);
	}
}
// TODO: maybe add a check that <P as Pointee>::Metadata is also thread-safe?
// SAFETY: should be fine, a reference to the thing should never observe any
// funky things
unsafe impl<T: ?Sized + Sync> Sync for Miny<T> {}
// SAFETY: should be fine, ownership is passed along with the pointer / value
unsafe impl<T: ?Sized + Send> Send for Miny<T> {}

/// Extra implementations for `Box<dyn Any>`-like behavior
impl<T: ?Sized + Any> Miny<T> {
	/// Returns `true` if the inner type is the same as `T`.
	#[inline]
	pub fn is<V: Any>(&self) -> bool {
		TypeId::of::<V>() == (**self).type_id()
	}
	/// Returns a reference to the inner value
	/// # Safety
	/// The contained value must be of type `T`. Calling this method
	/// with the incorrect type is *undefined behavior*.
	pub unsafe fn downcast_ref_unchecked<V: Any>(&self) -> &V {
		debug_assert!(self.is::<V>(), "unchecked cast was wrong!");
		&*(self.as_ref() as *const T).cast::<V>()
	}
	/// Returns a mutable reference to the inner value
	/// # Safety
	/// The contained value must be of type `T`. Calling this method
	/// with the incorrect type is *undefined behavior*.
	pub unsafe fn downcast_mut_unchecked<V: Any>(&mut self) -> &mut V {
		debug_assert!(self.is::<V>(), "unchecked cast was wrong!");
		&mut *(self.as_mut() as *mut T).cast::<V>()
	}
	/// Downcasts the value to a concrete type
	/// # Safety
	/// The contained value must be of type `T`. Calling this method
	/// with the incorrect type is *undefined behavior*.
	pub unsafe fn downcast_unchecked<V: Any>(self) -> V {
		debug_assert!(self.is::<V>(), "unchecked cast was wrong!");
		// cursed as hell
		let data = self.data;
		mem::forget(self);
		Miny::into_inner(Miny::<V> {
			data,
			meta: (),
			marker: PhantomData,
		})
	}
	/// Returns a reference to the inner value if it is of type `T`, or `None`
	/// if it isn't
	pub fn downcast_ref<V: Any>(&self) -> Option<&V> {
		self.is::<V>().then(||
			// SAFETY: asserted type matches
			unsafe { self.downcast_ref_unchecked() })
	}
	/// Returns a mutable reference to the inner value if it is of type `T`, or
	/// `None` if it isn't
	pub fn downcast_mut<V: Any>(&mut self) -> Option<&mut V> {
		self.is::<V>().then(||
			// SAFETY: asserted type matches
			unsafe { self.downcast_mut_unchecked() })
	}
	/// Attempts to downcast the value to a concrete type, returning the
	/// original instance if not
	pub fn downcast<V: Any>(self) -> Result<V, Self> {
		if self.is::<V>() {
			// SAFETY: asserted type matches
			Ok(unsafe { self.downcast_unchecked() })
		} else {
			Err(self)
		}
	}
}
