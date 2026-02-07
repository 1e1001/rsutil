// SPDX-License-Identifier: MIT OR Apache-2.0
use crate::lexer::Token::{
	Bom, Bool, CloseCurly, CloseParen, Equals, Lines, Null, OpenCurly, OpenParen, SemiColon,
	SkippedNumber, SkippedString, SlashDash, Spaces,
};
use crate::lexer::{Input, Lexer, LexerError, Token};

fn skip_rewrite(token: Token) -> Token {
	match token {
		Token::String(_) => SkippedString,
		Token::Number(_) => SkippedNumber,
		token => token,
	}
}

fn lexer_test(
	text: impl AsRef<[u8]>,
	tokens: &[Result<Token, LexerError>],
	tokens_skip: &[Result<Token, LexerError>],
) {
	fn lexer_tokens<T: Input>(mut lexer: Lexer<T>, skip: bool) -> Vec<Result<Token, LexerError>> {
		let mut out = Vec::new();
		loop {
			let next = lexer.next_token(skip).0;
			if let Ok(Token::Eof) = next {
				break;
			}
			out.push(next);
		}
		out
	}
	let text = text.as_ref();
	let lexer1 = Lexer::new(text);
	assert_eq!(lexer_tokens(lexer1, false), tokens, "wrong result (normal)");
	let lexer2 = Lexer::new(text);
	assert_eq!(
		lexer_tokens(lexer2, true),
		tokens_skip,
		"wrong result (skip)"
	);
	#[cfg(feature = "std")]
	#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
	{
		use crate::lexer::ReadInput;
		let lexer3 = Lexer::new(ReadInput::new(text));
		assert_eq!(
			lexer_tokens(lexer3, false),
			tokens,
			"wrong result (normal read)"
		);
		let lexer4 = Lexer::new(ReadInput::new(text));
		assert_eq!(
			lexer_tokens(lexer4, true),
			tokens_skip,
			"wrong result (skip read)"
		);
	}
}

/* TODO: possible kdl reference/js bugs

spec typo at 3.12.2.3 "indented a bit" missing dot
spec typo(?) at 3.12.1 different names

test[FEFF] (should be invalid)

test """
\\""" for varying amounts of \'s and spaces (should be invalid except for one backslash and >0 spaces)

*/

trait TestMatchToken {
	fn tmt_into(self) -> Result<Token, LexerError>;
}

impl TestMatchToken for LexerError {
	fn tmt_into(self) -> Result<Token, LexerError> { Err(self) }
}
impl TestMatchToken for Token {
	fn tmt_into(self) -> Result<Token, LexerError> { Ok(self) }
}

macro_rules! tests {
	($($(#[$meta:meta])* $name:ident: $text:literal $(=> $($token:expr),* $(,)?)?;)*) => {
		$(
			$(#[$meta])* #[test]
			fn $name() {
				lexer_test($text, &[$($($token.tmt_into()),*)?], &[$($($token.tmt_into().map(skip_rewrite)),*)?])
			}
		)*
	}
}

#[expect(non_snake_case, reason = "token replacement")]
fn String(text: &str) -> Token { Token::String(text.into()) }
#[expect(non_snake_case, reason = "token replacement")]
fn Number(text: &str) -> Token { Token::Number(text.parse().unwrap()) }

tests! {
	ops_simple: "\u{FEFF}/-;=(){}" => Bom, SlashDash, SemiColon, Equals, OpenParen, CloseParen, OpenCurly, CloseCurly;
	late_bom: "}\u{FEFF}" => CloseCurly, LexerError::InvalidCharacter(0);
	spaces1: "\t; ;\u{A0};\u{1680};\u{202F};\u{205F};\u{3000};"
	=> Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon;
	spaces2: "\u{2000};\u{2001};\u{2002};\u{2003};\u{2004};\u{2005};\u{2006};\u{2007};\u{2008};\u{2009};\u{200A};"
	=> Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon, Spaces, SemiColon;
	lines: "\x0A;\x0B;\x0C;\x0D;\u{2028};\u{2029};"
	=> Lines, SemiColon, Lines, SemiColon, Lines, SemiColon, Lines, SemiColon, Lines, SemiColon, Lines, SemiColon;
	line_comment: r"// test
" => Lines;
	block_comment: "/* test with /* insides */; */;" => Spaces, SemiColon;
	esclines: r";\
;\   // test

;" => SemiColon, Spaces, SemiColon, Spaces, Lines, SemiColon;
	space_then_line: "  \n;\n\n;" => Spaces, Lines, SemiColon, Lines, SemiColon;
	op_keywords: "#true#false#null#inf#-inf#nantest;" => Bool(true), Bool(false), Null, Number("#inf"), Number("#-inf"), Number("#nan"), String("test"), SemiColon;
	ident: "test #trueue False; -1;" => String("test"), Spaces, Bool(true), String("ue"), Spaces, String("False"), SemiColon, Spaces, Number("-1"), SemiColon;
	ident_utf8: "aλ…𐀀;" => String("aλ…𐀀"), SemiColon;
	ident_numbers1: "ab -cd +ef .gh -.ij +.kl .-mn .+op" => String("ab"), Spaces, String("-cd"), Spaces, String("+ef"), Spaces, String(".gh"), Spaces, String("-.ij"), Spaces, String("+.kl"), Spaces, String(".-mn"), Spaces, String(".+op");
	ident_numbers2: "0 -1 +2 0__.3__ -0.4 +0.5 .-6 .+7 +-8" => Number("0"), Spaces, Number("-1"), Spaces, Number("+2"), Spaces, Number("0.3"), Spaces, Number("-0.4"), Spaces, Number("+0.5"), Spaces, String(".-6"), Spaces, String(".+7"), Spaces, String("+-8");
	ident_numbers3: "0x1f 0b001001 0o177" => Number("0x1f"), Spaces, Number("0b001001"), Spaces, Number("0o177");
	invalid_number1: ".3" => LexerError::InvalidNumber, Number("3");
	invalid_number2: "+.4" => LexerError::InvalidNumber, Number("4");
	invalid_number3: "-1a" => LexerError::InvalidNumber, String("a");
	invalid_number4: "0x" => LexerError::InvalidNumber;
	string: r#""te\nxt";"# => String("te\nxt"), SemiColon;
	string_escapes: r#""\"\\\b\f\n\r\t\
\s\u{0}\u{10ffff}";"# => String("\"\\\x08\x0C\n\r\t \x00\u{10ffff}"), SemiColon;
	string_multiline: r#""""
    te\nxt

  """;"# => String("  te\nxt\n"), SemiColon;
	string_raw: r##"#"te\xt"#;"## => String("te\\xt"), SemiColon;
	string_raw_multiline: r##"#"""
    te\xt

  """#;"## => String("  te\\xt\n"), SemiColon;

	banned1: "\u{200E}" => LexerError::InvalidCharacter(0);
	banned2: "// \u{FEFF}\nrecovery" => Lines, LexerError::InvalidCharacter(0), String("recovery");
	banned3: "/* /* \u{FEFF} */ */ recovery" => Spaces, LexerError::InvalidCharacter(0), Spaces, String("recovery");
	op_bad: "[" => LexerError::InvalidOperator;
	bad_slash: "/?" => LexerError::InvalidOperator, String("?");
	bad_block: "/* /* */ /* */ ** /* *" => Spaces, LexerError::UnexpectedEof(0);
	banned_idents: "nan" => LexerError::UnexpectedKeyword;
	invalid_utf8_regular: b" \x80" => Spaces, LexerError::InvalidUtf8(0);
	invalid_utf8_ident: b"a\xF0\x90\x80a" => LexerError::InvalidUtf8(0), LexerError::InvalidUtf8(0), LexerError::InvalidUtf8(0), String("a");
	invalid_utf8_line_comment: b"// \x80\nrecovery" => Lines, LexerError::InvalidUtf8(0), String("recovery");
	invalid_utf8_block_comment: b"/* /* \x80 */ */recovery" => Spaces, LexerError::InvalidUtf8(0), String("recovery");
	invalid_utf8_string: b"\"\x80\" recovery" => LexerError::InvalidUtf8(0), Spaces, String("recovery");
	escline_bad1: r"a\b" => String("a"), Spaces, LexerError::BadEscline(0), String("b");

	// kdl spec string examples
	spec_singleline1: r#""Hello World""# => String("Hello World");
	spec_singleline2: r#""Hello \    World""# => String("Hello World");
	spec_singleline3: r#""Hello\       \nWorld""# => String("Hello\nWorld");
	spec_singleline4: r#"    "Hello\n\
	World""# => Spaces, String("Hello\nWorld");
	spec_singleline5: r#""Hello\nWorld""# => String("Hello\nWorld");
	// it's in the basic strings section of the spec…
	spec_multiline10: r#""""
  Hello
  World
  """"# => String("Hello\nWorld");
	spec_multiline11: "multi-line \"\"\"
    \\r\\n\r
    foo\r
    \"\"\"" => String("multi-line"), Spaces, String("\r\n\nfoo");
	spec_multiline1: r#"multi-line """
        foo
    This is the base indentation
            bar
    """"# => String("multi-line"), Spaces, String("    foo\nThis is the base indentation\n        bar");
	spec_multiline2: r#"multi-line """
        foo
    This is no longer on the left edge
            bar
  """"# => String("multi-line"), Spaces, String("      foo\n  This is no longer on the left edge\n          bar");
	spec_multiline3: r#"multi-line """
    Indented a bit.

    A second indented paragraph.
    """"# => String("multi-line"), Spaces, String("Indented a bit.\n\nA second indented paragraph.");
	spec_multiline4: r#"multi-line """can't be single line""" recovery"# => String("multi-line"), Spaces, LexerError::MissingStringNewline, Spaces, String("recovery");
	spec_multiline5: r#"multi-line """
  closing quote with non-whitespace prefix""""# => String("multi-line"), Spaces, LexerError::BadEndString(0);
	spec_multiline6: r#"multi-line """stuff
  """ recovery"# => String("multi-line"), Spaces, LexerError::MissingStringNewline, Spaces, String("recovery");
	spec_multiline7: "multi-line \"\"\"
\ta
  b
 \t
\t\"\"\"" => String("multi-line"), Spaces, LexerError::BadIndent(None);
	spec_multiline8: r#"  """
  foo
  bar\
  """"# => Spaces, LexerError::BadEndString(0);
	spec_multiline9: r#"  """
  foo \
bar
  baz
  \   """"# => Spaces, String("foo bar\nbaz");
	spec_raw1: r##"just-escapes #"\n will be literal"#"## => String("just-escapes"), Spaces, String("\\n will be literal");
	spec_raw2: r###"quotes-and-escapes ##"hello\n\r\asd"#world"##"### => String("quotes-and-escapes"), Spaces, String("hello\\n\\r\\asd\"#world");
	spec_raw2_bad: r###"quotes-and-escapes ##"hello\n\r\asd
"#world"## recovery"### => String("quotes-and-escapes"), Spaces, LexerError::UnexpectedStringNewline(0), Spaces, String("recovery");
	spec_raw3: r##"raw-multi-line #"""
    Here's a """
        multiline string
        """
    without escapes.
    """#"## => String("raw-multi-line"), Spaces, String("Here's a \"\"\"\n    multiline string\n    \"\"\"\nwithout escapes.");
	js_weird1: "test\u{FEFF}recovery" => String("test"), LexerError::InvalidCharacter(0), String("recovery");
	js_weird2: r#"test """

\\""""# => String("test"), Spaces, LexerError::BadEndString(0);
}
