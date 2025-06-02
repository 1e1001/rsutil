// SPDX-License-Identifier: MIT OR Apache-2.0
//! The actual parser
//!
//! While this is technically a streaming parser, it operates on a complete
//! `&str` data, and requires that data to be borrowed for the duration of the
//! returned events.

// TODO: spec-breaking configs:
// - v2_0_1: draft spec syntax differences
// - really_raw: allow arbitrary bytes in raw strings, including newlines in
//   single-line strings (which remain unprocessed)
// TODO: some alternative input api: peek/consume utf-8 stream?
// TODO: fuzzing!

use std::borrow::Cow;
use std::fmt;

use crate::dom::Value;
use crate::number::{Number, NumberError};
use crate::{IdentDisplay, cow_static};

/// A parsing error
/// `usize` arguments are byte positions in the source text
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
	/// A space character was expected
	ExpectedSpace(usize),
	/// A `)` was expected
	ExpectedCloseParen(usize),
	/// A single-line comment was expected
	ExpectedComment(usize),
	/// A mandatory newline was expected here
	ExpectedNewline(usize),
	/// A string or identifier was expected
	ExpectedString(usize),
	/// A value of some kind was expected
	ExpectedValue(usize),
	/// A `}` was placed where it shouldn't be
	UnexpectedCloseBracket(usize),
	/// A newline is not allowed here
	UnexpectedNewline(usize),
	/// A number is invalid
	InvalidNumber(usize),
	/// A keyword has an invalid name
	BadKeyword(usize),
	/// An identifier with an invalid name
	BadIdentifier(usize),
	/// An invalid escape at this position
	BadEscape(usize),
	/// The indentation for this line doesn't match the string pattern
	BadIndent(usize),
	/// Multiple children blocks are present for one node
	MultipleChildren(usize),
	/// The file ended too early
	UnexpectedEof,
	/// An always-invalid character at this position
	BannedChar(char, usize),
}

type PResult<T> = Result<T, Error>;

/// a parsing event
#[derive(Debug)]
pub enum Event<'text> {
	/// A new node
	Node {
		/// Optional node type hint
		r#type: Option<Cow<'text, str>>,
		/// Node name
		name: Cow<'text, str>,
	},
	/// A node value or property
	Entry {
		/// The key, if it exists
		key: Option<Cow<'text, str>>,
		/// Optional value type hint
		r#type: Option<Cow<'text, str>>,
		/// Value
		value: Value<'text>,
	},
	/// Start of children list for the previous node,
	/// will only be emitted once per node
	Begin,
	/// End of children list
	End,
}

impl Event<'_> {
	/// Convert into an owned value
	pub fn into_static(self) -> Event<'static> {
		match self {
			Self::Node { r#type, name } => Event::Node {
				r#type: r#type.map(cow_static),
				name: cow_static(name),
			},
			Self::Entry { key, r#type, value } => Event::Entry {
				key: key.map(cow_static),
				r#type: r#type.map(cow_static),
				value: value.into_owned(),
			},
			Self::Begin => Event::Begin,
			Self::End => Event::End,
		}
	}
}

#[derive(Debug)]
enum InnerEvent<'text> {
	Node {
		sd: bool,
		r#type: Option<Cow<'text, str>>,
		name: Cow<'text, str>,
	},
	PropValue {
		sd: bool,
		r#type: Option<Cow<'text, str>>,
		key: Option<Cow<'text, str>>,
		value: Value<'text>,
	},
	Begin {
		sd: bool,
	},
	End,
	Done,
}

enum ParserState {
	/// right after init
	BeginDocument,
	/// next item is a node or ending
	NextNode,
	/// while parsing a node
	NodeProps,
	/// only children left
	NodeChildren,
	/// die
	Done,
}

/// A value that's been parsed enough to differentiate it
enum SemiValue<'text> {
	/// `string` always
	String(Cow<'text, str>),
	/// `number` or invalid
	Number(&'text str),
	/// `keyword` or invalid
	Keyword(&'text str),
}

/// parsing position
#[repr(transparent)]
#[derive(Clone, Copy)]
struct Pos(usize);

impl Pos {
	fn offset_bytes(self, n: usize) -> Self { Self(self.0 + n) }
	fn offset_char(self, ch: char) -> Self { self.offset_bytes(ch.len_utf8()) }
	fn offset_str(self, text: &str) -> Self { self.offset_bytes(text.len()) }
}

struct Grammar<'text>(&'text str);

// in this impl: anything in `backticks` (except that)
// represents a kdl grammar item or expression
impl<'text> Grammar<'text> {
	fn tail(&self, at: Pos) -> &str { &self.0[at.0..] }
	// TODO: i realize now this could be written a lot better as a
	// "(&Self, Pos) -> (char, Pos)", kinda like every other parser
	// would require rewriting every single parse rule but could be nice
	fn top_char(&self, at: Pos) -> Option<char> { self.tail(at).chars().next() }
	/// `bom`
	fn bom(&self, at: Pos) -> Option<Pos> {
		(self.top_char(at) == Some('\u{FEFF}')).then(|| at.offset_char('\u{FEFF}'))
	}
	// `disallowed-literal-code-points`
	fn banned(ch: char) -> bool {
		// D800-DFFF are not allowed by rust char
		matches!(ch, '\u{0}'..='\u{8}' | '\u{E}'..='\u{1F}' | '\u{7F}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
	}
	// `identifier-char`
	fn ident(ch: char) -> bool {
		!(Self::banned(ch)
			|| Self::space(ch)
			|| Self::newline(ch)
			|| matches!(
				ch,
				'\\' | '/' | '(' | ')' | '{' | '}' | ';' | '[' | ']' | '"' | '#' | '='
			))
	}
	// `unicode-space`
	fn space(ch: char) -> bool {
		matches!(
			ch,
			'\u{9}' | '\u{20}' | '\u{A0}' | '\u{1680}' | '\u{2000}'
				..='\u{200A}' | '\u{202F}' | '\u{205F}' | '\u{3000}'
		)
	}
	// `newline`
	fn newline(ch: char) -> bool {
		matches!(ch, '\u{A}'..='\u{D}' | '\u{85}' | '\u{2028}' | '\u{2029}')
	}
	fn number_like(text: &str) -> bool {
		let text = text.strip_prefix(['+', '-']).unwrap_or(text);
		let text = text.strip_prefix('.').unwrap_or(text);
		text.as_bytes().first().is_some_and(u8::is_ascii_digit)
	}
	/// `number`, assuming text is a valid ident
	fn all_number(&self, at: Pos) -> Result<Number, NumberError> {
		#[derive(Clone, Copy)]
		enum Radix {
			Binary = 2,
			Octal = 8,
			Decimal = 10,
			Hexadecimal = 16,
		}
		fn append(
			buf: &mut String,
			state: &mut bool,
			ch: char,
			radix: Radix,
		) -> Result<(), NumberError> {
			*state = match (radix, ch) {
				(_, '_') if *state => return Ok(()),
				(Radix::Binary, '0'..='1')
				| (Radix::Octal, '0'..='7')
				| (Radix::Decimal, '0'..='9')
				| (Radix::Hexadecimal, '0'..='9' | 'a'..='f' | 'A'..='F') => true,
				(Radix::Decimal, '.' | 'e' | 'E') if *state => false,
				(Radix::Decimal, '+' | '-') => false,
				_ => return Err(NumberError::BadSyntax),
			};
			buf.push(ch);
			Ok(())
		}
		// sign: +? uses unsigned, - uses signed
		// [+-]?0b[01][01_]* -> int base 2
		// [+-]?0o[0-7][0-7_]* -> int base 2
		// [+-]?0x[0-9a-fA-F][0-9a-fA-F_]* -> int base 2
		// [+-]?[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][+-]?[0-9][0-9_]*)?
		let (at, negative) = match self.top_char(at) {
			Some('-') => (at.offset_char('-'), true),
			Some('+') => (at.offset_char('+'), false),
			_ => (at, false),
		};
		// TODO: this can definitely be done without allocating,
		// but i don't know a good way of doing it without rewriting f64::from_str
		let mut buffer = if negative {
			"-".to_owned()
		} else {
			String::new()
		};
		let mut state = false;
		let (at, radix) = match self.tail(at).as_bytes() {
			[b'0', b'b', ..] => (at.offset_str("0b"), Radix::Binary),
			[b'0', b'o', ..] => (at.offset_str("0o"), Radix::Octal),
			[b'0', b'x', ..] => (at.offset_str("0x"), Radix::Hexadecimal),
			_ => (at, Radix::Decimal),
		};
		for ch in self.tail(at).chars() {
			append(&mut buffer, &mut state, ch, radix)?;
		}
		let radix = radix as u32;
		if let Ok(value) = u64::from_str_radix(&buffer, radix) {
			Ok(Number::from_u64(value))
		} else if let Ok(value) = i64::from_str_radix(&buffer, radix) {
			Ok(Number::from_i64(value))
		} else if radix == 10 {
			if buffer.ends_with('.') {
				Err(NumberError::BadSyntax)
			} else if let Ok(value) = buffer.parse() {
				Ok(Number::from_f64(value))
			} else {
				Err(NumberError::BadSyntax)
			}
		} else {
			Err(NumberError::BadSyntax)
		}
	}
	/// `single-line-comment` after `//`
	/// = `^newline* (newline | eof)`
	fn single_line_comment(&self, mut at: Pos) -> PResult<Pos> {
		loop {
			match self.top_char(at) {
				Some(ch) if Self::banned(ch) => return Err(Error::BannedChar(ch, at.0)),
				Some(ch) if Self::newline(ch) => return Ok(at.offset_char(ch)),
				None => return Ok(at),
				Some(ch) => at = at.offset_char(ch),
			}
		}
	}
	// `multi-line-comment` after `/*`
	// = `(…) */`
	fn multi_line_comment(&self, mut at: Pos) -> PResult<Pos> {
		let mut nest = 0_usize;
		loop {
			if self.tail(at).starts_with("*/") {
				at = at.offset_str("*/");
				if let Some(next) = nest.checked_sub(1) {
					nest = next;
				} else {
					return Ok(at);
				}
			} else if self.tail(at).starts_with("/*") {
				at = at.offset_str("/*");
				// i don't care about overflow here,
				// if you open 4 billion comments that's your problem
				nest += 1;
			} else {
				match self.top_char(at) {
					Some(ch) if Self::banned(ch) => return Err(Error::BannedChar(ch, at.0)),
					Some(ch) => at = at.offset_char(ch),
					None => return Err(Error::UnexpectedEof),
				}
			}
		}
	}
	// `escline` after `\`
	// = `ws* (single-line-comment | newline | eof)`
	fn escline(&self, mut at: Pos) -> PResult<Pos> {
		// valid: `unicode-space` `multi-line-comment`
		loop {
			match self.top_char(at) {
				Some(ch) if Self::space(ch) => {
					at = at.offset_char(ch);
				}
				Some('/') if self.top_char(at.offset_char('/')) == Some('*') => {
					at = self.multi_line_comment(at.offset_str("/*"))?;
				}
				_ => break,
			}
		}
		// valid: `single-line-comment` `newline` `eof`
		match self.top_char(at) {
			Some('/') if self.top_char(at.offset_char('/')) == Some('/') => {
				self.single_line_comment(at.offset_str("//"))
			}
			Some(ch) if Self::newline(ch) => Ok(at.offset_char(ch)),
			None => Ok(at),
			_ => Err(Error::ExpectedComment(at.0)),
		}
	}
	/// `line-space*`
	fn line_space(&self, mut at: Pos) -> PResult<Pos> {
		// valid:
		//  `unicode-space` `newline` `escline`
		//  `single-line-comment` `multi-line-comment`
		loop {
			match self.top_char(at) {
				Some('\\') => at = self.escline(at.offset_char('\\'))?,
				Some('/') => match self.top_char(at.offset_char('/')) {
					Some('/') => at = self.single_line_comment(at.offset_str("//"))?,
					Some('*') => at = self.multi_line_comment(at.offset_str("/*"))?,
					_ => break,
				},
				Some(ch) if Self::newline(ch) || Self::space(ch) => {
					at = at.offset_char(ch);
				}
				_ => break,
			}
		}
		Ok(at)
	}
	// `node-space*` or `node-space+`
	fn node_space(&self, start: Pos, req: bool) -> PResult<Pos> {
		let mut at = start;
		// valid: `unicode-space` `escline` `multi-line-comment`
		loop {
			match self.top_char(at) {
				Some('\\') => {
					at = self.escline(at.offset_char('\\'))?;
				}
				Some('/') if self.top_char(at.offset_char('/')) == Some('*') => {
					at = self.multi_line_comment(at.offset_str("/*"))?;
				}
				Some(ch) if Self::space(ch) => {
					at = at.offset_char(ch);
				}
				_ => break,
			}
		}
		if !req || at.0 != start.0 {
			Ok(at)
		} else {
			Err(Error::ExpectedSpace(at.0))
		}
	}
	/// `slashdash`
	fn slash_dash(&self, at: Pos) -> PResult<Option<Pos>> {
		self.tail(at)
			.starts_with("/-")
			.then(|| self.line_space(at.offset_str("/-")))
			.transpose()
	}
	/// `identifier-string`
	fn identifier_string(&self, at: Pos) -> (Pos, &'text str) {
		let mut end = at;
		while let Some(ch) = self.top_char(end) {
			if !Self::ident(ch) {
				break;
			}
			end = end.offset_char(ch);
		}
		(end, &self.0[at.0..end.0])
	}
	/// string escape after \
	fn escape(&self, at: Pos) -> PResult<(Pos, Option<char>)> {
		let (at, ch) = match self.top_char(at).ok_or(Error::UnexpectedEof)? {
			'n' => (at.offset_char('n'), Some('\n')),
			'r' => (at.offset_char('r'), Some('\r')),
			't' => (at.offset_char('t'), Some('\t')),
			'\\' => (at.offset_char('\\'), Some('\\')),
			'"' => (at.offset_char('"'), Some('"')),
			'b' => (at.offset_char('b'), Some('\x08')),
			'f' => (at.offset_char('f'), Some('\x0C')),
			's' => (at.offset_char('s'), Some(' ')),
			'u' => {
				let start = at.offset_char('u');
				let Some('{') = self.top_char(start) else {
					return Err(Error::BadEscape(start.0));
				};
				let start = start.offset_char('{');
				let mut end = start;
				for _ in 0..6 {
					match self.top_char(end) {
						Some(ch @ ('0'..='9' | 'a'..='f' | 'A'..='F')) => end = end.offset_char(ch),
						Some('}') => break,
						_ => return Err(Error::BadEscape(at.0)),
					}
				}
				let number = u32::from_str_radix(&self.0[start.0..end.0], 16)
					.map_err(|_| Error::BadEscape(at.0))?;
				if self.top_char(end) != Some('}') {
					return Err(Error::BadEscape(at.0));
				}
				let char = char::from_u32(number).ok_or(Error::BadEscape(at.0))?;
				(end.offset_char('}'), Some(char))
			}
			ch if Self::space(ch) || Self::newline(ch) => {
				let mut at = at.offset_char(ch);
				while let Some(next) = self.top_char(at) {
					if !Self::space(next) && !Self::newline(next) {
						break;
					}
					at = at.offset_char(next);
				}
				(at, None)
			}
			_ => return Err(Error::BadEscape(at.0)),
		};
		Ok((at, ch))
	}
	fn string_end(&self, pos: Pos, multi: bool, raw: usize) -> Option<Pos> {
		self.tail(pos)
			.strip_prefix(if multi { "\"\"\"" } else { "\"" })
			.and_then(|tail| {
				tail.as_bytes()[..raw]
					.iter()
					.all(|&ch| ch == b'#')
					.then(|| pos.offset_bytes(if multi { raw + 3 } else { raw + 1 }))
			})
	}
	fn dedent_multiline(&self, first: Pos, mut lines: Vec<Pos>, raw: usize) -> PResult<String> {
		// an interesting thing to note is that whitespace escapes
		// can never be within an indent, as they'll consume all the indent afterwards
		// this also means that all indents are byte-for-byte exact
		let Some(last) = lines.pop() else {
			return Err(Error::ExpectedNewline(first.0));
		};
		// validate that last line is all space
		let indent = {
			let start = last;
			let mut first = None;
			let mut end = start;
			loop {
				match self.top_char(end) {
					Some('\\') if raw == 0 => {
						let (next, ch) = self.escape(end.offset_char('\\'))?;
						if ch.is_some() {
							return Err(Error::ExpectedSpace(end.0));
						}
						first = first.or(Some(end));
						end = next;
					}
					Some('"') if self.string_end(end, true, raw).is_some() => {
						first = first.or(Some(end));
						break;
					}
					Some(ch) => {
						if Self::space(ch) {
							end = end.offset_char(ch);
						} else {
							return Err(Error::ExpectedSpace(end.0));
						}
					}
					None => return Err(Error::UnexpectedEof),
				}
			}
			&self.tail(start)[..first.unwrap().0 - start.0]
		};
		lines
			.into_iter()
			.map(|start| {
				// entirely space line?
				let mut at = start;
				loop {
					let top = self.top_char(at).ok_or(Error::UnexpectedEof)?;
					if Self::newline(top) {
						return Ok(String::new());
					} else if !Self::space(top) {
						break;
					}
					at = at.offset_char(top);
				}
				if self.tail(start).starts_with(indent) {
					let mut at = start.offset_str(indent);
					let mut text = String::new();
					loop {
						match self.top_char(at) {
							Some('\\') if raw == 0 => {
								let (next, ch) = self.escape(at.offset_char('\\'))?;
								at = next;
								text.extend(ch);
							}
							Some(ch) => {
								if Self::newline(ch) {
									break Ok(text);
								} else {
									text.push(ch);
									at = at.offset_char(ch);
								}
							}
							None => break Err(Error::UnexpectedEof),
						}
					}
				} else {
					Err(Error::BadIndent(start.0))
				}
			})
			.collect::<PResult<Vec<_>>>()
			.map(|lines| dbg!(lines).join("\n"))
	}
	/// {single, multi}-line {raw, escaped} string, starting after the first "
	fn quoted_string(&self, start: Pos, raw: usize) -> PResult<(Pos, Cow<'text, str>)> {
		if self.tail(start).starts_with("\"\"") {
			// multi-line: `newline (line newline)* indent* """`
			// line: `indent* text*`
			let mut at = start.offset_str("\"\"");
			let mut lines = Vec::<Pos>::new();
			loop {
				match self.top_char(at) {
					Some('\\') if raw == 0 => {
						let (next, _) = self.escape(at.offset_char('\\'))?;
						at = next;
					}
					Some('"') if self.tail(at).starts_with("\"\"\"") => {
						if let Some(next) = self.string_end(at, true, raw) {
							break Ok((next, Cow::Owned(self.dedent_multiline(at, lines, raw)?)));
						}
						// more text!
						at = at.offset_str("\"\"\"");
					}
					Some(ch) => {
						if Self::newline(ch) {
							let mut next = at.offset_char(ch);
							if ch == '\r' && self.top_char(next) == Some('\n') {
								at = next;
								next = at.offset_char('\n');
							}
							lines.push(next);
						} else if Self::banned(ch) {
							return Err(Error::BannedChar(ch, at.0));
						} else if lines.is_empty() {
							return Err(Error::ExpectedNewline(at.0));
						}
						at = at.offset_char(ch);
					}
					None => return Err(Error::UnexpectedEof),
				}
			}
		} else {
			// single-line
			// none = can be borrowed
			let mut text = None::<String>;
			let mut at = start;
			loop {
				match self.top_char(at) {
					Some('\\') if raw == 0 => {
						let text = text.get_or_insert_with(|| self.0[start.0..at.0].to_owned());
						let (next, ch) = self.escape(at.offset_char('\\'))?;
						at = next;
						text.extend(ch);
					}
					Some('"') => {
						if let Some(next) = self.string_end(at, false, raw) {
							break Ok((
								next,
								text.map_or_else(
									|| Cow::Borrowed(&self.0[start.0..at.0]),
									Cow::Owned,
								),
							));
						}
						// more text!
						if let Some(text) = &mut text {
							text.push('"');
						}
						at = at.offset_char('"');
					}
					Some(ch) => {
						if Self::newline(ch) {
							return Err(Error::UnexpectedNewline(at.0));
						} else if Self::banned(ch) {
							return Err(Error::BannedChar(ch, at.0));
						}
						if let Some(text) = &mut text {
							text.push(ch);
						}
						at = at.offset_char(ch);
					}
					None => return Err(Error::UnexpectedEof),
				}
			}
		}
	}
	/// `string | number | keyword`, may be invalid
	fn semi_value(&self, at: Pos) -> PResult<(Pos, SemiValue<'text>)> {
		match self.top_char(at) {
			Some('"') => {
				let (at, text) = self.quoted_string(at.offset_char('"'), 0)?;
				Ok((at, SemiValue::String(text)))
			}
			Some('#') => {
				let start = at;
				let mut at = at.offset_char('#');
				match self.top_char(at) {
					Some(ch) if Self::ident(ch) => {
						let (at, text) = self.identifier_string(at);
						Ok((at, SemiValue::Keyword(text)))
					}
					_ => {
						let mut raw = 1;
						while let Some('#') = self.top_char(at) {
							at = at.offset_char('#');
							raw += 1;
						}
						if self.top_char(at) != Some('"') {
							return Err(Error::ExpectedString(start.0));
						}
						let (at, text) = self.quoted_string(at.offset_char('"'), raw)?;
						Ok((at, SemiValue::String(text)))
					}
				}
			}
			Some(ch) if Self::ident(ch) => {
				let (next, text) = self.identifier_string(at);
				Ok((
					next,
					if Self::number_like(text) {
						SemiValue::Number(text)
					} else if matches!(text, "inf" | "-inf" | "nan" | "true" | "false" | "null") {
						return Err(Error::BadIdentifier(at.0));
					} else {
						SemiValue::String(Cow::Borrowed(text))
					},
				))
			}
			_ => Err(Error::ExpectedCloseParen(at.0)),
		}
	}

	/// `string`
	fn string(&self, at: Pos) -> PResult<(Pos, Cow<'text, str>)> {
		let (next, value) = self.semi_value(at)?;
		match value {
			SemiValue::String(text) => Ok((next, text)),
			_ => Err(Error::ExpectedString(at.0)),
		}
	}
	/// `string | number | keyword`
	fn value(&self, at: Pos) -> PResult<(Pos, Value<'text>)> {
		let (next, value) = self.semi_value(at)?;
		Ok((next, match value {
			SemiValue::String(text) => Value::String(text),
			SemiValue::Number(text) => {
				Value::Number(text.parse().map_err(|_| Error::InvalidNumber(at.0))?)
			}
			SemiValue::Keyword("null") => Value::Null,
			SemiValue::Keyword("true") => Value::Bool(true),
			SemiValue::Keyword("false") => Value::Bool(false),
			SemiValue::Keyword("inf") => Value::Number(Number::from_f64(f64::INFINITY)),
			SemiValue::Keyword("-inf") => Value::Number(Number::from_f64(f64::NEG_INFINITY)),
			SemiValue::Keyword("nan") => Value::Number(Number::from_f64(f64::NAN)),
			SemiValue::Keyword(_) => return Err(Error::BadKeyword(at.0)),
		}))
	}
	/// `type?`
	fn type_hint(&self, at: Pos) -> PResult<(Pos, Option<Cow<'text, str>>)> {
		if self.top_char(at) == Some('(') {
			let at = self.node_space(at.offset_char('('), false)?;
			let (at, text) = self.string(at)?;
			let at = self.node_space(at, false)?;
			if self.top_char(at) == Some(')') {
				Ok((at.offset_char(')'), Some(text)))
			} else {
				Err(Error::ExpectedCloseParen(at.0))
			}
		} else {
			Ok((at, None))
		}
	}
	/// `line-space* eob` or `line-space* slashdash? type? node-space* string`
	fn start_node(&self, at: Pos, root: bool) -> PResult<(Pos, InnerEvent<'text>)> {
		let at = self.line_space(at)?;
		if root {
			if self.top_char(at).is_none() {
				return Ok((at, InnerEvent::Done));
			}
		} else if self.top_char(at) == Some('}') {
			return Ok((at.offset_char('}'), InnerEvent::End));
		}
		let next = self.slash_dash(at)?;
		let sd = next.is_some();
		let at = next.unwrap_or(at);
		let (at, r#type) = self.type_hint(at)?;
		let at = self.node_space(at, false)?;
		let (at, name) = self.string(at)?;
		Ok((at, InnerEvent::Node { sd, r#type, name }))
	}
	fn begin_document(&self, at: Pos) -> PResult<(Pos, InnerEvent<'text>)> {
		let at = self.bom(at).unwrap_or(at);
		self.start_node(at, true)
	}
	// TODO: in the next release of KDL, this syntax changes!
	// node-space* eob -> End
	// node-space* node-terminator start-node (if !root) -> Node
	// node-space+ slashdash? node-prop-or-arg (if props) -> PropValue
	// node-space+ slashdash? { -> Begin
	fn node_item(&self, at: Pos, root: bool, props: bool) -> PResult<(Pos, InnerEvent<'text>)> {
		let first = at;
		let at = self.node_space(at, false)?;
		if root && self.top_char(at).is_none() {
			return Ok((at, InnerEvent::Done));
		}
		match self.top_char(at) {
			Some('}') if !root => return Ok((at.offset_char('}'), InnerEvent::End)),
			// node-terminator
			Some(';') => return self.start_node(at.offset_char(';'), root),
			Some('/') if self.top_char(at.offset_char('/')) == Some('/') => {
				let at = self.single_line_comment(at.offset_str("//"))?;
				return self.start_node(at, root);
			}
			Some(ch) if Self::newline(ch) => return self.start_node(at.offset_char(ch), root),
			_ => {}
		}
		if at.0 == first.0 {
			return Err(Error::ExpectedSpace(at.0));
		}
		let next = self.slash_dash(at)?;
		let sd = next.is_some();
		let at = next.unwrap_or(at);
		if self.top_char(at) == Some('{') {
			Ok((at.offset_char('{'), InnerEvent::Begin { sd }))
		} else if props {
			// prop/value sucks to parse, the two valid options here are:
			// - type? node-space* value
			// - string node-space* = node-space* type? node-space* value
			// which we can parse as:
			// - type node-space* value
			// - value (node-space* was already consumed)
			// - string node-space* = node-space* type? node-space* value
			// that third one comes as a tail-check of the second, only consume the space if
			// it's used
			if let (at, Some(r#type)) = self.type_hint(at)? {
				let at = self.node_space(at, false)?;
				let (at, value) = self.value(at)?;
				Ok((at, InnerEvent::PropValue {
					sd,
					r#type: Some(r#type),
					key: None,
					value,
				}))
			} else {
				// this is a different at binding than type_hint, but it's the same value
				let (at, value) = self.value(at)?;
				// try for a property
				let value = match value {
					Value::String(key) => {
						let at = self.node_space(at, false)?;
						if self.top_char(at) == Some('=') {
							let at = self.node_space(at.offset_char('='), false)?;
							let (at, r#type) = self.type_hint(at)?;
							let at = self.node_space(at, false)?;
							let (at, real) = self.value(at)?;
							return Ok((at, InnerEvent::PropValue {
								sd,
								r#type,
								key: Some(key),
								value: real,
							}));
						}
						// fail and reuse value
						Value::String(key)
					}
					_ => value,
				};
				Ok((at, InnerEvent::PropValue {
					sd,
					r#type: None,
					key: None,
					value,
				}))
			}
		} else {
			Err(Error::ExpectedValue(at.0))
		}
	}
}

/// Actual number parsing implementation based on the streaming combinators
pub(crate) fn parse_number(text: &str) -> Result<Number, NumberError> {
	Grammar(text).all_number(Pos(0))
}

/// A streaming parser, is an [`Iterator`] of [`Event`]
pub struct Parser<'text> {
	grammar: Grammar<'text>,
	cursor: Pos,
	state: ParserState,
	// used in sd-filtering
	begin_valid: bool,
	// number of levels deep
	// used to determine if a } is still needed
	nest: usize,
}

impl<'text> Parser<'text> {
	/// Create a new parser from a text string
	pub fn new(text: &'text str) -> Self {
		Self {
			grammar: Grammar(text),
			cursor: Pos(0),
			state: ParserState::BeginDocument,
			begin_valid: false,
			nest: 0,
		}
	}
	fn next_event(&mut self) -> PResult<InnerEvent<'text>> {
		let event = match &mut self.state {
			ParserState::BeginDocument => {
				let (cursor, event) = self.grammar.begin_document(self.cursor)?;
				self.cursor = cursor;
				event
			}
			ParserState::NextNode => {
				let (cursor, event) = self.grammar.start_node(self.cursor, self.nest == 0)?;
				self.cursor = cursor;
				event
			}
			ParserState::NodeProps => {
				let (cursor, event) = self.grammar.node_item(self.cursor, self.nest == 0, true)?;
				self.cursor = cursor;
				event
			}
			// same as NodeProps without propvalue
			ParserState::NodeChildren => {
				let (cursor, event) = self.grammar.node_item(self.cursor, self.nest == 0, false)?;
				self.cursor = cursor;
				event
			}
			ParserState::Done => InnerEvent::Done,
		};
		self.state = match event {
			InnerEvent::Node { .. } | InnerEvent::PropValue { .. } => ParserState::NodeProps,
			InnerEvent::Begin { .. } => {
				self.nest += 1;
				ParserState::NextNode
			}
			InnerEvent::End => {
				self.nest -= 1;
				ParserState::NodeChildren
			}
			InnerEvent::Done => ParserState::Done,
		};
		Ok(event)
	}
	fn next_real(&mut self) -> PResult<Option<Event<'text>>> {
		// sd node -> pull until node/end/finish, consume end, then loop
		// sd value -> consume & loop
		let mut next_pop = None;
		Ok(Some(loop {
			// this position is only the real start if next_pop is none
			// used for diagnostics
			let start_cursor = self.cursor;
			break match next_pop.take().ok_or(()).or_else(|()| self.next_event())? {
				InnerEvent::Node { sd: true, .. } => {
					// continue until next node/end/finish
					// next_pop is always none here so we can just take events
					let mut depth = 0_usize;
					next_pop = loop {
						match self.next_event()? {
							node @ InnerEvent::Node { sd: false, .. } if depth == 0 => {
								break Some(node);
							}
							InnerEvent::Begin { .. } => depth += 1,
							InnerEvent::End => match depth.checked_sub(1) {
								Some(next) => {
									depth = next;
									if depth == 0 {
										break None;
									}
								}
								None => break Some(InnerEvent::End),
							},
							InnerEvent::Done => break Some(InnerEvent::Done),
							InnerEvent::Node { .. } | InnerEvent::PropValue { .. } => {}
						}
					};
					continue;
				}
				InnerEvent::Begin { sd: true } => {
					let mut depth = 0_usize;
					loop {
						match self.next_event()? {
							InnerEvent::Begin { .. } => depth += 1,
							InnerEvent::End => {
								if let Some(next) = depth.checked_sub(1) {
									depth = next;
								} else {
									break;
								}
							}
							_ => {}
						}
					}
					continue;
				}
				InnerEvent::Node {
					sd: false,
					r#type,
					name,
				} => {
					self.begin_valid = true;
					Event::Node { r#type, name }
				}
				InnerEvent::PropValue { sd: true, .. } => continue,
				InnerEvent::PropValue {
					sd: false,
					r#type,
					key,
					value,
				} => Event::Entry { r#type, key, value },
				InnerEvent::Begin { sd: false } => {
					if self.begin_valid {
						Event::Begin
					} else {
						return Err(Error::MultipleChildren(start_cursor.0));
					}
				}
				InnerEvent::End => {
					self.begin_valid = false;
					Event::End
				}
				InnerEvent::Done => return Ok(None),
			};
		}))
	}
}

impl<'text> Iterator for Parser<'text> {
	type Item = PResult<Event<'text>>;
	fn next(&mut self) -> Option<Self::Item> {
		let event = self.next_real();
		// this is a terrible place to put it but oh well
		if event.is_err() {
			self.state = ParserState::Done;
		}
		event.transpose()
	}
}

/// Write an iterator of events out as text, without constructing a
/// [`Document`] first
///
/// [`Document`]: crate::dom::Document
pub fn write_stream<'text, I: IntoIterator<Item = Event<'text>>>(
	f: &mut impl fmt::Write,
	events: I,
) -> fmt::Result {
	let mut depth = 0;
	let mut non_start = false;
	for event in events {
		match event {
			Event::Node { r#type, name } => {
				if non_start {
					f.write_str("\n")?;
				}
				non_start = true;
				for _ in 0..depth {
					f.write_str("    ")?;
				}
				if let Some(r#type) = &r#type {
					write!(f, "({})", IdentDisplay(r#type))?;
				}
				write!(f, "{}", IdentDisplay(&name))?;
			}
			Event::Entry { key, r#type, value } => {
				f.write_str(" ")?;
				if let Some(key) = &key {
					write!(f, "{}=", IdentDisplay(key))?;
				}
				if let Some(r#type) = &r#type {
					write!(f, "({})", IdentDisplay(r#type))?;
				}
				write!(f, "{value}")?;
			}
			Event::Begin => {
				f.write_str(" {")?;
				depth += 1;
			}
			Event::End => {
				f.write_str("\n")?;
				depth -= 1;
				for _ in 0..depth {
					f.write_str("    ")?;
				}
				f.write_str("}")?;
			}
		}
	}
	Ok(())
}
