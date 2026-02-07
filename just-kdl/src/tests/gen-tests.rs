// SPDX-License-Identifier: MIT OR Apache-2.0
//! rust-script to generate kdl tests
//! run in same dir as cloned kdl and pipe output to tests/spec.rs
use std::fs::{read_dir, read_to_string};
use std::path::Path;

fn main() {
	let tests = read_dir("kdl/tests/test_cases/input")
		.unwrap()
		.map(|file| {
			let file = file.unwrap();
			let name = file
				.path()
				.file_stem()
				.unwrap()
				.to_owned()
				.into_string()
				.unwrap();
			(
				name,
				read_to_string(file.path()).unwrap(),
				read_to_string(
					Path::new("kdl/tests/test_cases/expected_kdl/").join(file.file_name()),
				)
				.ok(),
			)
		})
		.collect::<Vec<_>>();
	println!("//! tests generated from kdl spec");
	for (name, _, _) in &tests {
		println!("#[test] pub fn {name}() {{ super::test_entry({name:?}); }}");
	}
	println!(
		"pub static TESTS: [(&str, &str, Option<&str>); {}] = [",
		tests.len()
	);
	for test in &tests {
		// TODO/style: consider writing test cases using raw strings for readability?
		println!("\t{test:?},");
	}
	println!("];");
}
