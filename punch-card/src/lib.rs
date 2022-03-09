#![feature(const_type_name)]
#![feature(const_fn_trait_bound)]
#![feature(const_trait_impl)]

use std::marker::PhantomData;
use std::ops::{RangeFull, RangeTo, RangeToInclusive};

trait PunchCardLine {
	type Inner: PunchCardLine;
	const VALUE: u8;
	const HAS_VALUE: bool;
}

trait PunchCard {
	type Inner: PunchCard;
	const VALUE: u8;
	fn sub_eval(v: &mut Vec<u8>);
	fn punch_card(&self) -> Vec<u8>;
}

impl<T0, T1, T2, T3, T4, T5, T6, T7> PunchCard for (T0, T1, T2, T3, T4, T5, T6, T7)
where
	T0: PunchCardLine,
	T1: PunchCardLine,
	T2: PunchCardLine,
	T3: PunchCardLine,
	T4: PunchCardLine,
	T5: PunchCardLine,
	T6: PunchCardLine,
	T7: PunchCardLine,
{
	type Inner = (
		T0::Inner,
		T1::Inner,
		T2::Inner,
		T3::Inner,
		T4::Inner,
		T5::Inner,
		T6::Inner,
		T7::Inner,
	);
	const VALUE: u8 = (T0::VALUE << 7)
		| (T1::VALUE << 6)
		| (T2::VALUE << 5)
		| (T3::VALUE << 4)
		| (T4::VALUE << 3)
		| (T5::VALUE << 2)
		| (T6::VALUE << 1)
		| (T7::VALUE);
	fn sub_eval(v: &mut Vec<u8>) {
		if T0::HAS_VALUE {
			v.push(Self::VALUE);
			Self::Inner::sub_eval(v);
		}
	}
	fn punch_card(&self) -> Vec<u8> {
		let mut out = Vec::new();
		Self::sub_eval(&mut out);
		out
	}
}

impl PunchCardLine for RangeFull {
	type Inner = Self;
	const HAS_VALUE: bool = false;
	const VALUE: u8 = 0;
}
impl<T: PunchCardLine> PunchCardLine for RangeTo<T> {
	type Inner = T;
	const HAS_VALUE: bool = true;
	const VALUE: u8 = 0;
}
impl<T: PunchCardLine> PunchCardLine for RangeToInclusive<T> {
	type Inner = T;
	const HAS_VALUE: bool = true;
	const VALUE: u8 = 1;
}

#[cfg(test)]
mod tests {
	use crate::PunchCard;

	#[test]
	fn test() {
		println!("{:?}", (
			.. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..,
			.. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..,
			.. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..,
			.. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..,
			.. .. .. .. .. .. .. .. ..=..=..=..=..=..=..=..=.. .. .. .. .. .. .. .. ..=..=..=..=..=..=..=..=..,
			.. .. .. .. ..=..=..=..=.. .. .. .. ..=..=..=..=.. .. .. .. ..=..=..=..=.. .. .. .. ..=..=..=..=..,
			.. .. ..=..=.. .. ..=..=.. .. ..=..=.. .. ..=..=.. .. ..=..=.. .. ..=..=.. .. ..=..=.. .. ..=..=..,
			.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=.. ..=..,
		).punch_card());
	}
}
