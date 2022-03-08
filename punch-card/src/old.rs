use std::ops::{RangeFull, RangeTo, RangeToInclusive};

#[cfg(test)]
mod tests {
	// #[test]
}

/// A tail of a single strip from a PunchTape. Strips are stored like linked lists, like so
/// ```no
/// ..=..=.. .. ..=.. ..=.. ..
/// │  │  │  │  │  │  │  │  └┤ ← a Strip
/// │  │  │  │  │  │  │  └───┤ ← a Strip
/// │  │  │  │  │  │  └──────┤ ← a Strip
/// │  │  │  │  │  └─────────┤ ← a Strip
/// │  │  │  │  └────────────┤ ← a Strip
/// │  │  │  └───────────────┤ ← a Strip
/// │  │  └──────────────────┤ ← a Strip
/// │  └─────────────────────┤ ← a Strip
/// └────────────────────────┘ ← a Strip
/// ```
///
// TODO: newdoc
pub trait StripPair {
	/// Evaluate the car and cdr and push the results onto a BitVec
	// TODO: newdoc
	fn eval(&self, out: &mut Vec<u8>, index: usize, val: u8);
	/// Get the length of the strip in bits, with a sum parameter to make it
	/// tail-recursive.
	// TODO: newdoc
	fn len(&self, sum: usize) -> usize;
}
/// A RangeFull marks the end of a strip `···..`
// TODO: newdoc
impl StripPair for RangeFull {
	fn eval(&self, _: &mut Vec<u8>, _: usize, _: u8) {}
	fn len(&self, sum: usize) -> usize {
		sum
	}
}
/// A RangeTo is a `···.. ···`, which evaluates to a 0.
// TODO: newdoc
impl<T: StripPair> StripPair for RangeTo<T> {
	fn eval(&self, out: &mut Vec<u8>, index: usize, val: u8) {
		out[index] |= val;
		self.end.eval(out, index + 1, val);
	}
	fn len(&self, sum: usize) -> usize {
		self.end.len(sum + 1)
	}
}
/// A RangeToInclusive is a `···..=···`, which evaluates to a 1.
// TODO: newdoc
impl<T: StripPair> StripPair for RangeToInclusive<T> {
	fn eval(&self, out: &mut Vec<u8>, index: usize, val: u8) {
		out[index] |= val;
		self.end.eval(out, index + 1, val);
	}
	fn len(&self, sum: usize) -> usize {
		self.end.len(sum + 1)
	}
}
// TODO: newdoc
pub fn punch_card<S0, S1, S2, S3, S4, S5, S6, S7>(
	v0: S0, v1: S1, v2: S2, v3: S3,
	v4: S4, v5: S5, v6: S6, v7: S7,
) -> Vec<u8>
where
	S0: StripPair, S1: StripPair, S2: StripPair, S3: StripPair,
	S4: StripPair, S5: StripPair, S6: StripPair, S7: StripPair,
{
	let len = v0.len(0);
	assert_eq!(len, v1.len(0));
	assert_eq!(len, v2.len(0));
	assert_eq!(len, v3.len(0));
	assert_eq!(len, v4.len(0));
	assert_eq!(len, v5.len(0));
	assert_eq!(len, v7.len(0));
	assert_eq!(len, v6.len(0));
	let mut vec = vec![0; len];
	v0.eval(&mut vec, 0, 0x80);
	v1.eval(&mut vec, 0, 0x40);
	v2.eval(&mut vec, 0, 0x20);
	v3.eval(&mut vec, 0, 0x10);
	v4.eval(&mut vec, 0, 0x08);
	v5.eval(&mut vec, 0, 0x04);
	v6.eval(&mut vec, 0, 0x02);
	v7.eval(&mut vec, 0, 0x01);
	vec
}
