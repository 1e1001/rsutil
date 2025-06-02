// SPDX-License-Identifier: MIT OR Apache-2.0
//! cargo script :)
use std::fs::{read_dir, read_to_string};
use std::path::Path;

fn main() {
	for file in read_dir("kdl/tests/test_cases/input").unwrap() {
		let file = file.unwrap();
		println!(
			"test_case! {{ {},",
			file.path().file_stem().unwrap().to_str().unwrap()
		);
		println!("\t{:?},", read_to_string(file.path()).unwrap());
		match read_to_string(Path::new("kdl/tests/test_cases/expected_kdl/").join(file.file_name()))
		{
			Ok(text) => {
				println!("\tdom: Equal({text:?}),\n\tstream: Equal({text:?}),");
			}
			Err(_) => println!("\tdom: Panic,\n\tstream: Panic,"),
		}
		println!("}}");
	}
}
