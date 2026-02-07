// SPDX-License-Identifier: MIT OR Apache-2.0
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
//! [![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/rsutil/tree/main/just-kdl)
//! [![Crates.io](https://img.shields.io/crates/v/just-kdl)](https://crates.io/crates/just-kdl)
//! [![docs.rs](https://img.shields.io/docsrs/just-kdl)](https://docs.rs/just-kdl)
//! [![MIT OR Apache-2.0](https://img.shields.io/crates/l/just-kdl)](https://github.com/1e1001/rsutil/blob/main/just-kdl/README.md#License)
//!
//! Small streaming [KDL] v2.0.1 parser
//!
//! Designed for reasonable performance and memory efficiency, at the expense
//! (or benefit, depending on use) of not storing formatting information.
//!
//! # Examples
//! ```
//! use just_kdl::dom::{Document, Event, Entry, Node, Value};
//! use just_kdl::reader::Reader;
//! use just_kdl::writer::Writer;
//! let text = "an example; kdl {document}";
//! ```
//! To read a structured [`Document`] you can use the
//! [`FromIterator`] implementation:
//! ```
//! # use just_kdl::dom::{Document, Entry, Node, Value};
//! # use just_kdl::reader::Reader;
//! # let text = "an example; kdl {document}";
//! let document = Reader::new(text.as_bytes())
//!     .collect::<Result<Document, _>>()
//!     .expect("syntax error");
//! assert_eq!(document, Document::from(vec![
//!     Node {
//!         name: "an".into(),
//!         entries: vec![Entry::new_value(Value::String("example".into()))],
//!         ..Default::default()
//!     },
//!     Node {
//!         name: "kdl".into(),
//!         children: Some(vec![Node::new("document")].into()),
//!         ..Default::default()
//!     },
//! ]));
//! ```
//! Or just use the [`Reader`] directly to get a stream of
//! [`Event`]s:
//! ```
//! # use just_kdl::dom::{Event, Entry, Value};
//! # use just_kdl::reader::Reader;
//! # let text = "an example; kdl {document}";
//! let events = Reader::new(text.as_bytes())
//!     .collect::<Result<Vec<_>, _>>()
//!     .expect("syntax error");
//! assert_eq!(events, &[
//!     (Event::Node { r#type: None, name: "an".into() }, 0..2),
//!         (Event::Entry(Entry::new_value(Value::String("example".into()))), 3..10),
//!     (Event::End, 10..11),
//!     (Event::Node { r#type: None, name: "kdl".into() }, 12..15),
//!         (Event::Children, 16..17),
//!         (Event::Node { r#type: None, name: "document".into() }, 17..25),
//!         (Event::End, 25..25),
//!     (Event::End, 25..26),
//! ]);
//! ```
//! Because formatting information is lost, writing applies a default format
//! style.
//!
//! For document oriented uses, [`Document`], [`Node`], [`Entry`], and [`Value`]
//! all implement [`Display`]:
//! ```
//! # use just_kdl::dom::{Document, Entry, Node, Value};
//! # use just_kdl::reader::Reader;
//! # let text = "an example; kdl {document}";
//! # let document = Reader::new(text.as_bytes())
//! #     .collect::<Result<Document, _>>()
//! #     .expect("syntax error");
//! assert_eq!(document.to_string(), "
//! an example
//! kdl {
//!     document
//! }
//! ".trim());
//! ```
//! And for stream-oriented uses, use the [`Writer`]:
//! ```
//! # use just_kdl::dom::{Event, Entry, Value};
//! # use just_kdl::reader::Reader;
//! # use just_kdl::writer::Writer;
//! # let text = "an example; kdl {document}";
//! # let events = Reader::new(text.as_bytes())
//! #     .collect::<Result<Vec<_>, _>>()
//! #     .expect("syntax error");
//! let mut output = String::new();
//! let mut writer = Writer::new(&mut output);
//! for (event, _) in events {
//!     writer.push(event);
//! }
//! assert_eq!(output, "
//! an example
//! kdl {
//!     document
//! }
//! ".trim());
//! ```
//!
//! [`Document`]: dom::Document
//! [`Node`]: dom::Node
//! [`Entry`]: dom::Entry
//! [`Event`]: dom::Event
//! [`Value`]: dom::Value
//! [`Reader`]: reader::Reader
//! [`Writer`]: writer::Writer
//! [`Display`]: fmt::Display
//!
//! ## Why?
//!
//! The [official Rust implementation][kdl-rs] is designed to support *editing*
//! of KDL files. While this is normally useful, if you just need to parse the
//! file into an internal data structure (configuration, document trees, etc.),
//! the formatting information is entirely redundant, wasting parse time and
//! memory.
//!
//! Additionally, this implementation has a few other benefits:
//! - Full compliance with the v2.0.1 specification
//! - Significantly fewer dependencies!
//! - `alloc`-only (`no_std`) support
//!
//! ## Benchmarks
//!
//! On my personal laptop, (i5-1240P, power-saver profile):
//! |Opt.|Parser|Benchmark|Time|Alloc|Resize|Free|Net|
//! |:-|:-|:-|:-|:-|:-|:-|:-|
//! |Release|`kdl-org/kdl`|`html-standard.kdl`|14.074s|7.2GiB|205.0MiB|5.9GiB|1.5GiB|
//! |Debug|`kdl-org/kdl`|`html-standard.kdl`|141.845s|7.2GiB|205.0MiB|5.9GiB|1.5GiB|
//! |Release|`just-kdl`|`html-standard.kdl`|0.930s|290.2MiB|40.6MiB|6.0MiB|324.8MiB|
//! |Debug|`just-kdl`|`html-standard.kdl`|4.788s|290.2MiB|40.6MiB|6.0MiB|324.8MiB|
//! |Release|`just-kdl` (Read)|`html-standard.kdl`|1.053s|290.2MiB|40.6MiB|6.0MiB|324.8MiB|
//! |Debug|`just-kdl` (Read)|`html-standard.kdl`|5.939s|290.2MiB|40.6MiB|6.0MiB|324.8MiB|
//! |Release|`kdl-org/kdl`|`html-standard-compact.kdl`|9.321s|4.5GiB|163.1MiB|3.9GiB|871.4MiB|
//! |Debug|`kdl-org/kdl`|`html-standard-compact.kdl`|101.818s|4.5GiB|163.1MiB|3.9GiB|871.4MiB|
//! |Release|`just-kdl`|`html-standard-compact.kdl`|0.730s|165.0MiB|35.1MiB|5.9MiB|194.2MiB|
//! |Debug|`just-kdl`|`html-standard-compact.kdl`|3.977s|165.0MiB|35.1MiB|5.9MiB|194.2MiB|
//! |Release|`just-kdl` (Read)|`html-standard-compact.kdl`|0.759s|165.0MiB|35.1MiB|5.9MiB|194.2MiB|
//! |Debug|`just-kdl` (Read)|`html-standard-compact.kdl`|4.704s|165.0MiB|35.1MiB|5.9MiB|194.2MiB|
//!
//! <small>(Read) = with `std::io::Read` overhead.
//! [Benchmark source][bsrc]</small>
//!
//! In summary:
//! - 10-15× faster in Release, 25-30× faster in Debug
//! - *Significantly* fewer temporary allocations
//! - Fewer final output allocations
//!
//! [kdl]: <https://kdl.dev>
//! [kdl-rs]: https://docs.rs/kdl
//! [bsrc]: https://github.com/1e1001/rsutil/blob/main/just-kdl/examples/benchmark.rs

extern crate alloc;

use core::fmt;

pub mod dom;
pub mod lexer;
pub mod reader;
mod ssb2;
pub mod validator;
pub mod writer;
// TODO: serde integration

#[cfg(test)]
mod tests;

/// if an ident can be misinterpreted as a number
fn is_ambiguous_ident(text: &str) -> bool {
	let text = text.strip_prefix(['+', '-']).unwrap_or(text);
	let text = text.strip_prefix('.').unwrap_or(text);
	matches!(text.chars().next(), Some('0'..='9'))
}

/// kdl-compatible text formatting
struct IdentDisplay<'text>(&'text str);
impl fmt::Display for IdentDisplay<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let text = self.0;
		if text.is_empty()
			|| is_ambiguous_ident(text)
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
