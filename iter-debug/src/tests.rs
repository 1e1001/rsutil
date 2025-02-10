extern crate core;

use core::fmt::{Result, Write};
use core::option::Option;
use core::{panic, write};

use crate::{DebugIterator, IterDebug};

// since i don't feel like using alloc, enjoy this cursed fmt writer
struct MatchStr<'str>(&'str str);
impl Write for MatchStr<'_> {
	fn write_str(&mut self, text: &str) -> Result {
		let Option::Some(new) = self.0.strip_prefix(text) else {
			panic!("Textual mismatch!");
		};
		self.0 = new;
		Result::Ok(())
	}
}
macro_rules! check {
	($t:literal == $e:expr) => {
		check!($t, "{:?}" == $e)
	};
	($t:literal, $f:literal $(== $($i:tt)*)?)	=> {{
		let mut matcher = MatchStr($t);
		write!(matcher, $f, $($($i)*)?).unwrap();
		assert!(matcher.0.is_empty(), "not all text matched");
	}};
}

#[test]
fn basic() {
	let iterator = [1, 2, 3];
	check!("[1, 2, 3]" == IterDebug::new(iterator));
	check!("[1, 2, 3]" == iterator.debug());
}

#[test]
fn empty() {
	check!("[]" == [0_u8; 0].debug());
}

#[test]
fn options() {
	let iterator = [1, 10, 100];
	check!("[01, 0a, 64]", "{:>02x?}" == iterator.debug());
	check!(
		"[\n    1,\n    10,\n    100,\n]",
		"{:#?}" == iterator.debug()
	);
}

#[test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: Error"]
fn invalid() {
	let iterator = [1, 2, 3].debug();
	check!("[1, 2, 3]" == iterator);
	check!("[1, 2, 3]" == iterator);
}
