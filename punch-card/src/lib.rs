#![feature(const_fn_trait_bound)]
use std::ops::{RangeFull, RangeTo, RangeToInclusive};

enum StripValue {
	Zero, One, End
}
trait Strip {
	type Inner: Strip;
	const VALUE: StripValue;
}
impl Strip for RangeFull {
	type Inner = RangeFull;
	const VALUE: StripValue = StripValue::End;
}
impl<T: Strip> Strip for RangeTo<T> {
	type Inner = T;
	const VALUE: StripValue = StripValue::Zero;
}
impl<T: Strip> Strip for RangeToInclusive<T> {
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
