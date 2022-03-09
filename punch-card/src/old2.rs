#![feature(generic_associated_types)]
#![feature(generic_const_exprs)]
use std::ops::{RangeFull, RangeTo, RangeToInclusive};

const fn next(n: usize) -> usize {
	n - 1
}

trait Strip<const N: usize> {
	type INNER: Strip<{next(N)}> where [(); next(N)]:;
	const VALUE: bool;
}

impl<const N: usize> Strip<N> for () {
	type INNER where [(); next(N)]: = ();
	const VALUE: bool = false;
}
impl Strip<1> for RangeFull {
	type INNER = ();
	const VALUE: bool = false;
}
impl<const N: usize, T: Strip<{next(N)}>> Strip<N> for RangeTo<T> {
	type INNER = T;
	const VALUE: bool = false;
}
impl<const N: usize, T: Strip<{next(N)}>> Strip<N> for RangeToInclusive<T> {
	type INNER = T;
	const VALUE: bool = true;
}

const fn eval_strip_type<const N: usize, T: Strip<N>>() where [(); next(N)]: {
	if N == 0 {
		println!(".");
	} else if <T>::VALUE {
		print!("1");
		eval_strip_type::<{next(N)}, <T>::INNER>()
	} else {
		print!("0");
		eval_strip_type::<{next(N)}, <T>::INNER>()
	}
}

const fn eval_strip<const N: usize, T: Strip<N>>(_: &T) where [(); next(N)]: {
	eval_strip_type::<N, T>();
}

#[cfg(test)]
mod tests {
    use old2::eval_strip;
	#[test]
	fn test() {
		eval_strip(&(..=..=.. ..=.. .. ..=.. ..));
	}
}
