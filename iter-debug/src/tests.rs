// SPDX-License-Identifier: MIT OR Apache-2.0
extern crate core;

use core::fmt::{Result, Write};
use core::option::Option;
use core::write;

use crate::{DebugIterator, IterDebug, KvIterDebug};

// since i don't feel like using alloc, enjoy this cursed fmt writer
struct MatchStr<'str>(&'str str);
impl Write for MatchStr<'_> {
	#[expect(clippy::panic_in_result_fn, reason = "different error messages")]
	fn write_str(&mut self, text: &str) -> Result {
		let Option::Some(new) = self.0.strip_prefix(text) else {
			panic!("mismatched text {:?} != {text:?}", self.0);
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
	let array = [(1, "a"), (2, "b"), (3, "c")];
	check!("[(1, \"a\"), (2, \"b\"), (3, \"c\")]" == IterDebug::new(array));
	check!("[(1, \"a\"), (2, \"b\"), (3, \"c\")]" == array.debug());
	check!("{(1, \"a\"), (2, \"b\"), (3, \"c\")}" == IterDebug::new_set(array));
	check!("{(1, \"a\"), (2, \"b\"), (3, \"c\")}" == array.debug_set());
	check!("{1: \"a\", 2: \"b\", 3: \"c\"}" == KvIterDebug::new(array));
	check!("{1: \"a\", 2: \"b\", 3: \"c\"}" == array.debug_map());
}

#[test]
fn empty() {
	check!("[]" == [0_u8; 0].debug());
	check!("{}" == [0_u8; 0].debug_set());
	check!("{}" == [(0_u8, 0_u8); 0].debug_map());
}

#[test]
fn options() {
	let array = [1, 10, 100];
	check!("[01, 0a, 64]", "{:>02x?}" == array.debug());
	check!("[\n    1,\n    10,\n    100,\n]", "{:#?}" == array.debug());
	check!("{01, 0a, 64}", "{:>02x?}" == array.debug_set());
	check!(
		"{\n    1,\n    10,\n    100,\n}",
		"{:#?}" == array.debug_set()
	);
	let array2 = [(1, 2), (10, 20), (100, 200)];
	check!("{01: 02, 0a: 14, 64: c8}", "{:>02x?}" == array2.debug_map());
	check!(
		"{\n    1: 2,\n    10: 20,\n    100: 200,\n}",
		"{:#?}" == array2.debug_map()
	);
}

#[test]
fn invalid() {
	let iterator = [1, 2, 3].debug();
	check!("[1, 2, 3]" == iterator);
	check!("<consumed iterator>" == iterator);
	let iterator2 = [1, 2, 3].debug_set();
	check!("{1, 2, 3}" == iterator2);
	check!("<consumed iterator>" == iterator2);
	let iterator3 = [(1, 1), (2, 2), (3, 3)].debug_map();
	check!("{1: 1, 2: 2, 3: 3}" == iterator3);
	check!("<consumed iterator>" == iterator3);
}
