// SPDX-License-Identifier: MIT OR Apache-2.0
//! trait forwards, in a module to not mess up the imports
use core::any::{Any, TypeId};
use core::borrow::{Borrow, BorrowMut};
use core::cmp::Ordering;
use core::fmt::{Debug, Display, Formatter, Pointer, Result as FmtResult};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::{mem, ptr};

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
// impl<T: ?Sized> Clone for Miny<T> where Box<T>: Clone
// & then specialize for normal T
impl<T: Clone> Clone for Miny<T> {
	#[inline]
	fn clone(&self) -> Self { Self::new((**self).clone()) }
	#[inline]
	fn clone_from(&mut self, source: &Self) { (**self).clone_from(source); }
}
impl<T: Default> Default for Miny<T> {
	#[inline]
	fn default() -> Self { Self::new(T::default()) }
}
impl<T: ?Sized + Debug> Debug for Miny<T> {
	#[inline]
	fn fmt(&self, fmt: &mut Formatter) -> FmtResult { Debug::fmt(&**self, fmt) }
}
impl<T: ?Sized + Display> Display for Miny<T> {
	#[inline]
	fn fmt(&self, fmt: &mut Formatter) -> FmtResult { Display::fmt(&**self, fmt) }
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
	fn eq(&self, other: &Self) -> bool { (**self).eq(&**other) }
}
impl<T: ?Sized + Eq> Eq for Miny<T> {}
impl<T: ?Sized + PartialOrd> PartialOrd for Miny<T> {
	#[inline]
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> { (**self).partial_cmp(&**other) }
}
impl<T: ?Sized + Ord> Ord for Miny<T> {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering { (**self).cmp(&**other) }
}
impl<T: ?Sized + Hash> Hash for Miny<T> {
	#[inline]
	fn hash<H: Hasher>(&self, state: &mut H) { (**self).hash(state); }
}
// SAFETY: pointee metadata is Send + Sync,
// the only time the pointer/inline can change is if it's mutable
unsafe impl<T: ?Sized + Sync> Sync for Miny<T> {}
// SAFETY: should be fine, ownership is passed along with the pointer / value
unsafe impl<T: ?Sized + Send> Send for Miny<T> {}

/// Extra implementations for `Box<dyn Any>`-like behavior.
///
/// If you have a `dyn Any + OtherTrait` or `dyn SuperTrait`
/// (where `SuperTrait: Any`), you can use [`Miny::unsize<dyn Any>`] first.
impl Miny<dyn Any> {
	/// Downcasts the value to a concrete type
	/// # Safety
	/// The contained value must be of type `T`. Calling this method
	/// with the incorrect type is *undefined behavior*.
	pub unsafe fn downcast_unchecked<T: Any>(self) -> Miny<T> {
		debug_assert!(self.is::<T>(), "unchecked cast was wrong!");
		// cursed as hell
		let data = self.data;
		mem::forget(self);
		Miny::<T> {
			data,
			meta: (),
			marker: PhantomData,
		}
	}
	/// Attempts to downcast the value to a concrete type, returning the
	/// original instance if not.
	#[expect(clippy::missing_errors_doc, reason = "non-error result")]
	pub fn downcast<T: Any>(self) -> Result<Miny<T>, Self> {
		if self.is::<T>() {
			// SAFETY: asserted type matches
			Ok(unsafe { self.downcast_unchecked() })
		} else {
			Err(self)
		}
	}
}

#[cfg(feature = "allocative")]
impl<T> allocative::Allocative for Miny<T>
where
	T: ?Sized + allocative::Allocative,
{
	fn visit<'visitor, 'contents: 'visitor>(
		&self,
		visitor: &'visitor mut allocative::Visitor<'contents>,
	) {
		use core::ptr::Pointee;

		let mut visitor = visitor.enter_self_sized::<Self>();
		let meta_size = mem::size_of::<<T as Pointee>::Metadata>();
		if meta_size > 0 {
			// metadata doesn't have anything we can inspect
			visitor.visit_field_with(allocative::Key::new("meta"), meta_size, |_| {});
		}
		if Self::on_stack(self) {
			visitor.visit_field(allocative::Key::new("inline"), &**self);
		} else {
			// using *mut () instead of *mut T, since we already visited the metadata
			let mut visitor =
				visitor.enter_unique(allocative::Key::new("ptr"), mem::size_of::<*mut ()>());
			T::visit(self, &mut visitor);
		}
		visitor.exit();
	}
}
