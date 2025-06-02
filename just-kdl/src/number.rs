// SPDX-License-Identifier: MIT OR Apache-2.0
//! Number representation
//!
//! This is a similar approach as used in other serialization libraries I found

use std::fmt;
use std::mem::discriminant;
use std::num::FpCategory;
use std::str::FromStr;

use thiserror::Error;

use crate::stream::parse_number;

/// An opaque numeric value
///
/// Guaranteed to contain at minimum `u64 ∪ i64 ∪ f64`,
/// might contain more in the future
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Number(NumberInner);

#[derive(Clone, Copy)]
// TODO: this number format could be better
// maybe using a bigint? or just i128?
// or some abstract representation of digits
enum NumberInner {
	Float(f64),
	Unsigned(u64),
	Signed(i64),
}

// evil comparison functions >:3
fn norm_float(v: f64) -> u64 {
	match v.classify() {
		FpCategory::Nan => u64::MAX,
		FpCategory::Zero => 0,
		FpCategory::Infinite | FpCategory::Subnormal | FpCategory::Normal => v.to_bits(),
	}
}
impl PartialEq for NumberInner {
	fn eq(&self, other: &Self) -> bool {
		match (*self, *other) {
			(Self::Float(l), Self::Float(r)) => norm_float(l) == norm_float(r),
			(Self::Unsigned(l), Self::Unsigned(r)) => l == r,
			(Self::Signed(l), Self::Signed(r)) => l == r,
			_ => false,
		}
	}
}
impl Eq for NumberInner {}
impl std::hash::Hash for NumberInner {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		discriminant(self).hash(state);
		match *self {
			NumberInner::Float(v) => norm_float(v).hash(state),
			NumberInner::Unsigned(v) => v.hash(state),
			NumberInner::Signed(v) => v.hash(state),
		}
	}
}

// these template values exist for the parser
impl Number {
	pub(crate) fn from_f64(v: f64) -> Self { Self(NumberInner::Float(v)) }
	pub(crate) fn from_u64(v: u64) -> Self { Self(NumberInner::Unsigned(v)) }
	pub(crate) fn from_i64(v: i64) -> Self { Self(NumberInner::Signed(v)) }
}

impl fmt::Display for Number {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.0 {
			NumberInner::Float(v) => match v.classify() {
				FpCategory::Nan => f.write_str("#nan"),
				FpCategory::Infinite => f.write_str(if v.is_sign_negative() {
					"#-inf"
				} else {
					"#inf"
				}),
				FpCategory::Zero | FpCategory::Subnormal | FpCategory::Normal => {
					// use debug fmt to ensure that floats get re-parsed as floats
					fmt::Debug::fmt(&v, f)
				}
			},
			NumberInner::Unsigned(v) => fmt::Display::fmt(&v, f),
			NumberInner::Signed(v) => fmt::Display::fmt(&v, f),
		}
	}
}
impl fmt::Debug for Number {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str("Number(")?;
		match self.0 {
			NumberInner::Float(v) => fmt::Debug::fmt(&v, f),
			NumberInner::Unsigned(v) => fmt::Debug::fmt(&v, f),
			NumberInner::Signed(v) => fmt::Debug::fmt(&v, f),
		}?;
		f.write_str(")")
	}
}
macro_rules! impl_from {
	($k:ident $t:ident) => {
		impl TryFrom<Number> for $t {
			type Error = NumberError;
			fn try_from(value: Number) -> Result<Self, Self::Error> {
				match value.0 {
					NumberInner::$k(value) => value.try_into().map_err(|_| NumberError::OutOfRange),
					_ => Err(NumberError::OutOfRange),
				}
			}
		}
	};
}
impl_from!(Unsigned u8);
impl_from!(Unsigned u16);
impl_from!(Unsigned u32);
impl_from!(Unsigned u64);
impl_from!(Unsigned u128);
impl_from!(Signed i8);
impl_from!(Signed i16);
impl_from!(Signed i32);
impl_from!(Signed i64);
impl_from!(Signed i128);
//impl_from!(Float f32);
impl_from!(Float f64);

/// Whoops! You need to use the correct number format!
#[derive(Debug, Error)]
#[expect(missing_docs, reason = "error impl")]
pub enum NumberError {
	#[error("Number out of range")]
	OutOfRange,
	#[error("Bad number syntax")]
	BadSyntax,
}

impl FromStr for Number {
	type Err = NumberError;
	/// Parses a number from kdl-equivalent string
	fn from_str(text: &str) -> Result<Self, Self::Err> { parse_number(text) }
}
