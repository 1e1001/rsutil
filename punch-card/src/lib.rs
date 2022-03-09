#![feature(const_type_name)]
#![feature(const_fn_trait_bound)]
#![feature(const_trait_impl)]

use std::marker::PhantomData;
use std::ops::{RangeFull, RangeTo, RangeToInclusive};

/// A single line (or tail of a line) in a punched card. Lines are typed like a linked list, for example the type of the tape in the `valid_u1` test is
/// ```
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
trait PunchCardLine {
	/// The tail of the line
	type Tail: PunchCardLine;
	/// The head of the line
	const VALUE: bool;
	/// The amount of remaining items in this line
	const LENGTH: usize;
}

/// A collection of vertically-stacked `PunchCardLine`'s.
pub trait PunchCard {
	/// The tail of every line, as a card.
	type Tail: PunchCard;
	/// The output type, usually a Vec<_>.
	type Output;
	/// Internal evaluation function, evaluates this line and appends the value onto an output.
	///
	/// Don't call this directly, use `punch_card` instead
	fn eval(v: &mut Self::Output);
	/// Returns the value of a punched card.
	fn punch_card(&self) -> Self::Output;
}

macro_rules! pcard {
	(($($in_type:ty => $shift:expr),* $(,)?), $t0:ty $out_type:ty, $out_push:expr, $new_out:expr) => (
		impl<$($in_type),*> PunchCard for ($($in_type),*)
		where $($in_type: PunchCardLine),* {
			type Tail = ($($in_type::Tail),*);
			type Output = $out_type;
			#[inline(always)]
			fn eval(v: &mut $out_type) {
				if $t0::LENGTH > 0 {
					$out_push(v, $($shift)|*);
					Self::Tail::eval(v);
				}
			}
			#[inline(always)]
			fn punch_card(&self) -> $out_type {
				let mut out = $new_out;
				Self::eval(&mut out);
				out
			}
		}
	);
}
macro_rules! pcard_int {
	($($typ:ty => $offset:expr),* $(,)?) => (
		($($typ => $typ::VALUE << $offset))
	);
}
/// TODO: consider using a BitVec for this
pcard!{(T => T::VALUE), T, Vec<bool>, Vec::new()}
pcard!{pcard_int(
	T0 => 7, T1 => 6, T2 => 5, T3 => 4, T4 => 3, T5 => 2, T6 => 1, T7 => 0,
), T0, Vec<u8>, Vec::push, Vec::new()}
pcard!{pcard_int(
	T0 => 15, T1 => 14, T2 => 13, T3 => 12, T4 => 11, T5 => 10, T6 =>  9, T7 =>  8, 
	T8 =>  7, T9 =>  6, T10 => 5, T11 => 4, T12 => 3, T13 => 2, T14 => 1, T15 => 0,
), T0, Vec<u16>, Vec::push, Vec::new()}
pcard!{pcard_int(
	T0  => 31, T1  => 30, T2  => 29, T3  => 28, T4  => 27, T5  => 26, T6  => 25, T7  => 24,
	T8  => 23, T9  => 22, T10 => 21, T11 => 20, T12 => 19, T13 => 18, T14 => 17, T15 => 16,
	T16 => 15, T17 => 14, T18 => 13, T19 => 12, T20 => 11, T21 => 10, T22 =>  9, T23 =>  8,
	T24 =>  7, T25 =>  6, T26 =>  5, T27 =>  4, T28 =>  3, T29 =>  2, T30 =>  1, T31 =>  0,
), T0, Vec<u32>, Vec::push, Vec::new()}
pcard!{pcard_int(
	T0  => 63, T1  => 62, T2  => 61, T3  => 60, T4  => 59, T5  => 58, T6  => 57, T7  => 56,
	T8  => 55, T9  => 54, T10 => 53, T11 => 52, T12 => 51, T13 => 50, T14 => 49, T15 => 48,
	T16 => 47, T17 => 46, T18 => 45, T19 => 44, T20 => 43, T21 => 42, T22 => 41, T23 => 40,
	T24 => 39, T25 => 38, T26 => 37, T27 => 36, T28 => 35, T29 => 34, T30 => 33, T31 => 32,
	T32 => 31, T33 => 30, T34 => 29, T35 => 28, T36 => 27, T37 => 26, T38 => 25, T39 => 24,
	T40 => 23, T41 => 22, T42 => 21, T43 => 20, T44 => 19, T45 => 18, T46 => 17, T47 => 16,
	T48 => 15, T49 => 14, T50 => 13, T51 => 12, T52 => 11, T53 => 10, T54 =>  9, T55 =>  8,
	T56 =>  7, T57 =>  6, T58 =>  5, T59 =>  4, T60 =>  3, T61 =>  2, T62 =>  1, T63 =>  0,
), T0, Vec<u64>, Vec::push, Vec::new()}
pcard!{pcard_int(
	T0  => 63, T1  => 62, T2  => 61, T3  => 60, T4  => 59, T5  => 58, T6  => 57, T7  => 56,
	T8  => 55, T9  => 54, T10 => 53, T11 => 52, T12 => 51, T13 => 50, T14 => 49, T15 => 48,
	T16 => 47, T17 => 46, T18 => 45, T19 => 44, T20 => 43, T21 => 42, T22 => 41, T23 => 40,
	T24 => 39, T25 => 38, T26 => 37, T27 => 36, T28 => 35, T29 => 34, T30 => 33, T31 => 32,
	T32 => 31, T33 => 30, T34 => 29, T35 => 28, T36 => 27, T37 => 26, T38 => 25, T39 => 24,
	T40 => 23, T41 => 22, T42 => 21, T43 => 20, T44 => 19, T45 => 18, T46 => 17, T47 => 16,
	T48 => 15, T49 => 14, T50 => 13, T51 => 12, T52 => 11, T53 => 10, T54 =>  9, T55 =>  8,
	T56 =>  7, T57 =>  6, T58 =>  5, T59 =>  4, T60 =>  3, T61 =>  2, T62 =>  1, T63 =>  0,
), T0, Vec<u128>, Vec::push, Vec::new()}

impl PunchCardLine for RangeFull {
	type Tail = Self;
	const LENGTH: usize = 0;
	const VALUE: u8 = panic!("length mismatch");
}
impl<T: PunchCardLine> PunchCardLine for RangeTo<T> {
	type Tail = T;
	const LENGTH: usize = Tail::LENGTH + 1;
	const VALUE: u8 = false;
}
impl<T: PunchCardLine> PunchCardLine for RangeToInclusive<T> {
	type Tail = T;
	const LENGTH: usize = Tail::LENGTH + 1;
	const VALUE: u8 = true;
}

#[cfg(test)]
mod tests;
