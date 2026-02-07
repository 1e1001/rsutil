// SPDX-License-Identifier: MIT OR Apache-2.0
//! Since the kdl document / stream structure is only needed at the boundary
//! between text and data, a string-based number representation works fine.

// TODO: add tests / fuzzing

use core::fmt;
use core::num::{FpCategory, IntErrorKind, ParseFloatError, ParseIntError};
use core::str::FromStr;

use smol_str::{SmolStr, format_smolstr};

use super::Number;
use crate::lexer::StringOutput;
use crate::ssb2::SmolStrBuilder2;

/* number format:
#.+ = special float
x-?[0-9a-f]+ = base16 int
d-?[0-9]+ = base10 int
o-?[0-9a-f]+ = base8 int
b-?[0-9a-f]+ = base2 int
f-?[0-9]+(.[0-9]+)?(E[0-9]+)? = base10 float
*/
impl Number {
	/// Value for positive infinity.
	pub const INFINITY: Self = Self(SmolStr::new_static("#inf"));
	/// Value for negative infinity.
	pub const NEG_INFINITY: Self = Self(SmolStr::new_static("#-inf"));
	/// Value for not-a-number, unlike floats, this value is equal to itself.
	pub const NAN: Self = Self(SmolStr::new_static("#nan"));
	/// Turn the number into a representation of its contents,
	/// Use this if you want to convert to a custom format.
	pub fn describe(&self) -> Description<'_> {
		let text = &self.0[1..];
		match self.0.as_bytes()[0] {
			b'#' => Description::Special { text: &self.0 },
			b'x' => Description::Integer {
				base: Base::Hexadecimal,
				text,
			},
			b'd' => Description::Integer {
				base: Base::Decimal,
				text,
			},
			b'o' => Description::Integer {
				base: Base::Octal,
				text,
			},
			b'b' => Description::Integer {
				base: Base::Binary,
				text,
			},
			b'f' => Description::Float { text },
			_ => unreachable!(),
		}
	}
	fn coerce_to_float<
		T: FromStr<Err = ParseFloatError>,
		I,
		FSR: Fn(&str, u32) -> Result<I, ParseIntError>,
		CAST: Fn(I) -> T,
	>(
		&self,
		fsr: FSR,
		cast: CAST,
		inf: T,
		neg_inf: T,
		nan: T,
	) -> T {
		match self.describe() {
			Description::Integer {
				base: Base::Decimal,
				text,
			}
			| Description::Float { text } => T::from_str(text).unwrap(),
			Description::Integer { base, text } => {
				match fsr(text, base.as_radix()) {
					Ok(value) => cast(value),
					Err(err) => match err.kind() {
						// saturating overflows
						IntErrorKind::PosOverflow => inf,
						IntErrorKind::NegOverflow => neg_inf,
						// other errors should never occur
						_ => unreachable!(),
					},
				}
			}
			Description::Special { text: "#inf" } => inf,
			Description::Special { text: "#-inf" } => neg_inf,
			Description::Special { text: "#nan" } => nan,
			Description::Special { .. } => unreachable!(),
		}
	}
	//fn coerce_to_f16(&self) -> f16 { … }
	/// Convert to a float, treating integers as valid.
	pub fn coerce_to_f32(&self) -> f32 {
		#[expect(clippy::cast_precision_loss, reason = "truncating")]
		self.coerce_to_float(
			i32::from_str_radix,
			|v| v as f32,
			f32::INFINITY,
			f32::NEG_INFINITY,
			f32::NAN,
		)
	}
	/// Convert to a float, treating integers as valid.
	pub fn coerce_to_f64(&self) -> f64 {
		#[expect(clippy::cast_precision_loss, reason = "truncating")]
		self.coerce_to_float(
			i64::from_str_radix,
			|v| v as f64,
			f64::INFINITY,
			f64::NEG_INFINITY,
			f64::NAN,
		)
	}
	//fn coerce_to_f128(&self) -> f128 { … }
}
impl fmt::Display for Number {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self.describe() {
			Description::Integer { base, text } => {
				if let Some(text) = text.strip_prefix('-') {
					write!(f, "-{base}{text}")
				} else {
					write!(f, "{base}{text}")
				}
			}
			Description::Float { text } | Description::Special { text } => f.write_str(text),
		}
	}
}

macro_rules! quick_impl {
	(int $ty:ty) => {
		impl From<$ty> for Number {
			fn from(v: $ty) -> Self { Self(format_smolstr!("d{v}")) }
		}
		impl TryFrom<&Number> for $ty {
			type Error = ParseIntError;
			fn try_from(v: &Number) -> Result<Self, Self::Error> {
				match v.describe() {
					Description::Integer { base, text } => {
						Self::from_str_radix(text, base.as_radix())
					}
					_ => Self::from_str("."),
				}
			}
		}
		impl TryFrom<Number> for $ty {
			type Error = ParseIntError;
			fn try_from(v: Number) -> Result<Self, Self::Error> { Self::try_from(&v) }
		}
	};
	(float $ty:ty) => {
		impl From<$ty> for Number {
			fn from(v: $ty) -> Self {
				match v.classify() {
					FpCategory::Nan | FpCategory::Infinite => Self(format_smolstr!("#{v:?}")),
					_ => Self(format_smolstr!("f{v:?}")),
				}
			}
		}
		impl TryFrom<&Number> for $ty {
			type Error = ParseFloatError;
			fn try_from(v: &Number) -> Result<Self, Self::Error> {
				match v.describe() {
					Description::Integer { .. } => Self::from_str("."),
					Description::Float { text } => Self::from_str(text),
					Description::Special { text: "#inf" } => Ok(Self::INFINITY),
					Description::Special { text: "#-inf" } => Ok(Self::NEG_INFINITY),
					Description::Special { text: "#nan" } => Ok(Self::NAN),
					Description::Special { .. } => unreachable!(),
				}
			}
		}
		impl TryFrom<Number> for $ty {
			type Error = ParseFloatError;
			fn try_from(v: Number) -> Result<Self, Self::Error> { Self::try_from(&v) }
		}
	};
}

quick_impl!(int u8);
quick_impl!(int u16);
quick_impl!(int u32);
quick_impl!(int u64);
quick_impl!(int u128);
quick_impl!(int usize);
quick_impl!(int i8);
quick_impl!(int i16);
quick_impl!(int i32);
quick_impl!(int i64);
quick_impl!(int i128);
quick_impl!(int isize);
//quick_impl!(float f16, i16);
quick_impl!(float f32);
quick_impl!(float f64);
//quick_impl!(float f128, i128);

/// Expanded description of a number, this format is not particularly stable.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Description<'text> {
	/// The number is an integer in some base.
	Integer {
		/// The base of the number value.
		base: Base,
		/// Body of the number, /`-?\d+`/.
		text: &'text str,
	},
	/// The number is a fractional decimal number.
	Float {
		/// Body of the number, /`-?\d+(.\d+)?(E[+-]\d+)?`/.
		text: &'text str,
	},
	/// The number is a special float value.
	Special {
		/// `#inf`, `#-inf`, or `#nan`
		text: &'text str,
	},
}

/// Base of a number, implements `Display` as its numeric prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
// TODO/perf: is it better to store these at their values
pub enum Base {
	/// Base 10
	Decimal,
	/// Base 16
	Hexadecimal,
	/// Base 8
	Octal,
	/// Base 2
	Binary,
}
impl Base {
	/// The base as radix
	pub fn as_radix(&self) -> u32 {
		match self {
			Base::Decimal => 10,
			Base::Hexadecimal => 16,
			Base::Octal => 8,
			Base::Binary => 2,
		}
	}
}
impl fmt::Display for Base {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Base::Decimal => "",
			Base::Hexadecimal => "0x",
			Base::Octal => "0o",
			Base::Binary => "0b",
		})
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NbState {
	Start,
	Signed,
	MaybeBased,
	ValueStart,
	Value,
	FractionStart,
	Fraction,
	ExponentStart,
	ExponentSigned,
	Exponent,
}

pub(crate) struct NumberBuilder<T> {
	text: T,
	state: NbState,
	base: Base,
}

impl<T: StringOutput> NumberBuilder<T> {
	pub fn new(mut text: T) -> Self {
		// future location of number tag
		text.so_push_str("?");
		Self {
			text,
			state: NbState::Start,
			base: Base::Decimal,
		}
	}
	fn digit(&self, byte: u8) -> bool {
		match self.base {
			Base::Decimal => byte.is_ascii_digit(),
			Base::Hexadecimal => byte.is_ascii_hexdigit(),
			Base::Octal => matches!(byte, b'0'..=b'7'),
			Base::Binary => matches!(byte, b'0' | b'1'),
		}
	}
	fn set_base(&mut self, base: Base) -> (NbState, bool) {
		self.base = base;
		// don't emit base-specific information for easier from_str_radix
		(NbState::ValueStart, false)
	}
	fn cancel_base(&mut self, state: NbState, push: bool) -> (NbState, bool) {
		// MaybeBased runs when there's no output, so we need an extra zero
		self.text.so_push_str("0");
		(state, push)
	}
	fn decimal(&self) -> bool { self.base == Base::Decimal }
	/// returns `false` on failure
	#[expect(clippy::match_same_arms, reason = "state machine")]
	pub fn step(&mut self, byte: u8) -> bool {
		// TODO/style: wildcard import breaks clippy
		use NbState::{
			Exponent, ExponentSigned, ExponentStart, Fraction, FractionStart, MaybeBased, Signed,
			Start, Value, ValueStart,
		};
		let push;
		(self.state, push) = match (self.state, byte) {
			// +- → Signed, 0 → MaybeBased, 1-9 → Value
			(Start, b'+') => (Signed, false),
			(Start, b'-') => (Signed, true),
			(Start, b'0') => (MaybeBased, false),
			(Start, b'1'..=b'9') => (Value, true),
			(Start, _) => return false,
			// 0 → MaybeBased, 1-9 → Value
			(Signed, b'0') => (MaybeBased, false),
			(Signed, b'1'..=b'9') => (Value, true),
			(Signed, _) => return false,
			// xob → ValueStart(base), 0-9_ → Value, . → FractionStart, e → ExponentStart
			// surprisingly capital bases aren't valid
			(MaybeBased, b'x') => self.set_base(Base::Hexadecimal),
			(MaybeBased, b'o') => self.set_base(Base::Octal),
			(MaybeBased, b'b') => self.set_base(Base::Binary),
			(MaybeBased, b'0'..=b'9') => (Value, true),
			(MaybeBased, b'_') => self.cancel_base(Value, false),
			(MaybeBased, b'.') => self.cancel_base(FractionStart, true),
			(MaybeBased, b'e' | b'E') => self.cancel_base(ExponentStart, true),
			(MaybeBased, _) => return false,
			// \d → Value
			(ValueStart, c) if self.digit(c) => (Value, true),
			(ValueStart, _) => return false,
			// \d_ → Value, . → FractionStart, e → ExponentStart
			(Value, c) if self.digit(c) => (Value, true),
			(Value, b'_') => (Value, false),
			(Value, b'.') if self.decimal() => (FractionStart, true),
			(Value, b'e' | b'E') if self.decimal() => (ExponentStart, true),
			(Value, _) => return false,
			// \d → Fraction
			(FractionStart, c) if self.digit(c) => (Fraction, true),
			(FractionStart, _) => return false,
			// \d_ → Fraction, e → ExponentStart
			(Fraction, c) if self.digit(c) => (Fraction, true),
			(Fraction, b'_') => (Fraction, false),
			(Fraction, b'e' | b'E') => (ExponentStart, true),
			(Fraction, _) => return false,
			// +- → ExponentSigned, 0-9 → Exponent
			(ExponentStart, b'+' | b'-') => (ExponentSigned, true),
			(ExponentStart, b'0'..=b'9') => {
				self.text.so_push_str("+");
				(Exponent, true)
			}
			(ExponentStart, _) => return false,
			// 0-9 → Exponent
			(ExponentSigned, b'0'..=b'9') => (Exponent, true),
			(ExponentSigned, _) => return false,
			// 0-9_ → Exponent
			(Exponent, b'0'..=b'9') => (Exponent, true),
			(Exponent, b'_') => (Exponent, false),
			(Exponent, _) => return false,
		};
		if push {
			let byte = if byte == b'e' { b'E' } else { byte };
			// technically not a correct cast, but this is only for ascii characters
			self.text.so_push_char(byte as char);
		}
		true
	}
	/// None = error, Some(None) = skipped
	#[expect(clippy::option_option, reason = "internal")]
	pub fn finish(mut self) -> Option<Option<Number>> {
		// valid place to end?
		let float = match self.state {
			NbState::Start
			| NbState::Signed
			| NbState::ValueStart
			| NbState::FractionStart
			| NbState::ExponentStart
			| NbState::ExponentSigned => return None,
			NbState::MaybeBased => {
				self.text.so_push_str("0");
				false
			}
			NbState::Value => false,
			NbState::Fraction | NbState::Exponent => true,
		};
		let first = match (self.base, float) {
			(Base::Decimal, false) => b'd',
			(Base::Hexadecimal, false) => b'x',
			(Base::Octal, false) => b'o',
			(Base::Binary, false) => b'b',
			(Base::Decimal, true) => b'f',
			(_, true) => unreachable!(),
		};
		Some(self.text.so_finish_num(first).map(Number))
	}
}

impl FromStr for Number {
	type Err = ();
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"#inf" => Ok(Self::INFINITY),
			"#-inf" => Ok(Self::NEG_INFINITY),
			"#nan" => Ok(Self::NAN),
			_ => {
				let mut out = NumberBuilder::new(SmolStrBuilder2::new());
				for &byte in s.as_bytes() {
					if byte >= 0x80 || !out.step(byte) {
						return Err(());
					}
				}
				out.finish()
					.ok_or(())
					.map(|v| v.unwrap_or_else(|| unreachable!()))
			}
		}
	}
}
