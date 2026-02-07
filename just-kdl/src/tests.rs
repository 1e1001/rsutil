// SPDX-License-Identifier: MIT OR Apache-2.0
#[rustfmt::skip]
mod spec;

/* TODO: fuzzing
	comparing kdl-rs/kdl-js & my kdl
	round-trip ability (bytes -> read(1) -> write -> read(2) leads to 1 == 2)
	lexer must always produce a complete stream (never panic)
	reader must always produce a stream or Err (never panic)
	dom round-trip (events -> document -> events), except spans
code coverage things
*/

fn test_info(name: &str) -> (&'static str, Option<&'static str>) {
	if let Ok(index) = spec::TESTS.binary_search_by_key(&name, |&(name, _, _)| name) {
		let (_, input, output) = spec::TESTS[index];
		return (input, output);
	}
	panic!("invalid test {name}");
}

fn test_entry(name: &'static str) { dom::test(name); }

mod dom {
	use std::mem::replace;

	use crate::dom::{Document, Node, Value};
	use crate::reader::Reader;
	use crate::tests::test_info;
	use crate::validator::Validator;
	use crate::writer::Writer;

	// convert numbers to decimal
	fn normalize2(node: &mut Node) {
		for entry in &mut node.entries {
			if let Value::Number(_) = &entry.value {
				let Value::Number(n) = replace(&mut entry.value, Value::Null) else {
					unreachable!();
				};
				let n = i128::try_from(n.clone()).ok().map_or(n, Into::into);
				entry.value = Value::Number(n);
			}
		}
		if let Some(children) = &mut node.children {
			for child in &mut children.nodes {
				normalize2(child);
			}
		}
	}
	#[expect(clippy::print_stderr, reason = "tests binary")]
	pub fn test(name: &'static str) {
		fn test_inner(input: &str, strict: bool) -> Option<String> {
			let mut validator = Validator::new();
			let mut document = Reader::new(input.as_bytes())
				.inspect(|event| {
					if let Ok((event, span)) = event {
						eprintln!("Event: {event:?} {span:?}");
						if let Err(err) = validator.push(event) {
							if strict {
								panic!("Validation: {err:?}");
							} else {
								eprintln!("Validation: {err:?}");
							}
						}
					}
				})
				.collect::<Result<Document, _>>()
				.inspect_err(|err| eprintln!("Error: {err:?}"))
				.ok()?;
			document.normalize();
			for node in &mut document.nodes {
				normalize2(node);
			}
			let display = document.to_string() + "\n";
			let mut written = String::new();
			let mut writer = Writer::new(&mut written);
			let mut post_validator = Validator::new();
			for event in document {
				if let Err(err) = post_validator.push(&event) {
					if strict {
						panic!("Validation: {err:?}");
					} else {
						eprintln!("Validation: {err:?}");
					}
				}
				writer.push(event).unwrap();
			}
			written.push('\n');
			assert_eq!(display, written, "Writer mismatch");
			Some(display)
		}
		let (input, output) = test_info(name);
		assert_eq!(
			test_inner(input, output.is_some()).as_deref(),
			output,
			"test body failed"
		);
	}
}
