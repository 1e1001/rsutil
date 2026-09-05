// SPDX-License-Identifier: MIT OR Apache-2.0
#![no_std]
#![no_implicit_prelude]
//! [![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/rsutil/tree/main/iter-debug)
//! [![Crates.io](https://img.shields.io/crates/v/iter-debug)](https://crates.io/crates/iter-debug)
//! [![docs.rs](https://img.shields.io/docsrs/iter-debug)](https://docs.rs/iter-debug)
//! [![MIT OR Apache-2.0](https://img.shields.io/crates/l/iter-debug)](https://github.com/1e1001/rsutil/blob/main/iter-debug/README.md#License)
//!
//! Allows debugging iterators without collecting them to a [`Vec`] first,
//! useful in `no_std` environments or when you're lazy.
//! ```rust
//! # use iter_debug::DebugIterator;
//! println!("{:?}", [1, 2, 3, 4].into_iter().map(|v| v * 2).debug());
//! // => [2, 4, 6, 8]
//! ```
//!
//! [`Vec`]: <https://doc.rust-lang.org/nightly/std/vec/struct.Vec.html>
extern crate core;

use core::cell::Cell;
use core::fmt::{Debug, Formatter, Result};
use core::iter::IntoIterator;
use core::marker::Sized;
use core::option::Option;

#[cfg(test)]
mod tests;

enum IterDebugStyle {
	List,
	Set,
}

/// The whole point, see the [crate docs](`crate`).
///
/// Note that an iterator can only be debugged once, aim to wrap your iterator
/// as late as possible, usually directly in the print / format statement.
pub struct IterDebug<T>(Cell<Option<T>>, IterDebugStyle);

impl<T> IterDebug<T> {
	/// Construct a new instance directly, instead of using the
	/// [`debug`](DebugIterator::debug) method. Prints output as a list.
	#[inline]
	pub fn new(item: T) -> Self { Self(Cell::new(Option::Some(item)), IterDebugStyle::List) }
	/// Construct a new instance directly, instead of using the
	/// [`debug`](DebugIterator::debug) method. Prints output as a set.
	#[inline]
	pub fn new_set(item: T) -> Self { Self(Cell::new(Option::Some(item)), IterDebugStyle::Set) }
	/// Attempt to extract the inner iterator, returning [`None`](Option::None)
	/// if it has already been removed or debug printed.
	#[inline]
	pub fn try_into_inner(&self) -> Option<T> { self.0.take() }
}

impl<T> Debug for IterDebug<T>
where
	T: IntoIterator,
	T::Item: Debug,
{
	#[inline]
	fn fmt(&self, f: &mut Formatter) -> Result {
		match (self.0.take(), &self.1) {
			(Option::Some(value), IterDebugStyle::List) => f.debug_list().entries(value).finish(),
			(Option::Some(value), IterDebugStyle::Set) => f.debug_set().entries(value).finish(),
			(Option::None, _) => f.write_str("<consumed iterator>"),
		}
	}
}

/// [`IterDebug`] but styled with key-value formatting
///
/// Note that an iterator can only be debugged once, aim to wrap your iterator
/// as late as possible, usually directly in the print / format statement.
pub struct KvIterDebug<T>(Cell<Option<T>>);

impl<T> KvIterDebug<T> {
	/// Construct a new instance directly, instead of using the
	/// [`debug`](DebugIterator::debug) method. Prints output as a map.
	#[inline]
	pub fn new(item: T) -> Self { Self(Cell::new(Option::Some(item))) }
	/// Attempt to extract the inner iterator, returning [`None`](Option::None)
	/// if it has already been removed or debug printed.
	#[inline]
	pub fn try_into_inner(&self) -> Option<T> { self.0.take() }
}

impl<T, K, V> Debug for KvIterDebug<T>
where
	T: IntoIterator<Item = (K, V)>,
	K: Debug,
	V: Debug,
{
	#[inline]
	fn fmt(&self, f: &mut Formatter) -> Result {
		match self.0.take() {
			Option::Some(value) => f.debug_map().entries(value).finish(),
			Option::None => f.write_str("<consumed iterator>"),
		}
	}
}

/// Helper trait that lets you `.debug()` an iterator, like the other
/// combinators.
///
/// Automatically implemented for all [`IntoIterator`] with [`Debug`]-able
/// items.
pub trait DebugIterator {
	/// Convert this iterator to a [`Debug`]-printable value, printed as a list.
	fn debug(self) -> IterDebug<Self>
	where
		Self: Sized;
	/// Convert this iterator to a [`Debug`]-printable value, printed as a set.
	fn debug_set(self) -> IterDebug<Self>
	where
		Self: Sized;
	/// Convert this iterator to a [`Debug`]-printable value, printed as a map.
	fn debug_map(self) -> KvIterDebug<Self>
	where
		Self: Sized;
}
impl<T> DebugIterator for T
where
	T: IntoIterator,
	T::Item: Debug,
{
	#[inline]
	fn debug(self) -> IterDebug<Self>
	where
		Self: Sized,
	{
		IterDebug::new(self)
	}
	#[inline]
	fn debug_set(self) -> IterDebug<Self>
	where
		Self: Sized,
	{
		IterDebug::new_set(self)
	}
	fn debug_map(self) -> KvIterDebug<Self>
	where
		Self: Sized,
	{
		KvIterDebug::new(self)
	}
}
