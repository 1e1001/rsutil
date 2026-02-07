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

/// The whole point, see the [crate docs](`crate`).
///
/// Note that an iterator can only be debugged once, aim to wrap your iterator
/// as late as possible, usually directly in the print / format statement.
pub struct IterDebug<T>(Cell<Option<T>>);

impl<T> IterDebug<T> {
	/// Construct a new instance directly, instead of using the
	/// [`debug`](DebugIterator::debug) method.
	#[inline]
	pub fn new(item: T) -> Self { Self(Cell::new(Option::Some(item))) }
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
		match self.0.take() {
			Option::Some(value) => f.debug_list().entries(value).finish(),
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
	/// Convert this iterator into an [`IterDebug`] for printing
	fn debug(self) -> IterDebug<Self>
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
}
