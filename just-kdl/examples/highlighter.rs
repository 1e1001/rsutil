//! Basic demo of the lexer, reads from stdin and prints with ANSI colors
#![expect(clippy::print_stdout, reason = "binary")]

use std::io::{Read, stdin};

use just_kdl::lexer::{Lexer, Token};

fn main() {
	let mut file = Vec::new();
	// can't stream to lexer since we need to reference the text for printing
	stdin()
		.read_to_end(&mut file)
		.expect("failed to read input");
	let mut lexer = Lexer::new(&*file);
	let mut prev_token = None;
	loop {
		let token = lexer.next_token(true);
		let eof = matches!(token, (Ok(Token::Eof), _));
		let end = token.1;
		if let Some(prev) = prev_token.replace(token) {
			let span = prev.1..end;
			// in a real world use you'd probably extend this with syntax-aware highlighting
			// (to show types or node names specially)
			let color = match prev.0 {
				Ok(Token::Bom | Token::Eof | Token::Lines | Token::Spaces | Token::SlashDash) => {
					"0"
				}
				Ok(Token::SkippedString) => match file.get(span.start).copied().unwrap_or_default()
				{
					b'"' => "34",
					b'#' => "35",
					_ => "36",
				},
				Ok(Token::SkippedNumber) => match file.get(span.start).copied().unwrap_or_default()
				{
					b'#' => "33",
					_ => "32",
				},
				Ok(
					Token::SemiColon
					| Token::Equals
					| Token::OpenParen
					| Token::CloseParen
					| Token::OpenCurly
					| Token::CloseCurly,
				) => "35",
				Ok(Token::Bool(_) | Token::Null) => "33",
				Ok(_) => unimplemented!(),
				Err(_) => "31",
			};
			print!("\x1b[{color}m");
			for chunk in file[span].utf8_chunks() {
				print!("{}", chunk.valid());
				for byte in chunk.invalid() {
					print!("\x1b[31m{byte:02x}\x1b[0m");
				}
			}
		}
		if eof {
			break;
		}
	}
}
