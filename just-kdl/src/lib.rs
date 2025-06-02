// SPDX-License-Identifier: MIT OR Apache-2.0
//! Small streaming [KDL] v2.0.0 parser
//!
//! Designed for reasonable performance and memory efficiency, at the expense
//! (or benefit, depending on use) of not storing formatting information
//!
//! ## Why?
//!
//! The [official Rust implementation][kdl-rs] is designed to support editing of
//! kdl files. While this is normally useful, my main use of KDL is to just
//! parse the values into some internal data structure (configuration, document
//! trees, etc.) where formatting information is entirely redundant and just
//! wasteful of parsing time and memory.
//!
//! Additionally, this implementation has a few other benefits:
//! - Full v2.0.0 compliance
//! - Significantly fewer dependencies!
//!
//! ## Benchmarks
//!
//! On my personal laptop, on low power setting:
//! ```text
//! // in release mode
//! bench "html-standard-compact.kdl" {
//!   JustKdlDom {
//!     time "688.9548ms"
//!     memory new=217_494_981 free=28_488_457 net=189_006_524
//!   }
//!   Kdl {
//!     time "13.228400752s"
//!     memory new=5_219_542_533 free=4_305_774_657 net=913_767_876
//!   }
//! }
//! bench "html-standard.kdl" {
//!   JustKdlDom {
//!     time "628.389088ms"
//!     memory new=357_151_222 free=35_738_264 net=321_412_958
//!   }
//!   Kdl {
//!     time "18.194044124s"
//!     memory new=8_146_781_320 free=6_584_885_611 net=1_561_895_709
//!   }
//! }
//!
//! // in debug mode
//! bench "html-standard-compact.kdl" {
//!   JustKdlDom {
//!     time "4.169828469s"
//!     memory new=217_494_981 free=28_488_457 net=189_006_524
//!   }
//!   Kdl {
//!     time "149.249652517s"
//!     memory new=5_219_542_533 free=4_305_774_657 net=913_767_876
//!   }
//! }
//! bench "html-standard.kdl" {
//!   JustKdlDom {
//!     time "5.756129305s"
//!     memory new=357_151_222 free=35_738_264 net=321_412_958
//!   }
//!   Kdl {
//!     time "211.812757636s"
//!     memory new=8_146_781_320 free=6_584_885_611 net=1_561_895_709
//!   }
//! }
//! ```
//! In summary:
//! - 19-36 times faster (on average, will likely be less in practice)
//! - *significantly* fewer temporary allocations
//! - fewer output allocations (even with cleared formatting!)
//!
//! [kdl]: <https://kdl.dev>
//! [kdl-rs]: https://docs.rs/kdl

use std::borrow::Cow;
use std::fmt;

pub mod dom;
pub mod number;
pub mod stream;

#[cfg(test)]
mod tests;

fn cow_static<T: ?Sized + ToOwned>(value: Cow<'_, T>) -> Cow<'static, T> {
	Cow::Owned(value.into_owned())
}

struct IdentDisplay<'text>(&'text str);
impl fmt::Display for IdentDisplay<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = self.0;
		let is_number_like = {
			let text = text.strip_prefix('+').unwrap_or(text);
			let text = text.strip_prefix('-').unwrap_or(text);
			let text = text.strip_prefix('.').unwrap_or(text);
			matches!(text.chars().next(), Some('0'..='9'))
		};
		if text.is_empty()
			|| is_number_like
			|| text.contains([
				'\u{0}', '\u{1}', '\u{2}', '\u{3}', '\u{4}', '\u{5}', '\u{6}', '\u{7}', '\u{8}',
				'\u{E}', '\u{F}', '\u{10}', '\u{11}', '\u{12}', '\u{13}', '\u{14}', '\u{15}',
				'\u{16}', '\u{17}', '\u{18}', '\u{19}', '\u{1A}', '\u{1B}', '\u{1C}', '\u{1D}',
				'\u{1E}', '\u{1F}', '\u{7F}', '\u{200E}', '\u{200F}', '\u{202A}', '\u{202B}',
				'\u{202C}', '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
				'\u{FEFF}', '\\', '/', '(', ')', '{', '}', ';', '[', ']', '"', '#', '=', '\u{9}',
				'\u{20}', '\u{A0}', '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}',
				'\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}', '\u{200A}',
				'\u{202F}', '\u{205F}', '\u{3000}', '\u{A}', '\u{B}', '\u{C}', '\u{D}', '\u{85}',
				'\u{2028}', '\u{2029}',
			]) {
			f.write_str("\"")?;
			for ch in text.chars() {
				match ch {
					'\u{8}' => f.write_str("\\b"),
					'\u{C}' => f.write_str("\\f"),
					'\'' => f.write_str("'"),
					_ => fmt::Display::fmt(&ch.escape_debug(), f),
				}?;
			}
			f.write_str("\"")
		} else {
			fmt::Display::fmt(&text, f)
		}
	}
}
