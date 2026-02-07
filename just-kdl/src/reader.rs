// SPDX-License-Identifier: MIT OR Apache-2.0
//! Read document events out of a file.
//!
//! You probably want to start at [`Reader`].

// TODO: consider re-merging lexer (or at least using hinted lexing?)

use alloc::vec::Vec;
use core::ops::Range;

use smol_str::SmolStr;
use thiserror::Error;

use crate::dom::{Entry, Event, Value};
use crate::lexer::{Input, Lexer, LexerError, Token};

/// An error while reading
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReaderError {
	#[error(transparent)]
	/// Inner lexer error, includes IO errors
	Lexer(LexerError),
	#[error("Expected string, got {0}")]
	#[doc = "Expected string, got {0}"]
	ExpectedString(Token),
	#[error("Expected value, got {0}")]
	#[doc = "Expected value, got {0}"]
	ExpectedValue(Token),
	#[error("Expected ')', got {0}")]
	#[doc = "Expected ')', got {0}"]
	ExpectedCloseParen(Token),
	#[error("Unclosed '{{' before end of file")]
	#[doc = "Unclosed '{' before end of file"]
	UnclosedOpen,
	#[error("Unmatched '}}'")]
	#[doc = "Unmatched '}'"]
	UnmatchedClose,
	#[error("Expected '/-' or space before entry")]
	#[doc = "Expected '/-' or space before entry"]
	ExpectedEntrySpace,
	#[error("Expected entry, block, or end of node")]
	#[doc = "Expected entry, block, or end of node"]
	ExpectedEntry,
	#[error("Expected block or end of node")]
	#[doc = "Expected block or end of node"]
	ExpectedBlock,
}
/// Value (event, error) with a span attached
pub type Spanned<T> = (T, Range<usize>);
// internal result with error spans
type ReaderResult<T> = Result<T, Spanned<ReaderError>>;

/// Event that might be commented
#[derive(PartialEq)]
enum InnerEvent {
	/// none = skip or sd
	Node(Option<(Option<SmolStr>, SmolStr)>),
	/// none = skip or sd, type, key, value
	Entry(Option<(Option<SmolStr>, Option<SmolStr>, Value)>),
	/// true = sd
	Children(bool),
	/// true = sd children block
	End(bool),
	Done,
}

enum State {
	Begin,
	NextNode,
	/// true = pre-spaced
	NodeEntries(bool),
	/// true = real part of node
	NodeChildren(bool),
	Done,
}

/// Reader of document events.
pub struct Reader<T> {
	// TODO: generic token source, if there's a use case for that
	lexer: Lexer<T>,
	// TODO/perf: remove implicit peek/advance pairs in mid-parsing code, looks awful
	peek_token: Option<Spanned<Token>>,
	state: State,
	/// current children block depth, true = sd
	// TODO/perf: replace with a bitwise vector of some kind
	brackets: Vec<bool>,
}

impl<T: Input> Reader<T> {
	/// Create a new reader from an input.
	pub fn new(input: T) -> Self { Self::from_lexer(Lexer::new(input)) }
	/// Create a new reader directly from the token source.
	pub fn from_lexer(lexer: Lexer<T>) -> Self {
		Self {
			lexer,
			peek_token: None,
			state: State::Begin,
			brackets: Vec::new(),
		}
	}
	fn peek(&mut self, skip: bool) -> ReaderResult<&Spanned<Token>> {
		// some weird lifetimes here, but it works
		let mut res = Ok(());
		let token = self.peek_token.get_or_insert_with(|| {
			let (token, pos) = self.lexer.next_token(skip);
			// this returns a single character span for Spaces / Lines,
			// that's OK as those spans are only used in:
			// - Event::End - where we only want one character of span anyways
			// - Errors, where preserving the end of span isn't too important (no recovery)
			let span = pos..self.lexer.current_position();
			match token {
				Ok(token) => (token, span),
				Err(err) => {
					res = Err((ReaderError::Lexer(err), span.clone()));
					(Token::Eof, span)
				}
			}
		});
		res.map(|()| &*token)
	}
	#[track_caller]
	fn advance(&mut self) -> Spanned<Token> { self.peek_token.take().unwrap() }
	fn skip_spaces(&mut self, skip: bool) -> ReaderResult<bool> {
		if self.peek(skip)?.0 == Token::Spaces {
			self.advance();
			Ok(true)
		} else {
			Ok(false)
		}
	}
	fn skip_lines(&mut self, skip: bool) -> ReaderResult<()> {
		match self.peek(skip)?.0 {
			Token::Spaces => {
				self.advance();
				if self.peek(skip)?.0 == Token::Lines {
					self.advance();
				}
			}
			Token::Lines => {
				self.advance();
			}
			_ => {}
		}
		Ok(())
	}
	fn string(token: Spanned<Token>) -> ReaderResult<Option<SmolStr>> {
		Ok(match token.0 {
			Token::String(text) => Some(text),
			Token::SkippedString => None,
			_ => return Err((ReaderError::ExpectedString(token.0), token.1)),
		})
	}
	fn value(skip: bool, token: Spanned<Token>) -> ReaderResult<Option<Value>> {
		Ok(match token.0 {
			Token::SkippedString | Token::SkippedNumber => None,
			// do base64/base85 decoding
			Token::String(text) => Some(Value::String(text)),
			Token::Number(number) => Some(Value::Number(number)),
			Token::Bool(value) => (!skip).then_some(Value::Bool(value)),
			Token::Null => (!skip).then_some(Value::Null),
			_ => return Err((ReaderError::ExpectedValue(token.0), token.1)),
		})
	}
	/// String ) Spaces?
	fn type_body(&mut self, skip: bool) -> ReaderResult<Option<SmolStr>> {
		self.skip_spaces(skip)?;
		self.peek(skip)?;
		let text = Self::string(self.advance())?;
		self.skip_spaces(skip)?;
		self.peek(skip)?;
		let close = self.advance();
		if close.0 != Token::CloseParen {
			return Err((ReaderError::ExpectedCloseParen(close.0), close.1));
		}
		self.skip_spaces(skip)?;
		Ok(text)
	}
	/// None = no type or skipped
	fn maybe_type(&mut self, skip: bool) -> ReaderResult<Option<SmolStr>> {
		Ok(if self.peek(skip)?.0 == Token::OpenParen {
			self.advance();
			self.type_body(skip)?
		} else {
			None
		})
	}
	/// (sd, skip)
	fn maybe_slash_dash(&mut self, skip: bool) -> ReaderResult<(bool, bool)> {
		if self.peek(skip)?.0 == Token::SlashDash {
			self.advance();
			self.skip_lines(true)?;
			Ok((true, true))
		} else {
			Ok((false, skip))
		}
	}
	#[expect(clippy::too_many_lines, reason = "too lazy to fix this")]
	fn next_inner_event(&mut self, skip: bool) -> ReaderResult<Spanned<InnerEvent>> {
		match self.state {
			State::Begin | State::NextNode => {
				if self.peek(skip)?.0 == Token::Bom {
					self.advance();
				}
				self.skip_lines(skip)?;
				let case_token = self.peek(skip)?;
				let span = case_token.1.clone();
				match case_token.0 {
					Token::Eof => {
						if !self.brackets.is_empty() {
							return Err((ReaderError::UnclosedOpen, span));
						}
						self.state = State::Done;
						return Ok((InnerEvent::Done, span));
					}
					Token::CloseCurly => {
						let span = span.clone();
						let Some(pop) = self.brackets.pop() else {
							return Err((ReaderError::UnmatchedClose, span));
						};
						self.advance();
						self.state = State::NodeChildren(pop);
						return Ok((InnerEvent::End(pop), span));
					}
					_ => {}
				}
				let (_, skip) = self.maybe_slash_dash(skip)?;
				let r#type = self.maybe_type(skip)?;
				self.peek(skip)?;
				let name_token = self.advance();
				let span = span.start..name_token.1.end;
				let name = Self::string(name_token)?;
				self.state = State::NodeEntries(false);
				Ok((InnerEvent::Node(name.map(|name| (r#type, name))), span))
			}
			State::NodeEntries(_) | State::NodeChildren(_) => {
				let (spaces, entries, real_body) = match self.state {
					State::NodeEntries(spaces) => {
						// reset space state now for following events
						self.state = State::NodeEntries(false);
						(spaces, true, true)
					}
					State::NodeChildren(real_body) => (false, false, real_body),
					_ => unreachable!(),
				};
				let spaces = spaces || self.skip_spaces(skip)?;
				let case_token = self.peek(skip)?;
				let case_span = case_token.1.clone();
				let start = case_token.1.start;
				if matches!(
					case_token.0,
					Token::Eof | Token::CloseCurly | Token::SemiColon | Token::Lines
				) {
					let span = if matches!(case_token.0, Token::SemiColon | Token::Lines) {
						self.advance();
						case_span
					} else {
						// keep spans incrementing
						case_span.start..case_span.start
					};
					self.state = State::NextNode;
					return if real_body {
						Ok((InnerEvent::End(false), span))
					} else {
						// this event doesn't require any span, but i have one anyways
						Ok((InnerEvent::Entry(None), span))
					};
				}
				let (sd, skip_or_sd) = self.maybe_slash_dash(skip)?;
				let mv_token = self.peek(skip_or_sd)?;
				let mv_span = mv_token.1.clone();
				match mv_token.0 {
					Token::OpenCurly if real_body || sd => {
						self.advance();
						self.brackets.push(sd && real_body);
						self.state = State::NextNode;
						Ok((InnerEvent::Children(sd), start..mv_span.end))
					}
					_ if !entries => Err((ReaderError::ExpectedBlock, start..mv_span.end)),
					_ if !sd && !spaces => {
						Err((ReaderError::ExpectedEntrySpace, start..mv_span.end))
					}
					// value or key, unsure
					Token::String(_) | Token::SkippedString => {
						let first = self.advance();
						// NOTE: if skipping, the next (property) might be peeked. if outer-reader
						// skip ever stops after a Entry, change this to a proper difference!
						let next_spaces = self.skip_spaces(skip)?;
						if self.peek(skip)?.0 == Token::Equals {
							let name = Self::string(first)?;
							self.advance();
							self.skip_spaces(skip_or_sd)?;
							let r#type = self.maybe_type(skip_or_sd)?;
							self.peek(skip_or_sd)?;
							let token = self.advance();
							let span = start..token.1.end;
							let value = Self::value(skip_or_sd, token)?;
							Ok((
								InnerEvent::Entry(
									name.zip(value)
										.map(|(key, value)| (r#type, Some(key), value)),
								),
								span,
							))
						} else {
							// consumed spaces that might be needed for the next entry
							self.state = State::NodeEntries(next_spaces);
							let span = start..first.1.end;
							let value = Self::value(skip_or_sd, first)?;
							Ok((
								InnerEvent::Entry(value.map(|value| (None, None, value))),
								span,
							))
						}
					}
					Token::Number(_) | Token::SkippedNumber | Token::Bool(_) | Token::Null => {
						let token = self.advance();
						let span = start..token.1.end;
						let value = Self::value(skip_or_sd, token)?;
						Ok((
							InnerEvent::Entry(value.map(|value| (None, None, value))),
							span,
						))
					}
					Token::OpenParen => {
						self.advance();
						let r#type = self.type_body(skip_or_sd)?;
						self.peek(skip_or_sd)?;
						let token = self.advance();
						let span = start..token.1.end;
						let value = Self::value(skip_or_sd, token)?;
						Ok((
							InnerEvent::Entry(value.map(|value| (r#type, None, value))),
							span,
						))
					}
					_ => Err((ReaderError::ExpectedEntry, start..mv_span.end)),
				}
			}
			State::Done => {
				let pos = self.lexer.current_position();
				Ok((InnerEvent::Done, pos..pos))
			}
		}
	}
	fn skip_bracketed(&mut self, open: &InnerEvent, close: &InnerEvent) -> ReaderResult<()> {
		let mut counter = 0_usize;
		loop {
			let event = self.next_inner_event(true)?.0;
			if event == InnerEvent::Done {
				// inner reader handles bracket mismatches, and this is trivially trigger-able
				break;
			}
			if &event == open {
				counter += 1;
			} else if &event == close {
				if let Some(next) = counter.checked_sub(1) {
					counter = next;
				} else {
					break;
				}
			}
		}
		Ok(())
	}
	fn next_event(&mut self) -> ReaderResult<Option<Spanned<Event>>> {
		Ok(Some(loop {
			let (event, span) = self.next_inner_event(false)?;
			break (
				match event {
					InnerEvent::Node(Some((r#type, name))) => Event::Node { r#type, name },
					InnerEvent::Node(None) => {
						self.skip_to_end()?;
						continue;
					}
					InnerEvent::Entry(Some((r#type, key, value))) => Event::Entry(Entry {
						r#type,
						name: key,
						value,
					}),
					InnerEvent::Entry(None) => continue,
					InnerEvent::Children(true) => {
						self.skip_bracketed(&InnerEvent::Children(true), &InnerEvent::End(true))?;
						continue;
					}
					InnerEvent::Children(false) => Event::Children,
					InnerEvent::End(true) => unreachable!("bad stream"),
					InnerEvent::End(false) => Event::End,
					InnerEvent::Done => return Ok(None),
				},
				span,
			);
		}))
	}
	/// Skip to the next node. When currently reading a node (i.e. after a
	/// `Node` event) this ends the node, otherwise it goes to the parent block
	///
	/// # Errors
	/// On any syntax errors.
	pub fn skip_to_end(&mut self) -> ReaderResult<()> {
		self.skip_bracketed(&InnerEvent::Node(None), &InnerEvent::End(false))
	}
}

/// Read one event at a time.
impl<T: Input> Iterator for Reader<T> {
	type Item = ReaderResult<Spanned<Event>>;
	fn next(&mut self) -> Option<Self::Item> {
		self.next_event()
			// stop reader after errors
			.inspect_err(|_| self.state = State::Done)
			.transpose()
	}
}
