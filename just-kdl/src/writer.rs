// SPDX-License-Identifier: MIT OR Apache-2.0
//! Write an event stream to an output.
//!
//! You probably want to start at [`Writer`].

// TODO: is there much use for a "minified" writer?

use core::fmt;

use crate::IdentDisplay;
use crate::dom::Event;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Writer of document events.
pub struct Writer<W> {
	inner: W,
	state: State,
	indent: usize,
	indent_text: &'static str,
}

// since the writer doesn't support comments, there's no need to track a
// comment-depth stack
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum State {
	First,
	BlockStart,
	Block,
	Node,
}

impl<W: fmt::Write> Writer<W> {
	/// Create a new writer with an output.
	pub const fn new(writer: W) -> Self {
		Self {
			inner: writer,
			state: State::First,
			indent: 0,
			indent_text: "    ",
		}
	}
	/// Change the text inserted for each indentation level,
	/// default is four spaces
	pub fn set_indent(&mut self, indent: &'static str) { self.indent_text = indent; }
	fn line(&mut self) -> fmt::Result {
		writeln!(self.inner)?;
		for _ in 0..self.indent {
			write!(self.inner, "{}", self.indent_text)?;
		}
		Ok(())
	}
	/// Write an event to the writer
	/// # Errors
	/// If the inner writer errors
	pub fn push(&mut self, event: &Event) -> fmt::Result {
		match event {
			Event::Node { r#type, name } => {
				if self.state != State::First {
					self.line()?;
				}
				self.state = State::Node;
				if let Some(r#type) = r#type {
					write!(self.inner, "({})", IdentDisplay(r#type))?;
				}
				write!(self.inner, "{}", IdentDisplay(name))
			}
			Event::Entry(entry) => write!(self.inner, " {entry}"),
			Event::Children => {
				self.indent += 1;
				self.state = State::BlockStart;
				write!(self.inner, " {{")
			}
			Event::End => {
				match self.state {
					State::BlockStart => {
						self.indent -= 1;
						write!(self.inner, "}}")?;
					}
					State::Block => {
						self.indent -= 1;
						self.line()?;
						write!(self.inner, "}}")?;
					}
					_ => {}
				}
				self.state = State::Block;
				Ok(())
			}
		}
	}
}

/// Bridge from [`fmt::Write`] to [`std::io::Write`], for writing to IO streams.
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WriteOutput<T>(pub T);

#[cfg(feature = "std")]
#[expect(clippy::absolute_paths, reason = "feature-gated")]
impl<T: std::io::Write> fmt::Write for WriteOutput<T> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)
	}
}
