// SPDX-License-Identifier: MIT OR Apache-2.0
#![expect(incomplete_features, reason = "whoops")]
#![feature(generic_const_exprs, test)]
// needed for a test
#![recursion_limit = "512"]
#![no_std]
//! # punch-card
//!
//! [![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/rsutil/tree/main/punch-card)
//! [![Crates.io](https://img.shields.io/crates/v/punch-card)](https://crates.io/crates/punch-card)
//! [![docs.rs](https://img.shields.io/docsrs/punch-card)](https://docs.rs/punch-card)
//! [![MIT OR Apache-2.0](https://img.shields.io/crates/l/punch-card)](https://github.com/1e1001/rsutil/blob/main/punch-card/README.md#License)
//!
//! A library for making punched cards like this:
//!
//! ```rust
//! use punch_card::PunchCard;
//!
//! #[rustfmt::skip]
//! println!("{}", std::str::from_utf8(&(
//!     .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..,
//!     ..=..=..=..=..=.. .. .. ..=..=..=..=..=..=.. ..=..=.. ..=..=..=..=..=..=.. ..=..=..=..=..=.. ..=..=..=..=..,
//!     ..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..,
//!     .. ..=..=..=..=..=.. .. .. ..=.. ..=.. ..=.. .. .. .. .. ..=.. ..=.. ..=.. ..=..=.. .. .. .. .. .. ..=.. ..,
//!     ..=.. .. .. .. ..=..=..=.. .. .. .. .. .. ..=..=..=..=.. .. .. .. .. .. ..=.. .. ..=.. ..=..=.. .. .. .. ..,
//!     .. ..=..=.. .. .. ..=..=.. .. .. ..=..=.. ..=.. ..=..=.. .. .. ..=..=.. ..=.. ..=..=.. .. ..=.. .. .. ..=..,
//!     .. .. .. .. ..=..=..=..=..=..=.. .. .. ..=..=.. ..=..=..=..=.. .. .. ..=..=.. .. ..=..=.. .. ..=.. ..=.. ..,
//!     .. .. .. .. ..=.. ..=..=..=.. ..=.. ..=..=.. ..=..=..=..=.. ..=.. ..=..=..=.. ..=.. ..=.. ..=..=..=.. .. ..,
//! ).punch_card()).unwrap());
//! ```
//!
//! ## Why?
//!
//! I saw the `punch_card` example in [`weird-exprs.rs`] and (inspired by
//! [`analog_literals`]) thought "what if that was useful?" and then created
//! this.
//!
//! ## Usage
//!
//! Run [`.punch_card()`](PunchCard::punch_card) on a card tuple to convert it
//! into an array of values
//!
//! By default, punch-card supports the following sizes of card:
//!
//! - *n* &times; 1 &rarr; array of [`bool`]
//! - *n* &times; 8 &rarr; array of [`u8`] (probably the one you'll be using the
//!   most)
//! - *n* &times; 16 &rarr; array of [`u16`]
//! - *n* &times; 32 &rarr; array of [`u32`]
//! - *n* &times; 64 &rarr; array of [`u64`]
//! - *n* &times; 128 &rarr; array of [`u128`]
//!
//! A card is simply a tuple of some amount of rows, where each row is a chain
//! of `..`'s or `..=`'s terminated by a `..`, as shown in the above example.
//!
//! *Note: this uses the [`generic_const_exprs`](https://github.com/rust-lang/rust/issues/76560) feature, it should be safe to use though.*
//!
//! [`analog_literals`]: <https://crates.io/crates/analog_literals>
//! [`weird-exprs.rs`]: <https://github.com/rust-lang/rust/blob/bdcb6a99e853732f8ec050ae4986aa3af51d44c5/src/test/ui/weird-exprs.rs#L123-L131>

use internal::PunchCardInner;

mod internal;
#[cfg(test)]
mod tests;

/// Represents a value that is a punch card, formatted like this:
/// ```rust
/// # use punch_card::PunchCard;
/// #
/// # #[rustfmt::skip]
/// # println!("{}", std::str::from_utf8(&
/// (
///     .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..,
///     ..=..=..=..=..=.. .. ..=..=..=..=..=.. .. ..,
///     .. ..=..=..=..=..=..=.. ..=..=..=..=..=.. ..,
///     .. .. .. .. .. .. .. ..=.. ..=.. .. .. .. ..,
///     ..=.. ..=..=..=..=.. .. ..=.. ..=.. .. ..=..,
///     .. ..=..=..=..=..=.. ..=..=.. ..=..=.. .. ..,
///     .. .. .. .. ..=.. .. ..=..=..=.. .. .. ..=..,
///     .. ..=.. .. ..=.. .. ..=..=.. .. .. ..=.. ..,
/// ).punch_card()
/// # ).unwrap());
/// ```
/// An `=` indicates a one bit and a space indicates a zero bit.
///
/// Automatically implemented for punched cards of heights
/// 1, 8, 16, 32, 64, and 128.
pub trait PunchCard {
	#[doc(hidden)]
	const LENGTH: usize;
	/// Type for each column of the tape
	type Output;
	/// Parses the punch card into your output format of choice.
	fn punch_card(&self) -> [Self::Output; Self::LENGTH];
}

impl<T: PunchCardInner> PunchCard for T {
	const LENGTH: usize = T::LENGTH;
	type Output = T::Output;
	fn punch_card(&self) -> [Self::Output; <Self as PunchCard>::LENGTH] {
		let mut out = [Default::default(); <Self as PunchCard>::LENGTH];
		Self::eval_part(&mut out, 0);
		out
	}
}
