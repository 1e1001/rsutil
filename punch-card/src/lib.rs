#![feature(const_type_name)]
#![feature(const_trait_impl)]
// needed for a test
#![recursion_limit = "256"]

use std::ops::{RangeFull, RangeTo, RangeToInclusive};

/// a single entry of a punch card
pub enum PunchCardItem {
	False, True, Error
}

impl PunchCardItem {
	const fn to_bool(self) -> bool {
		match self {
			Self::False => false,
			Self::True => true,
			Self::Error => panic!("mismatched tape"),
		}
	}
}

/// A single line (or tail of a line) in a punched card. Lines are typed like a linked list, for example the type of the tape in the `valid_u1` test is
/// ```no
/// ..= RangeToInclusive<
/// ..   RangeTo<
/// ..=   RangeToInclusive<
/// ..=    RangeToInclusive<
/// ..      RangeTo<
/// ..       RangeTo<
/// ..=       RangeToInclusive<
/// ..=        RangeToInclusive<
/// ..=         RangeToInclusive<
/// ..           RangeFull>>>>>>>>>
/// ```
pub trait PunchCardLine {
	const HEAD: PunchCardItem;
	type Tail: PunchCardLine;
	/// The amount of remaining items in this line
	const LENGTH: usize;
}

/// A container to output data to, there are default implementations for `Vec<T>`, `[T; N]`
pub trait PunchCardOutput<T> {
	fn new(size: usize) -> Self;
	fn set(&mut self, index: usize, item: T);
}

impl<T> PunchCardOutput<T> for Vec<T> {
	fn new(size: usize) -> Self {
		Vec::with_capacity(size)
	}
	fn set(&mut self, _index: usize, item: T) {
		self.push(item)
	}
}
impl<T: Default + Copy, const N: usize> PunchCardOutput<T> for [T; N] {
	fn new(size: usize) -> Self {
		assert_eq!(size, N);
		[T::default(); N]
	}
	fn set(&mut self, index: usize, item: T) {
		self[index] = item;
	}
}

/// A collection of vertically-stacked `PunchCardLine`'s.
pub trait PunchCard {
	/// The tail of every line, as a card.
	type Tail: PunchCard;
	/// The output type, usually a Vec<_>.
	type Output;
	/// Internal evaluation function, evaluates this card and appends the value onto an output.
	///
	/// Don't call this directly, use `punch_card` instead
	fn eval<T: PunchCardOutput<Self::Output>>(v: &mut T, i: usize);
	/// Returns the value of a punched card.
	fn punch_card<T: PunchCardOutput<Self::Output>>(&self) -> T;
}

// TODO: library doesn't work until this macro uses the new output-passing style
macro_rules! pcard {
	(($($in_type:ident),* $(,)?), $first:ident, $eval:expr, $out_type:ty) => {
		// #[allow(unused_parens)]
		impl<$($in_type: PunchCardLine),*> PunchCard for ($($in_type),+,) {
			type Tail = ($($in_type::Tail),*,);
			type Output = $out_type;
			#[inline(always)]
			fn eval<S: PunchCardOutput<Self::Output>>(v: &mut S, i: usize) {
				// we use || here to keep going and catch any length mismatches
				if $($in_type::LENGTH > 0)||* {
					v.set(i, $eval);
					Self::Tail::eval(v, i + 1);
				}
			}
			#[inline(always)]
			fn punch_card<S: PunchCardOutput<Self::Output>>(&self) -> S {
				let mut out = <S>::new($first::LENGTH);
				Self::eval(&mut out, 0);
				out
			}
		}
	};
}
macro_rules! pcard_int {
	($res:ty, ($first:ident, $($typ:ident => $offset:expr),* $(,)?)) => {
		pcard!(($($typ),*), $first, $((if <$typ>::HEAD.to_bool() {1 << $offset} else {0}))|*, $res);
	};
}
// TODO: consider using a BitVec for this
pcard! {(T), T, T::HEAD.to_bool(), bool}
pcard_int! {u8, (T0,
	T0 => 7, T1 => 6, T2 => 5, T3 => 4, T4 => 3, T5 => 2, T6 => 1, T7 => 0,
)}
pcard_int! {u16, (T0,
	T0 => 0xf, T1 => 0xe, T2 => 0xd, T3 => 0xc,
	T4 => 0xb, T5 => 0xa, T6 => 0x9, T7 => 0x8,
	T8 => 0x7, T9 => 0x6, Ta => 0x5, Tb => 0x4,
	Tc => 0x3, Td => 0x2, Te => 0x1, Tf => 0x0,
)}
pcard_int! {u32, (T00,
	T00 => 0x1f, T01 => 0x1e, T02 => 0x1d, T03 => 0x1c,
	T04 => 0x1b, T05 => 0x1a, T06 => 0x19, T07 => 0x18,
	T08 => 0x17, T09 => 0x16, T0a => 0x15, T0b => 0x14,
	T0c => 0x13, T0d => 0x12, T0e => 0x11, T0f => 0x10,
	T10 => 0x0f, T11 => 0x0e, T12 => 0x0d, T13 => 0x0c,
	T14 => 0x0b, T15 => 0x0a, T16 => 0x09, T17 => 0x08,
	T18 => 0x07, T19 => 0x06, T1a => 0x05, T1b => 0x04,
	T1c => 0x03, T1d => 0x02, T1e => 0x01, T1f => 0x00,
)}
pcard_int! {u64, (T00,
	T00 => 0x3f, T01 => 0x3e, T02 => 0x3d, T03 => 0x3c,
	T04 => 0x3b, T05 => 0x3a, T06 => 0x39, T07 => 0x38,
	T08 => 0x37, T09 => 0x36, T0a => 0x35, T0b => 0x34,
	T0c => 0x33, T0d => 0x32, T0e => 0x31, T0f => 0x30,
	T10 => 0x2f, T11 => 0x2e, T12 => 0x2d, T13 => 0x2c,
	T14 => 0x2b, T15 => 0x2a, T16 => 0x29, T17 => 0x28,
	T18 => 0x27, T19 => 0x26, T1a => 0x25, T1b => 0x24,
	T1c => 0x23, T1d => 0x22, T1e => 0x21, T1f => 0x20,
	T20 => 0x1f, T21 => 0x1e, T22 => 0x1d, T23 => 0x1c,
	T24 => 0x1b, T25 => 0x1a, T26 => 0x19, T27 => 0x18,
	T28 => 0x17, T29 => 0x16, T2a => 0x15, T2b => 0x14,
	T2c => 0x13, T2d => 0x12, T2e => 0x11, T2f => 0x10,
	T30 => 0x0f, T31 => 0x0e, T32 => 0x0d, T33 => 0x0c,
	T34 => 0x0b, T35 => 0x0a, T36 => 0x09, T37 => 0x08,
	T38 => 0x07, T39 => 0x06, T3a => 0x05, T3b => 0x04,
	T3c => 0x03, T3d => 0x02, T3e => 0x01, T3f => 0x00,
)}
pcard_int! {u128, (T00,
	T00 => 0x7f, T01 => 0x7e, T02 => 0x7d, T03 => 0x7c,
	T04 => 0x7b, T05 => 0x7a, T06 => 0x79, T07 => 0x78,
	T08 => 0x77, T09 => 0x76, T0a => 0x75, T0b => 0x74,
	T0c => 0x73, T0d => 0x72, T0e => 0x71, T0f => 0x70,
	T10 => 0x6f, T11 => 0x6e, T12 => 0x6d, T13 => 0x6c,
	T14 => 0x6b, T15 => 0x6a, T16 => 0x69, T17 => 0x68,
	T18 => 0x67, T19 => 0x66, T1a => 0x65, T1b => 0x64,
	T1c => 0x63, T1d => 0x62, T1e => 0x61, T1f => 0x60,
	T20 => 0x5f, T21 => 0x5e, T22 => 0x5d, T23 => 0x5c,
	T24 => 0x5b, T25 => 0x5a, T26 => 0x59, T27 => 0x58,
	T28 => 0x57, T29 => 0x56, T2a => 0x55, T2b => 0x54,
	T2c => 0x53, T2d => 0x52, T2e => 0x51, T2f => 0x50,
	T30 => 0x4f, T31 => 0x4e, T32 => 0x4d, T33 => 0x4c,
	T34 => 0x4b, T35 => 0x4a, T36 => 0x49, T37 => 0x48,
	T38 => 0x47, T39 => 0x46, T3a => 0x45, T3b => 0x44,
	T3c => 0x43, T3d => 0x42, T3e => 0x41, T3f => 0x40,
	T40 => 0x3f, T41 => 0x3e, T42 => 0x3d, T43 => 0x3c,
	T44 => 0x3b, T45 => 0x3a, T46 => 0x39, T47 => 0x38,
	T48 => 0x37, T49 => 0x36, T4a => 0x35, T4b => 0x34,
	T4c => 0x33, T4d => 0x32, T4e => 0x31, T4f => 0x30,
	T50 => 0x2f, T51 => 0x2e, T52 => 0x2d, T53 => 0x2c,
	T54 => 0x2b, T55 => 0x2a, T56 => 0x29, T57 => 0x28,
	T58 => 0x27, T59 => 0x26, T5a => 0x25, T5b => 0x24,
	T5c => 0x23, T5d => 0x22, T5e => 0x21, T5f => 0x20,
	T60 => 0x1f, T61 => 0x1e, T62 => 0x1d, T63 => 0x1c,
	T64 => 0x1b, T65 => 0x1a, T66 => 0x19, T67 => 0x18,
	T68 => 0x17, T69 => 0x16, T6a => 0x15, T6b => 0x14,
	T6c => 0x13, T6d => 0x12, T6e => 0x11, T6f => 0x10,
	T70 => 0x0f, T71 => 0x0e, T72 => 0x0d, T73 => 0x0c,
	T74 => 0x0b, T75 => 0x0a, T76 => 0x09, T77 => 0x08,
	T78 => 0x07, T79 => 0x06, T7a => 0x05, T7b => 0x04,
	T7c => 0x03, T7d => 0x02, T7e => 0x01, T7f => 0x00,
)}

impl PunchCardLine for RangeFull {
	const HEAD: PunchCardItem = PunchCardItem::Error;
	type Tail = Self;
	const LENGTH: usize = 0;
}
impl<T: PunchCardLine> PunchCardLine for RangeTo<T> {
	const HEAD: PunchCardItem = PunchCardItem::False;
	type Tail = T;
	const LENGTH: usize = Self::Tail::LENGTH + 1;
}
impl<T: PunchCardLine> PunchCardLine for RangeToInclusive<T> {
	const HEAD: PunchCardItem = PunchCardItem::True;
	type Tail = T;
	const LENGTH: usize = Self::Tail::LENGTH + 1;
}

#[cfg(test)]
mod tests;
