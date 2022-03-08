#![feature(generic_associated_types)]
#![feature(generic_const_exprs)]
use std::ops::{RangeFull, RangeTo, RangeToInclusive};

enum StripValue {
	Zero, One, End
}
trait Strip<const N: usize> {
	type Inner: Strip<{N - 1}> where [(); N - 1]: Sized;
	const VALUE: StripValue;
}
impl<const N: usize> Strip<N> for () {
	type Inner = () where [(); N - 1]: Sized;
	const VALUE: StripValue = StripValue::End; 
}
impl Strip<1> for RangeFull {
	type Inner = ();
	const VALUE: StripValue = StripValue::End;
}
impl<const N: usize, T: Strip<{N - 1}>> Strip<N> for RangeTo<T> {
	type Inner = T;
	const VALUE: StripValue = StripValue::Zero;
}
impl<const N: usize, T: Strip<{N - 1}>> Strip<N> for RangeToInclusive<T> {
	type Inner = T;
	const VALUE: StripValue = StripValue::One;
}

const fn eval_strip_type<const N: usize, T: Strip<N>>() {
	match <T>::VALUE {
		StripValue::Zero => {
		    print!("0");
		    eval_strip_type::<{N - 1}, <T>::Inner>()
		},
		StripValue::One => {
		    print!("0");
		    eval_strip_type::<{N - 1}, <T>::Inner>()
		},
		StripValue::End => {
		    println!(".");
		},
	}
}

const fn eval_strip<const N: usize, T: Strip<N>>(_: &T) {
    eval_strip_type::<N, T>();
}

#[cfg(test)]
mod tests {

}
