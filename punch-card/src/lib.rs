#![feature(const_fn_trait_bound)]
use std::ops::{RangeFull, RangeTo, RangeToInclusive};

enum StripValue {
	Zero, One, End
}
trait Strip<const N: usize> {
	type Inner: Strip<N - 1>;
	const VALUE: StripValue;
}
impl Strip<0> for RangeFull {
	type Inner = RangeFull;
	const VALUE: StripValue = StripValue::End;
}
impl<const N: usize, T: Strip<N - 1>> Strip<N> for RangeTo<T> {
	type Inner = T;
	const VALUE: StripValue = StripValue::Zero;
}
impl<const N: usize, T: Strip<N - 1>> Strip<N> for RangeToInclusive<T> {
	type Inner = T;
	const VALUE: StripValue = StripValue::One;
}

const fn eval_strip<T: Strip>(_strip: &T) {
	match <T>::VALUE {
		StripValue::Zero => {},
		StripValue::One => {},
		StripValue::End => {},
	}
}

#[cfg(test)]
mod tests {

}
