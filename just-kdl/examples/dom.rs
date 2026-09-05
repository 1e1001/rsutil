//! Just debug print the document for testing
#![expect(clippy::print_stdout, reason = "binary")]

use std::io::stdin;

use just_kdl::dom::Document;
use just_kdl::lexer::ReadInput;
use just_kdl::reader::Reader;

fn main() {
	println!(
		"{:?}",
		Reader::new(ReadInput::new(stdin().lock())).collect::<Result<Document, _>>()
	);
}
