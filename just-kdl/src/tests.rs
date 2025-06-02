// SPDX-License-Identifier: MIT OR Apache-2.0
//! kdl spec conformance testing
use std::collections::HashSet;
use std::panic::{UnwindSafe, catch_unwind};

use crate::dom::Document;
use crate::stream::{Parser, write_stream};

fn run_test_ref(input: &str, output: Test) {
	fn normalize(document: &mut KdlDocument) {
		for node in document.nodes_mut() {
			let entries = node.entries_mut();
			let mut seen = HashSet::new();
			for i in (0..entries.len()).rev() {
				if let Some(name) = entries[i].name() {
					if !seen.insert(name.clone()) {
						entries.remove(i);
					}
				}
			}
			if let Some(doc) = node.children_mut() {
				if doc.nodes().is_empty() {
					node.clear_children();
				} else {
					normalize(doc);
				}
			}
		}
	}
	output.run("ref", || {
		let mut doc = KdlDocument::parse_v2(input).expect("Sub-test ref");
		normalize(&mut doc);
		doc.autoformat_no_comments();
		doc.to_string()
	});
}

fn run_test_stream(input: &str, output: Test) {
	output.run("stream", || {
		let mut out = String::new();
		write_stream(&mut out, Parser::new(input).map(Result::unwrap)).expect("Sub-test stream");
		out.push('\n');
		out
	});
}

fn run_test_dom(input: &str, output: Test) {
	output.run("dom", || {
		let mut doc = Document::parse(input).expect("Sub-test dom");
		for node in &mut doc.nodes {
			node.normalize();
		}
		format!("{doc}\n")
	});
}

enum Test {
	Panic,
	Equal(&'static str),
}

use Test::{Equal, Panic};
use kdl::KdlDocument;

impl Test {
	fn run(&self, label: &str, inner: impl FnOnce() -> String + UnwindSafe) {
		match self {
			Test::Panic => assert_eq!(catch_unwind(inner).ok(), None, "Sub-test {label}"),
			Test::Equal(output) => assert_eq!(&inner(), output, "Sub-test {label}"),
		}
	}
}

macro_rules! test_case {
	($(#[ignore $($ignore:lifetime)?])? $name:ident, $input:literal, ref: $ref:expr, dom: $dom:expr, stream: $stream:expr,) => {
		#[test]
		$(#[ignore $($ignore)?])?
		fn $name() {
			run_test_ref($input, $ref);
			run_test_dom($input, $dom);
			run_test_stream($input, $stream);
		}
	};
}

// my own test cases
test_case! { custom_hex_int,
	"node 0xABCDEF 0x0123456789 0xabcdef\n",
	ref: Equal("node 11259375 4886718345 11259375\n"),
	dom: Equal("node 11259375 4886718345 11259375\n"),
	stream: Equal("node 11259375 4886718345 11259375\n"),
}
// test cases from main
test_case! { braces_in_bare_id,
	// test doesn't match spec (space required before children block)
	"foo123{bar}\n",
	ref: Equal("foo123 {\n    bar\n}\n"), // Bad reference :)
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_raw_string_empty,
	"node #\"\"\"\n\"\"\"#",
	ref: Equal("node \"\"\n"),
	dom: Equal("node \"\"\n"),
	stream: Equal("node \"\"\n"),
}
test_case! { multiline_raw_string_empty_indented,
	"node #\"\"\"\n\t\"\"\"#",
	ref: Equal("node \"\"\n"),
	dom: Equal("node \"\"\n"),
	stream: Equal("node \"\"\n"),
}
test_case! { multiline_string_empty,
	"node \"\"\"\n\"\"\"",
	ref: Equal("node \"\"\n"),
	dom: Equal("node \"\"\n"),
	stream: Equal("node \"\"\n"),
}
test_case! { multiline_string_empty_indented,
	"node \"\"\"\n\t\"\"\"",
	ref: Equal("node \"\"\n"),
	dom: Equal("node \"\"\n"),
	stream: Equal("node \"\"\n"),
}
test_case! { multiline_string_wrapped_binary,
	"node \"\"\"\n    dead\\\n    beef\n    \"\"\"\n",
	ref: Equal("node deadbeef\n"),
	dom: Equal("node deadbeef\n"),
	stream: Equal("node deadbeef\n"),
}
test_case! { semicolon_missing_after_children_fail,
	"foo123{bar}foo weeee\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_above_max_fail,
	"no \"Higher than max Unicode Scalar Value \\u{10FFFF} \\u{11FFFF}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_h1_fail,
	"no \"Surrogates high\\u{D800}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_h2_fail,
	"no \"Surrogates high\\u{D911}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_h3_fail,
	"no \"Surrogates high\\u{DABB}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_h4_fail,
	"no \"Surrogates high\\u{DBFF}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_l1_fail,
	"no \"Surrogates low\\u{DC00}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_l2_fail,
	"no \"Surrogates low\\u{DEAD}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_l3_fail,
	"eno \"Surrogates low\\u{DFFF}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_escaped_too_long_lead0_fail,
	"no \"Even with leading 0s Unicode Scalar Value escapes must ≤6: \\u{0012345}\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { zero_space_before_slashdash_arg,
	// test doesn't match spec (space required before children block)
	"node \"string\"/-1\n",
	// Equal("node string\n")
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { zero_space_before_slashdash_children,
	// test doesn't match spec (space required before children block)
	"node \"string\"/-{}\nnode \"string\" {}/-{}\n",
	// Equal("node string\nnode string\n")
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { zero_space_before_slashdash_prop,
	// test doesn't match spec (space required before children block)
	"node \"string\"/-foo=1\n",
	// Equal("node string\n")
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
// tests from v2.0.0
test_case! { all_escapes,
	"node \"\\\"\\\\\\b\\f\\n\\r\\t\\s\"\n",
	ref: Equal("node \"\\\"\\\\\\b\\f\\n\\r\\t \"\n"),
	dom: Equal("node \"\\\"\\\\\\b\\f\\n\\r\\t \"\n"),
	stream: Equal("node \"\\\"\\\\\\b\\f\\n\\r\\t \"\n"),
}
test_case! { all_node_fields,
	"node arg prop=val {\n    inner_node\n}\n",
	ref: Equal("node arg prop=val {\n    inner_node\n}\n"),
	dom: Equal("node arg prop=val {\n    inner_node\n}\n"),
	stream: Equal("node arg prop=val {\n    inner_node\n}\n"),
}
test_case! { arg_and_prop_same_name,
	"node arg arg=val\n",
	ref: Equal("node arg arg=val\n"),
	dom: Equal("node arg arg=val\n"),
	stream: Equal("node arg arg=val\n"),
}
test_case! { arg_bare,
	"node a\n",
	ref: Equal("node a\n"),
	dom: Equal("node a\n"),
	stream: Equal("node a\n"),
}
test_case! { arg_false_type,
	"node (type)#false\n",
	ref: Equal("node (type)#false\n"),
	dom: Equal("node (type)#false\n"),
	stream: Equal("node (type)#false\n"),
}
test_case! { arg_float_type,
	"node (type)2.5",
	ref: Equal("node (type)2.5\n"),
	dom: Equal("node (type)2.5\n"),
	stream: Equal("node (type)2.5\n"),
}
test_case! { arg_hex_type,
	"node (type)0x10\n",
	ref: Equal("node (type)16\n"),
	dom: Equal("node (type)16\n"),
	stream: Equal("node (type)16\n"),
}
test_case! { arg_null_type,
	"node (type)#null\n",
	ref: Equal("node (type)#null\n"),
	dom: Equal("node (type)#null\n"),
	stream: Equal("node (type)#null\n"),
}
test_case! { arg_raw_string_type,
	"node (type)#\"str\"#\n",
	ref: Equal("node (type)str\n"),
	dom: Equal("node (type)str\n"),
	stream: Equal("node (type)str\n"),
}
test_case! { arg_string_type,
	"node (type)\"str\"\n",
	ref: Equal("node (type)str\n"),
	dom: Equal("node (type)str\n"),
	stream: Equal("node (type)str\n"),
}
test_case! { arg_true_type,
	"node (type)#true\n",
	ref: Equal("node (type)#true\n"),
	dom: Equal("node (type)#true\n"),
	stream: Equal("node (type)#true\n"),
}
test_case! { arg_type,
	"node (type)arg\n",
	ref: Equal("node (type)arg\n"),
	dom: Equal("node (type)arg\n"),
	stream: Equal("node (type)arg\n"),
}
test_case! { arg_zero_type,
	"node (type)0\n",
	ref: Equal("node (type)0\n"),
	dom: Equal("node (type)0\n"),
	stream: Equal("node (type)0\n"),
}
test_case! { asterisk_in_block_comment,
	"node /* * */",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { bare_emoji,
	"😁 happy!\n",
	ref: Equal("😁 happy!\n"),
	dom: Equal("😁 happy!\n"),
	stream: Equal("😁 happy!\n"),
}
test_case! { bare_ident_dot,
	"node .",
	ref: Equal("node .\n"),
	dom: Equal("node .\n"),
	stream: Equal("node .\n"),
}
test_case! { bare_ident_numeric_dot_fail,
	"node .0n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { bare_ident_numeric_fail,
	"node 0n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { bare_ident_numeric_sign_fail,
	"node +0n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { bare_ident_sign,
	"node +",
	ref: Equal("node +\n"),
	dom: Equal("node +\n"),
	stream: Equal("node +\n"),
}
test_case! { bare_ident_sign_dot,
	"node +.",
	ref: Equal("node +.\n"),
	dom: Equal("node +.\n"),
	stream: Equal("node +.\n"),
}
test_case! { binary,
	"node 0b10",
	ref: Equal("node 2\n"),
	dom: Equal("node 2\n"),
	stream: Equal("node 2\n"),
}
test_case! { binary_trailing_underscore,
	"node 0b10_",
	ref: Equal("node 2\n"),
	dom: Equal("node 2\n"),
	stream: Equal("node 2\n"),
}
test_case! { binary_underscore,
	"node 0b1_0\n",
	ref: Equal("node 2\n"),
	dom: Equal("node 2\n"),
	stream: Equal("node 2\n"),
}
test_case! { blank_arg_type,
	"node (\"\")10",
	ref: Equal("node (\"\")10\n"),
	dom: Equal("node (\"\")10\n"),
	stream: Equal("node (\"\")10\n"),
}
test_case! { blank_node_type,
	"(\"\")node\n",
	ref: Equal("(\"\")node\n"),
	dom: Equal("(\"\")node\n"),
	stream: Equal("(\"\")node\n"),
}
test_case! { blank_prop_type,
	"node key=(\"\")#true\n",
	ref: Equal("node key=(\"\")#true\n"),
	dom: Equal("node key=(\"\")#true\n"),
	stream: Equal("node key=(\"\")#true\n"),
}
test_case! { block_comment,
	"node /* comment */ arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { block_comment_after_node,
	"node /* hey */ arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { block_comment_before_node,
	"/* hey */ node",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { block_comment_before_node_no_space,
	"/* hey*/node\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { block_comment_newline,
	"/* hey */\n",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { bom_initial,
	"\u{feff}node arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { bom_later_fail,
	"node \u{feff}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { boolean_arg,
	"node #false #true\n",
	ref: Equal("node #false #true\n"),
	dom: Equal("node #false #true\n"),
	stream: Equal("node #false #true\n"),
}
test_case! { boolean_prop,
	"node prop1=#true prop2=#false\n",
	ref: Equal("node prop1=#true prop2=#false\n"),
	dom: Equal("node prop1=#true prop2=#false\n"),
	stream: Equal("node prop1=#true prop2=#false\n"),
}
test_case! { chevrons_in_bare_id,
	"foo123<bar>foo weeee\n",
	ref: Equal("foo123<bar>foo weeee\n"),
	dom: Equal("foo123<bar>foo weeee\n"),
	stream: Equal("foo123<bar>foo weeee\n"),
}
test_case! { comma_in_bare_id,
	"foo123,bar weeee\n",
	ref: Equal("foo123,bar weeee\n"),
	dom: Equal("foo123,bar weeee\n"),
	stream: Equal("foo123,bar weeee\n"),
}
test_case! { comment_after_arg_type,
	"node (type)/*hey*/10\n",
	ref: Equal("node (type)10\n"),
	dom: Equal("node (type)10\n"),
	stream: Equal("node (type)10\n"),
}
test_case! { comment_after_node_type,
	"(type)/*hey*/node\n",
	ref: Equal("(type)node\n"),
	dom: Equal("(type)node\n"),
	stream: Equal("(type)node\n"),
}
test_case! { comment_after_prop_type,
	"node key=(type)/*hey*/10\n",
	ref: Equal("node key=(type)10\n"),
	dom: Equal("node key=(type)10\n"),
	stream: Equal("node key=(type)10\n"),
}
test_case! { comment_and_newline,
	"node1 //\nnode2\n",
	ref: Equal("node1\nnode2\n"),
	dom: Equal("node1\nnode2\n"),
	stream: Equal("node1\nnode2\n"),
}
test_case! { comment_in_arg_type,
	"node (type/*hey*/)10\n",
	ref: Equal("node (type)10\n"),
	dom: Equal("node (type)10\n"),
	stream: Equal("node (type)10\n"),
}
test_case! { comment_in_node_type,
	"(type/*hey*/)node\n",
	ref: Equal("(type)node\n"),
	dom: Equal("(type)node\n"),
	stream: Equal("(type)node\n"),
}
test_case! { comment_in_prop_type,
	"node key=(type/*hey*/)10\n",
	ref: Equal("node key=(type)10\n"),
	dom: Equal("node key=(type)10\n"),
	stream: Equal("node key=(type)10\n"),
}
test_case! { commented_arg,
	"node /- arg1 arg2\n",
	ref: Equal("node arg2\n"),
	dom: Equal("node arg2\n"),
	stream: Equal("node arg2\n"),
}
test_case! { commented_child,
	"node arg /- {\n     inner_node\n}\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { commented_line,
	"// node_1\nnode_2",
	ref: Equal("node_2\n"),
	dom: Equal("node_2\n"),
	stream: Equal("node_2\n"),
}
test_case! { commented_node,
	"/- node_1\nnode_2\n/- node_3\n",
	ref: Equal("node_2\n"),
	dom: Equal("node_2\n"),
	stream: Equal("node_2\n"),
}
test_case! { commented_prop,
	"node /- prop=val arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { crlf_between_nodes,
	"node1\r\nnode2\r\n",
	ref: Equal("node1\nnode2\n"),
	dom: Equal("node1\nnode2\n"),
	stream: Equal("node1\nnode2\n"),
}
test_case! { dash_dash,
	"node --\n",
	ref: Equal("node --\n"),
	dom: Equal("node --\n"),
	stream: Equal("node --\n"),
}
test_case! { dot_but_no_fraction_before_exponent_fail,
	"node 1.e7",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { dot_but_no_fraction_fail,
	"node 1.",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { dot_in_exponent_fail,
	"node 1.0.0",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { dot_zero_fail,
	"node .0",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { emoji,
	"node 😀\n",
	ref: Equal("node 😀\n"),
	dom: Equal("node 😀\n"),
	stream: Equal("node 😀\n"),
}
test_case! { empty,
	"",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { empty_arg_type_fail,
	"node ()10\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { empty_child,
	"node {\n}",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node {\n}\n"), // no normalization
}
test_case! { empty_child_different_lines,
	"node {\n}",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node {\n}\n"), // no normalization
}
test_case! { empty_child_same_line,
	"node {}",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node {\n}\n"), // no normalization
}
test_case! { empty_child_whitespace,
	"node {\n\n     }",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node {\n}\n"), // no normalization
}
test_case! { empty_line_comment,
	"//\nnode",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { empty_node_type_fail,
	"()node\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { empty_prop_type_fail,
	"node key=()#false\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { empty_quoted_node_id,
	"\"\" arg\n",
	ref: Equal("\"\" arg\n"),
	dom: Equal("\"\" arg\n"),
	stream: Equal("\"\" arg\n"),
}
test_case! { empty_quoted_prop_key,
	"node \"\"=empty\n",
	ref: Equal("node \"\"=empty\n"),
	dom: Equal("node \"\"=empty\n"),
	stream: Equal("node \"\"=empty\n"),
}
test_case! { empty_string_arg,
	"node \"\"\n",
	ref: Equal("node \"\"\n"),
	dom: Equal("node \"\"\n"),
	stream: Equal("node \"\"\n"),
}
test_case! { eof_after_escape,
	"node \\",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { err_backslash_in_bare_id_fail,
	"foo123\\bar weeee\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { esc_multiple_newlines,
	"node \"1\\\n\n\n2\"\n",
	ref: Equal("node \"12\"\n"),
	dom: Equal("node \"12\"\n"),
	stream: Equal("node \"12\"\n"),
}
test_case! { esc_newline_in_string,
	"node \"hello\\nworld\"",
	ref: Equal("node \"hello\\nworld\"\n"),
	dom: Equal("node \"hello\\nworld\"\n"),
	stream: Equal("node \"hello\\nworld\"\n"),
}
test_case! { esc_unicode_in_string,
	"node \"hello\\u{0a}world\"\n",
	ref: Equal("node \"hello\\nworld\"\n"),
	dom: Equal("node \"hello\\nworld\"\n"),
	stream: Equal("node \"hello\\nworld\"\n"),
}
test_case! { escaped_whitespace,
	"// All of these strings are the same\nnode \\\n\t\"Hello\\n\\tWorld\" \\\n\t\"\"\"\n\tHello\n\t\tWorld\n\t\"\"\" \\\n\t\"Hello\\n\\      \\tWorld\" \\\n\t\"Hello\\n\\\n    \\tWorld\" \\\n\t\"Hello\\n\\t\\\n        World\"\n\n// Note that this file deliberately mixes space and newline indentation for\n// test purposes\n",
	ref: Equal("node \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\"\n"),
	dom: Equal("node \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\"\n"),
	stream: Equal("node \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\" \"Hello\\n\\tWorld\"\n"),
}
test_case! { escline,
	"node \\\n    arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { escline_after_semicolon,
	"node; \\\nnode\n",
	ref: Equal("node\nnode\n"),
	dom: Equal("node\nnode\n"),
	stream: Equal("node\nnode\n"),
}
test_case! { escline_alone,
	"\\\n",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { escline_empty_line,
	"\\\n\nnode\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { escline_end_of_node,
	"a \\\n\nb\n",
	ref: Equal("a\nb\n"),
	dom: Equal("a\nb\n"),
	stream: Equal("a\nb\n"),
}
test_case! { escline_in_child_block,
	"parent {\n    child\n    \\ // comment\n    child\n}\n",
	ref: Equal("parent {\n    child\n    child\n}\n"),
	dom: Equal("parent {\n    child\n    child\n}\n"),
	stream: Equal("parent {\n    child\n    child\n}\n"),
}
test_case! { escline_line_comment,
	"node \\   // comment\n    arg \\// comment\n    arg2\n",
	ref: Equal("node arg arg2\n"),
	dom: Equal("node arg arg2\n"),
	stream: Equal("node arg arg2\n"),
}
test_case! { escline_node,
	"node1\n\\\nnode2\n",
	ref: Equal("node1\nnode2\n"),
	dom: Equal("node1\nnode2\n"),
	stream: Equal("node1\nnode2\n"),
}
test_case! { escline_node_type,
	"\\\n(type)node\n",
	ref: Equal("(type)node\n"),
	dom: Equal("(type)node\n"),
	stream: Equal("(type)node\n"),
}
test_case! { escline_slashdash,
	"node\n\\\n/-\nnode\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { false_prefix_in_bare_id,
	"false_id\n",
	ref: Equal("false_id\n"),
	dom: Equal("false_id\n"),
	stream: Equal("false_id\n"),
}
test_case! { false_prefix_in_prop_key,
	"node false_id=1\n",
	ref: Equal("node false_id=1\n"),
	dom: Equal("node false_id=1\n"),
	stream: Equal("node false_id=1\n"),
}
test_case! { false_prop_key_fail,
	"node false=1\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { floating_point_keyword_identifier_strings_fail,
	"floats inf -inf nan\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { floating_point_keywords,
	"floats #inf #-inf #nan\n",
	ref: Equal("floats #inf #-inf #nan\n"),
	dom: Equal("floats #inf #-inf #nan\n"),
	stream: Equal("floats #inf #-inf #nan\n"),
}
test_case! { hash_in_id_fail,
	"foo#bar weee\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { hex,
	"node 0xabcdef1234567890",
	ref: Equal("node 12379813812177893520\n"),
	dom: Equal("node 12379813812177893520\n"),
	stream: Equal("node 12379813812177893520\n"),
}
test_case! { hex_int,
	// number representation bug
	"node 0xABCDEF0123456789abcdef\n",
	ref: Equal("node 207698809136909011942886895\n"),
	dom: Panic,
	stream: Panic,
}
test_case! { hex_int_underscores,
	"node 0xABC_def_0123",
	ref: Equal("node 737894400291\n"),
	dom: Equal("node 737894400291\n"),
	stream: Equal("node 737894400291\n"),
}
test_case! { hex_leading_zero,
	"node 0x01",
	ref: Equal("node 1\n"),
	dom: Equal("node 1\n"),
	stream: Equal("node 1\n"),
}
test_case! { illegal_char_in_binary_fail,
	"node 0bx01\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { illegal_char_in_hex_fail,
	"node 0x10g10",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { illegal_char_in_octal_fail,
	"node 0o45678",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { initial_slashdash,
	"/-node here\nanother-node\n",
	ref: Equal("another-node\n"),
	dom: Equal("another-node\n"),
	stream: Equal("another-node\n"),
}
test_case! { int_multiple_underscore,
	"node 1_2_3_4",
	ref: Equal("node 1234\n"),
	dom: Equal("node 1234\n"),
	stream: Equal("node 1234\n"),
}
test_case! { just_block_comment,
	"/* hey */",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { just_child,
	"node {\n    inner_node     \n}",
	ref: Equal("node {\n    inner_node\n}\n"),
	dom: Equal("node {\n    inner_node\n}\n"),
	stream: Equal("node {\n    inner_node\n}\n"),
}
test_case! { just_newline,
	"\n",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { just_node_id,
	"node",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { just_space,
	" ",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { just_space_in_arg_type_fail,
	"node ( )false\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { just_space_in_node_type_fail,
	"( )node\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { just_space_in_prop_type_fail,
	"node key=( )0x10\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { just_type_no_arg_fail,
	"node (type)\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { just_type_no_node_id_fail,
	"(type)\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { just_type_no_prop_fail,
	"node key=(type)\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { leading_newline,
	"\nnode",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { leading_zero_binary,
	"node 0b01\n",
	ref: Equal("node 1\n"),
	dom: Equal("node 1\n"),
	stream: Equal("node 1\n"),
}
test_case! { leading_zero_int,
	"node 011\n",
	ref: Equal("node 11\n"),
	dom: Equal("node 11\n"),
	stream: Equal("node 11\n"),
}
test_case! { leading_zero_oct,
	"node 0o01\n",
	ref: Equal("node 1\n"),
	dom: Equal("node 1\n"),
	stream: Equal("node 1\n"),
}
test_case! { legacy_raw_string_fail,
	"node r\"foo\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { legacy_raw_string_hash_fail,
	"node r#\"foo\"#\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_comment,
	"node /*\nsome\ncomments\n*/ arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { multiline_nodes,
	"node \\\n    arg1 \\// comment\n    arg2\n",
	ref: Equal("node arg1 arg2\n"),
	dom: Equal("node arg1 arg2\n"),
	stream: Equal("node arg1 arg2\n"),
}
test_case! { multiline_raw_string,
	"node #\"\"\"\nhey\neveryone\nhow goes?\n\"\"\"#\n",
	ref: Equal("node \"hey\\neveryone\\nhow goes?\"\n"),
	dom: Equal("node \"hey\\neveryone\\nhow goes?\"\n"),
	stream: Equal("node \"hey\\neveryone\\nhow goes?\"\n"),
}
test_case! { multiline_raw_string_containing_quotes,
	"node ##\"\"\"\n\"\"\"triple-quote\"\"\"\n##\"too few quotes\"##\n#\"\"\"too few #\"\"\"#\n\"\"\"##\n",
	ref: Equal("node \"\\\"\\\"\\\"triple-quote\\\"\\\"\\\"\\n##\\\"too few quotes\\\"##\\n#\\\"\\\"\\\"too few #\\\"\\\"\\\"#\"\n"),
	dom: Equal("node \"\\\"\\\"\\\"triple-quote\\\"\\\"\\\"\\n##\\\"too few quotes\\\"##\\n#\\\"\\\"\\\"too few #\\\"\\\"\\\"#\"\n"),
	stream: Equal("node \"\\\"\\\"\\\"triple-quote\\\"\\\"\\\"\\n##\\\"too few quotes\\\"##\\n#\\\"\\\"\\\"too few #\\\"\\\"\\\"#\"\n"),
}
test_case! { multiline_raw_string_indented,
	"node #\"\"\"\n    hey\n   everyone\n     how goes?\n  \"\"\"#\n",
	ref: Equal("node \"  hey\\n everyone\\n   how goes?\"\n"),
	dom: Equal("node \"  hey\\n everyone\\n   how goes?\"\n"),
	stream: Equal("node \"  hey\\n everyone\\n   how goes?\"\n"),
}
test_case! { multiline_raw_string_non_matching_prefix_character_error_fail,
	"node #\"\"\"\n    hey\n   everyone\n\t   how goes?\n  \"\"\"#\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_raw_string_non_matching_prefix_count_error_fail,
	"node #\"\"\"\n    hey\n everyone\n     how goes?\n  \"\"\"#\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_raw_string_single_line_err_fail,
	// bug in reference implementation
	"node #\"\"\"one line\"\"\"#",
	// Panic,
	ref: Equal("node \"\\\"\\\"one line\\\"\\\"\"\n"),
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_raw_string_single_quote_err_fail,
	"node #\"\nhey\neveryone\nhow goes?\n\"#\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_string,
	"node \"\"\"\nhey\neveryone\nhow goes?\n\"\"\"\n",
	ref: Equal("node \"hey\\neveryone\\nhow goes?\"\n"),
	dom: Equal("node \"hey\\neveryone\\nhow goes?\"\n"),
	stream: Equal("node \"hey\\neveryone\\nhow goes?\"\n"),
}
test_case! { multiline_string_containing_quotes,
	"node \"\"\"\nthis string contains \"quotes\", twice\"\"\n\"\"\"\n",
	ref: Equal("node \"this string contains \\\"quotes\\\", twice\\\"\\\"\"\n"),
	dom: Equal("node \"this string contains \\\"quotes\\\", twice\\\"\\\"\"\n"),
	stream: Equal("node \"this string contains \\\"quotes\\\", twice\\\"\\\"\"\n"),
}
test_case! { multiline_string_double_backslash,
	"node \"\"\"\na\\\\ b\na\\\\\\ b\n\"\"\"\n",
	ref: Equal("node \"a\\\\ b\\na\\\\b\"\n"),
	dom: Equal("node \"a\\\\ b\\na\\\\b\"\n"),
	stream: Equal("node \"a\\\\ b\\na\\\\b\"\n"),
}
test_case! { multiline_string_escape_delimiter,
	"node \"\"\"\n\\\"\"\"\n\"\"\"\n",
	ref: Equal("node \"\\\"\\\"\\\"\"\n"),
	dom: Equal("node \"\\\"\\\"\\\"\"\n"),
	stream: Equal("node \"\\\"\\\"\\\"\"\n"),
}
test_case! { multiline_string_escape_in_closing_line,
	"node \"\"\"\n  foo \\\nbar\n  baz\n  \\   \"\"\"\n",
	ref: Equal("node \"foo bar\\nbaz\"\n"),
	dom: Equal("node \"foo bar\\nbaz\"\n"),
	stream: Equal("node \"foo bar\\nbaz\"\n"),
}
test_case! { multiline_string_escape_in_closing_line_shallow,
	"node \"\"\"\n  foo \\\nbar\n  baz\n\\   \"\"\"\n",
	ref: Equal("node \"  foo bar\\n  baz\"\n"),
	dom: Equal("node \"  foo bar\\n  baz\"\n"),
	stream: Equal("node \"  foo bar\\n  baz\"\n"),
}
test_case! { multiline_string_escape_newline_at_end,
	"node \"\"\"\n    a\n   \\\n\"\"\"\n",
	ref: Equal("node \" a\"\n"),
	dom: Equal("node \" a\"\n"),
	stream: Equal("node \" a\"\n"),
}
test_case! { multiline_string_escape_newline_at_end_fail,
	"node \"\"\"\na\n   \\\n\"\"\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_string_final_whitespace_escape_fail,
	"node \"\"\"\n  foo\n  bar\\\n  \"\"\"",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_string_indented,
	"node \"\"\"\n    hey\n   everyone\n     how goes?\n  \"\"\"\n",
	ref: Equal("node \"  hey\\n everyone\\n   how goes?\"\n"),
	dom: Equal("node \"  hey\\n everyone\\n   how goes?\"\n"),
	stream: Equal("node \"  hey\\n everyone\\n   how goes?\"\n"),
}
test_case! { multiline_string_non_literal_prefix_fail,
	"node \"\"\"\n\\s escaped prefix\n  literal prefix\n  \"\"\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_string_non_matching_prefix_character_error_fail,
	"node \"\"\"\n    hey\n   everyone\n\t   how goes?\n  \"\"\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_string_non_matching_prefix_count_error_fail,
	"node \"\"\"\n    hey\n everyone\n     how goes?\n  \"\"\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_string_single_line_err_fail,
	"node \"\"\"one line\"\"\"",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_string_single_quote_err_fail,
	"node \"\nhey\neveryone\nhow goes?\n\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiline_string_whitespace_only,
	"// This file deliberately contains unusual whitespace\n// The first two strings are empty\nnode \"\"\"\n\u{2001} \t\"\"\" \"\"\"\n\u{2007}\t\u{a0}\\\n   \u{2009}\u{2000}        \n\u{2007}\t\u{a0}\"\"\" \"\"\"\n     \u{2009}\u{2000}                     \n\u{2007}\"\"\"\\\n    \\ // The next two strings contains only whitespace\n    \"\"\"\n \u{a0}\u{205f}    \n       \n \u{a0}\u{205f}   \\s \n \u{a0}\u{205f} \"\"\" #\"\"\"\n\u{a0}\u{205f}\u{2009}\u{2000}\n\n  \"\"\"#\n",
	ref: Equal("node \"\" \"\" \"\" \"\\n\\n    \" \"\\n\"\n"),
	dom: Equal("node \"\" \"\" \"\" \"\\n\\n    \" \"\\n\"\n"),
	stream: Equal("node \"\" \"\" \"\" \"\\n\\n    \" \"\\n\"\n"),
}
test_case! { multiple_dots_in_float_before_exponent_fail,
	"node 1.0.0e7",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiple_dots_in_float_fail,
	"node 1.0.0",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiple_es_in_float_fail,
	"node 1.0E10e10\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { multiple_x_in_hex_fail,
	"node 0xx10",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { negative_exponent,
	// wrong number repr
	"node 1.0e-10",
	// Equal("node 1.0E-10\n"),
	ref: Equal("node 1e-10\n"),
	dom: Equal("node 1e-10\n"),
	stream: Equal("node 1e-10\n"),
}
test_case! { negative_float,
	"node -1.0 key=-10.0",
	ref: Equal("node -1.0 key=-10.0\n"),
	dom: Equal("node -1.0 key=-10.0\n"),
	stream: Equal("node -1.0 key=-10.0\n"),
}
test_case! { negative_int,
	"node -10 prop=-15",
	ref: Equal("node -10 prop=-15\n"),
	dom: Equal("node -10 prop=-15\n"),
	stream: Equal("node -10 prop=-15\n"),
}
test_case! { nested_block_comment,
	"node /* hi /* there */ everyone */ arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { nested_children,
	"node1 {\n    node2 {\n        node\n    }\n}",
	ref: Equal("node1 {\n    node2 {\n        node\n    }\n}\n"),
	dom: Equal("node1 {\n    node2 {\n        node\n    }\n}\n"),
	stream: Equal("node1 {\n    node2 {\n        node\n    }\n}\n"),
}
test_case! { nested_comments,
	"node /*/* nested */*/ arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { nested_multiline_block_comment,
	"node /*\nhey /*\nhow's\n*/\n    it going\n    */ arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { newline_between_nodes,
	"node1\nnode2\n",
	ref: Equal("node1\nnode2\n"),
	dom: Equal("node1\nnode2\n"),
	stream: Equal("node1\nnode2\n"),
}
test_case! { newlines_in_block_comment,
	"node /* hey so\nI was thinking\nabout newts */ arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { no_decimal_exponent,
	// wrong number repr
	"node 1e10",
	// Equal("node 1.0E+10\n"),
	ref: Equal("node 10000000000.0\n"),
	dom: Equal("node 10000000000.0\n"),
	stream: Equal("node 10000000000.0\n"),
}
test_case! { no_digits_in_hex_fail,
	"node 0x",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { no_integer_digit_fail,
	"node .1",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { no_solidus_escape_fail,
	"node \"\\/\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { node_false,
	"node #false\n",
	ref: Equal("node #false\n"),
	dom: Equal("node #false\n"),
	stream: Equal("node #false\n"),
}
test_case! { node_true,
	"node #true\n",
	ref: Equal("node #true\n"),
	dom: Equal("node #true\n"),
	stream: Equal("node #true\n"),
}
test_case! { node_type,
	"(type)node",
	ref: Equal("(type)node\n"),
	dom: Equal("(type)node\n"),
	stream: Equal("(type)node\n"),
}
test_case! { null_arg,
	"node #null\n",
	ref: Equal("node #null\n"),
	dom: Equal("node #null\n"),
	stream: Equal("node #null\n"),
}
test_case! { null_prefix_in_bare_id,
	"null_id\n",
	ref: Equal("null_id\n"),
	dom: Equal("null_id\n"),
	stream: Equal("null_id\n"),
}
test_case! { null_prefix_in_prop_key,
	"node null_id=1\n",
	ref: Equal("node null_id=1\n"),
	dom: Equal("node null_id=1\n"),
	stream: Equal("node null_id=1\n"),
}
test_case! { null_prop,
	"node prop=#null\n",
	ref: Equal("node prop=#null\n"),
	dom: Equal("node prop=#null\n"),
	stream: Equal("node prop=#null\n"),
}
test_case! { null_prop_key_fail,
	"node null=1\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { numeric_arg,
	"node 15.7",
	ref: Equal("node 15.7\n"),
	dom: Equal("node 15.7\n"),
	stream: Equal("node 15.7\n"),
}
test_case! { numeric_prop,
	"node prop=10.0",
	ref: Equal("node prop=10.0\n"),
	dom: Equal("node prop=10.0\n"),
	stream: Equal("node prop=10.0\n"),
}
test_case! { octal,
	"node 0o76543210",
	ref: Equal("node 16434824\n"),
	dom: Equal("node 16434824\n"),
	stream: Equal("node 16434824\n"),
}
test_case! { only_cr,
	"\r",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { only_line_comment,
	"// hi",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { only_line_comment_crlf,
	"// comment\r\n",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { only_line_comment_newline,
	"// hiiii\n",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { optional_child_semicolon,
	"node {foo;bar;baz}\n",
	ref: Equal("node {\n    foo\n    bar\n    baz\n}\n"),
	dom: Equal("node {\n    foo\n    bar\n    baz\n}\n"),
	stream: Equal("node {\n    foo\n    bar\n    baz\n}\n"),
}
test_case! { parens_in_bare_id_fail,
	"foo123(bar)foo weeee\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { parse_all_arg_types,
	// wrong number repr
	"node 1 1.0 1.0e10 1.0e-10 0x01 0o07 0b10 arg \"arg\" #\"arg\\\"# #true #false #null\n",
	// Equal("node 1 1.0 1.0E+10 1.0E-10 1 7 2 arg arg \"arg\\\\\" #true #false #null\n")
	ref: Equal("node 1 1.0 10000000000.0 1e-10 1 7 2 arg arg \"arg\\\\\" #true #false #null\n"),
	dom: Equal("node 1 1.0 10000000000.0 1e-10 1 7 2 arg arg \"arg\\\\\" #true #false #null\n"),
	stream: Equal("node 1 1.0 10000000000.0 1e-10 1 7 2 arg arg \"arg\\\\\" #true #false #null\n"),
}
test_case! { positive_exponent,
	// wrong number repr
	"node 1.0e+10",
	// Equal("node 1.0E+10\n")
	ref: Equal("node 10000000000.0\n"),
	dom: Equal("node 10000000000.0\n"),
	stream: Equal("node 10000000000.0\n"),
}
test_case! { positive_int,
	"node +10",
	ref: Equal("node 10\n"),
	dom: Equal("node 10\n"),
	stream: Equal("node 10\n"),
}
test_case! { preserve_duplicate_nodes,
	"node\nnode\n",
	ref: Equal("node\nnode\n"),
	dom: Equal("node\nnode\n"),
	stream: Equal("node\nnode\n"),
}
test_case! { preserve_node_order,
	"node2\nnode5\nnode1",
	ref: Equal("node2\nnode5\nnode1\n"),
	dom: Equal("node2\nnode5\nnode1\n"),
	stream: Equal("node2\nnode5\nnode1\n"),
}
test_case! { prop_false_type,
	"node key=(type)#false\n",
	ref: Equal("node key=(type)#false\n"),
	dom: Equal("node key=(type)#false\n"),
	stream: Equal("node key=(type)#false\n"),
}
test_case! { prop_float_type,
	// wrong number repr
	"node key=(type)2.5E10\n",
	// Equal("node key=(type)2.5E+10\n")
	ref: Equal("node key=(type)25000000000.0\n"),
	dom: Equal("node key=(type)25000000000.0\n"),
	stream: Equal("node key=(type)25000000000.0\n"),
}
test_case! { prop_hex_type,
	"node key=(type)0x10\n",
	ref: Equal("node key=(type)16\n"),
	dom: Equal("node key=(type)16\n"),
	stream: Equal("node key=(type)16\n"),
}
test_case! { prop_identifier_type,
	"node key=(type)str\n",
	ref: Equal("node key=(type)str\n"),
	dom: Equal("node key=(type)str\n"),
	stream: Equal("node key=(type)str\n"),
}
test_case! { prop_null_type,
	"node key=(type)#null\n",
	ref: Equal("node key=(type)#null\n"),
	dom: Equal("node key=(type)#null\n"),
	stream: Equal("node key=(type)#null\n"),
}
test_case! { prop_raw_string_type,
	"node key=(type)#\"str\"#\n",
	ref: Equal("node key=(type)str\n"),
	dom: Equal("node key=(type)str\n"),
	stream: Equal("node key=(type)str\n"),
}
test_case! { prop_string_type,
	"node key=(type)\"str\"\n",
	ref: Equal("node key=(type)str\n"),
	dom: Equal("node key=(type)str\n"),
	stream: Equal("node key=(type)str\n"),
}
test_case! { prop_true_type,
	"node key=(type)#true\n",
	ref: Equal("node key=(type)#true\n"),
	dom: Equal("node key=(type)#true\n"),
	stream: Equal("node key=(type)#true\n"),
}
test_case! { prop_type,
	"node key=(type)#true\n",
	ref: Equal("node key=(type)#true\n"),
	dom: Equal("node key=(type)#true\n"),
	stream: Equal("node key=(type)#true\n"),
}
test_case! { prop_zero_type,
	"node key=(type)0\n",
	ref: Equal("node key=(type)0\n"),
	dom: Equal("node key=(type)0\n"),
	stream: Equal("node key=(type)0\n"),
}
test_case! { question_mark_before_number,
	"node ?15\n",
	ref: Equal("node ?15\n"),
	dom: Equal("node ?15\n"),
	stream: Equal("node ?15\n"),
}
test_case! { quote_in_bare_id_fail,
	"foo123\"bar weeee\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { quoted_arg_type,
	"node (\"type/\")10",
	ref: Equal("node (\"type/\")10\n"),
	dom: Equal("node (\"type/\")10\n"),
	stream: Equal("node (\"type/\")10\n"),
}
test_case! { quoted_node_name,
	"\"0node\"",
	ref: Equal("\"0node\"\n"),
	dom: Equal("\"0node\"\n"),
	stream: Equal("\"0node\"\n"),
}
test_case! { quoted_node_type,
	"(\"type/\")node\n",
	ref: Equal("(\"type/\")node\n"),
	dom: Equal("(\"type/\")node\n"),
	stream: Equal("(\"type/\")node\n"),
}
test_case! { quoted_numeric,
	"node prop=\"10.0\"",
	ref: Equal("node prop=\"10.0\"\n"),
	dom: Equal("node prop=\"10.0\"\n"),
	stream: Equal("node prop=\"10.0\"\n"),
}
test_case! { quoted_prop_name,
	"node \"0prop\"=val\n",
	ref: Equal("node \"0prop\"=val\n"),
	dom: Equal("node \"0prop\"=val\n"),
	stream: Equal("node \"0prop\"=val\n"),
}
test_case! { quoted_prop_type,
	"node key=(\"type/\")#true\n",
	ref: Equal("node key=(\"type/\")#true\n"),
	dom: Equal("node key=(\"type/\")#true\n"),
	stream: Equal("node key=(\"type/\")#true\n"),
}
test_case! { r_node,
	"r \"arg\"\n",
	ref: Equal("r arg\n"),
	dom: Equal("r arg\n"),
	stream: Equal("r arg\n"),
}
test_case! { raw_arg_type,
	"node (type)#true\n",
	ref: Equal("node (type)#true\n"),
	dom: Equal("node (type)#true\n"),
	stream: Equal("node (type)#true\n"),
}
test_case! { raw_node_name,
	"#\"\\node\"#\n",
	ref: Equal("\"\\\\node\"\n"),
	dom: Equal("\"\\\\node\"\n"),
	stream: Equal("\"\\\\node\"\n"),
}
test_case! { raw_node_type,
	"(type)node",
	ref: Equal("(type)node\n"),
	dom: Equal("(type)node\n"),
	stream: Equal("(type)node\n"),
}
test_case! { raw_prop_type,
	"node key=(type)#true\n",
	ref: Equal("node key=(type)#true\n"),
	dom: Equal("node key=(type)#true\n"),
	stream: Equal("node key=(type)#true\n"),
}
test_case! { raw_string_arg,
	"node_1 #\"\"arg\\n\"and #stuff\"#\nnode_2 ##\"#\"arg\\n\"#and #stuff\"##\n",
	ref: Equal("node_1 \"\\\"arg\\\\n\\\"and #stuff\"\nnode_2 \"#\\\"arg\\\\n\\\"#and #stuff\"\n"),
	dom: Equal("node_1 \"\\\"arg\\\\n\\\"and #stuff\"\nnode_2 \"#\\\"arg\\\\n\\\"#and #stuff\"\n"),
	stream: Equal("node_1 \"\\\"arg\\\\n\\\"and #stuff\"\nnode_2 \"#\\\"arg\\\\n\\\"#and #stuff\"\n"),
}
test_case! { raw_string_backslash,
	"node #\"\\n\"#\n",
	ref: Equal("node \"\\\\n\"\n"),
	dom: Equal("node \"\\\\n\"\n"),
	stream: Equal("node \"\\\\n\"\n"),
}
test_case! { raw_string_hash_no_esc,
	"node #\"#\"#\n",
	ref: Equal("node \"#\"\n"),
	dom: Equal("node \"#\"\n"),
	stream: Equal("node \"#\"\n"),
}
test_case! { raw_string_just_backslash,
	"node #\"\\\"#\n",
	ref: Equal("node \"\\\\\"\n"),
	dom: Equal("node \"\\\\\"\n"),
	stream: Equal("node \"\\\\\"\n"),
}
test_case! { raw_string_just_quote_fail,
	"// This fails because `\"\"\"` MUST be followed by a newline.\nnode #\"\"\"#\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { raw_string_multiple_hash,
	"node ###\"\"#\"##\"###\n",
	ref: Equal("node \"\\\"#\\\"##\"\n"),
	dom: Equal("node \"\\\"#\\\"##\"\n"),
	stream: Equal("node \"\\\"#\\\"##\"\n"),
}
test_case! { raw_string_newline,
	"node #\"\"\"\nhello\nworld\n\"\"\"#\n",
	ref: Equal("node \"hello\\nworld\"\n"),
	dom: Equal("node \"hello\\nworld\"\n"),
	stream: Equal("node \"hello\\nworld\"\n"),
}
test_case! { raw_string_prop,
	"node_1 prop=#\"\"arg#\"\\n\"#\nnode_2 prop=##\"#\"arg#\"#\\n\"##\n",
	ref: Equal("node_1 prop=\"\\\"arg#\\\"\\\\n\"\nnode_2 prop=\"#\\\"arg#\\\"#\\\\n\"\n"),
	dom: Equal("node_1 prop=\"\\\"arg#\\\"\\\\n\"\nnode_2 prop=\"#\\\"arg#\\\"#\\\\n\"\n"),
	stream: Equal("node_1 prop=\"\\\"arg#\\\"\\\\n\"\nnode_2 prop=\"#\\\"arg#\\\"#\\\\n\"\n"),
}
test_case! { raw_string_quote,
	"node #\"a\"b\"#\n",
	ref: Equal("node \"a\\\"b\"\n"),
	dom: Equal("node \"a\\\"b\"\n"),
	stream: Equal("node \"a\\\"b\"\n"),
}
test_case! { repeated_arg,
	"node arg arg\n",
	ref: Equal("node arg arg\n"),
	dom: Equal("node arg arg\n"),
	stream: Equal("node arg arg\n"),
}
test_case! { repeated_prop,
	"node prop=10 prop=11",
	ref: Equal("node prop=11\n"),
	dom: Equal("node prop=11\n"),
	stream: Equal("node prop=10 prop=11\n"), // no normalization
}
test_case! { same_name_nodes,
	"node\nnode\n",
	ref: Equal("node\nnode\n"),
	dom: Equal("node\nnode\n"),
	stream: Equal("node\nnode\n"),
}
test_case! { sci_notation_large,
	// number representation limit
	"node prop=1.23E+1000",
	// Equal("node prop=1.23E+1000\n")
	ref: Equal("node prop=#inf\n"),
	dom: Equal("node prop=#inf\n"),
	stream: Equal("node prop=#inf\n"),
}
test_case! { sci_notation_small,
	// number representation limit
	"node prop=1.23E-1000",
	// Equal("node prop=1.23E-1000\n")
	ref: Equal("node prop=0.0\n"),
	dom: Equal("node prop=0.0\n"),
	stream: Equal("node prop=0.0\n"),
}
test_case! { semicolon_after_child,
	"node {\n     childnode\n};\n",
	ref: Equal("node {\n    childnode\n}\n"),
	dom: Equal("node {\n    childnode\n}\n"),
	stream: Equal("node {\n    childnode\n}\n"),
}
test_case! { semicolon_in_child,
	"node1 {\n      node2;\n}",
	ref: Equal("node1 {\n    node2\n}\n"),
	dom: Equal("node1 {\n    node2\n}\n"),
	stream: Equal("node1 {\n    node2\n}\n"),
}
test_case! { semicolon_separated,
	"node1;node2",
	ref: Equal("node1\nnode2\n"),
	dom: Equal("node1\nnode2\n"),
	stream: Equal("node1\nnode2\n"),
}
test_case! { semicolon_separated_nodes,
	"node1; node2; ",
	ref: Equal("node1\nnode2\n"),
	dom: Equal("node1\nnode2\n"),
	stream: Equal("node1\nnode2\n"),
}
test_case! { semicolon_terminated,
	"node1;",
	ref: Equal("node1\n"),
	dom: Equal("node1\n"),
	stream: Equal("node1\n"),
}
test_case! { single_arg,
	"node arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { single_prop,
	"node prop=val\n",
	ref: Equal("node prop=val\n"),
	dom: Equal("node prop=val\n"),
	stream: Equal("node prop=val\n"),
}
test_case! { slash_in_bare_id_fail,
	"foo123/bar weeee\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_after_arg_type_fail,
	"node (ty)/-arg1 arg2\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_after_node_type_fail,
	"(ty)/-node\nother-node\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_after_prop_key_fail,
	"node key /- = value\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_after_prop_val_type_fail,
	"node key=(ty)/-val other-arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_after_type_fail,
	"node (type) /- arg1 arg2\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_arg_after_newline_esc,
	"node \\\n    /- arg arg2\n",
	ref: Equal("node arg2\n"),
	dom: Equal("node arg2\n"),
	stream: Equal("node arg2\n"),
}
test_case! { slashdash_arg_before_newline_esc,
	"node /-    \\\n    arg\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { slashdash_before_children_end_fail,
	"node {\n    child1\n    /-\n}\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_before_eof_fail,
	"node foo /-\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_before_prop_value_fail,
	"node key = /-val etc\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_before_semicolon_fail,
	"node foo /-;\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_between_child_blocks_fail,
	"node { one } /- { two } { three }\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_child,
	"node /- {\n    node2\n}\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { slashdash_child_block_before_entry_err_fail,
	"node /-{\n    child\n} foo {\n    bar\n}\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_empty_child,
	"node /- {\n}\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { slashdash_escline_before_arg_type,
	"node /-\\\n(ty)arg1 arg2\n",
	ref: Equal("node arg2\n"),
	dom: Equal("node arg2\n"),
	stream: Equal("node arg2\n"),
}
test_case! { slashdash_escline_before_children,
	"node arg1 /-\\\n{\n}\n",
	ref: Equal("node arg1\n"),
	dom: Equal("node arg1\n"),
	stream: Equal("node arg1\n"),
}
test_case! { slashdash_escline_before_node,
	"/-\\\nnode1\nnode2\n",
	ref: Equal("node2\n"),
	dom: Equal("node2\n"),
	stream: Equal("node2\n"),
}
test_case! { slashdash_false_node,
	"node foo /-\nnot-a-node bar\n",
	ref: Equal("node foo bar\n"),
	dom: Equal("node foo bar\n"),
	stream: Equal("node foo bar\n"),
}
test_case! { slashdash_full_node,
	"/- node 1.0 \"a\" b=\"\"\"\nb\n\"\"\"\n",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { slashdash_in_slashdash,
	"/- node1 /- 1.0\nnode2",
	ref: Equal("node2\n"),
	dom: Equal("node2\n"),
	stream: Equal("node2\n"),
}
test_case! { slashdash_inside_arg_type_fail,
	"node (/-bad)nope\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_inside_node_type_fail,
	"(/-ty)node\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { slashdash_multi_line_comment_entry,
	"node 1 /- /*\nmulti\nline\ncomment\nhere\n*/ 2 3\n",
	ref: Equal("node 1 3\n"),
	dom: Equal("node 1 3\n"),
	stream: Equal("node 1 3\n"),
}
test_case! { slashdash_multi_line_comment_inline,
	"node 1 /-/*two*/2 3\n",
	ref: Equal("node 1 3\n"),
	dom: Equal("node 1 3\n"),
	stream: Equal("node 1 3\n"),
}
test_case! { slashdash_multiple_child_blocks,
	"node foo /-{\n    one\n} \\\n/-{\n    two\n} {\n    three\n} /-{\n    four\n}\n",
	ref: Equal("node foo {\n    three\n}\n"),
	dom: Equal("node foo {\n    three\n}\n"),
	stream: Equal("node foo {\n    three\n}\n"),
}
test_case! { slashdash_negative_number,
	"node /--1.0 2.0",
	ref: Equal("node 2.0\n"),
	dom: Equal("node 2.0\n"),
	stream: Equal("node 2.0\n"),
}
test_case! { slashdash_newline_before_children,
	"node 1 2 /-\n{\n    child\n}\n",
	ref: Equal("node 1 2\n"),
	dom: Equal("node 1 2\n"),
	stream: Equal("node 1 2\n"),
}
test_case! { slashdash_newline_before_entry,
	"node 1 /-\n2 3\n",
	ref: Equal("node 1 3\n"),
	dom: Equal("node 1 3\n"),
	stream: Equal("node 1 3\n"),
}
test_case! { slashdash_newline_before_node,
	"/-\nnode 1 2 3\n",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { slashdash_node_in_child,
	"node1 {\n    /- node2\n}",
	ref: Equal("node1\n"),
	dom: Equal("node1\n"),
	stream: Equal("node1 {\n}\n"), // no normalization
}
test_case! { slashdash_node_with_child,
	"/- node {\n   node2\n}",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { slashdash_only_node,
	"/-node\n",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { slashdash_only_node_with_space,
	"/- node",
	ref: Equal("\n"),
	dom: Equal("\n"),
	stream: Equal("\n"),
}
test_case! { slashdash_prop,
	"node /- key=value arg\n",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { slashdash_raw_prop_key,
	"node /- key=value\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { slashdash_repeated_prop,
	"node arg=correct /- arg=wrong\n",
	ref: Equal("node arg=correct\n"),
	dom: Equal("node arg=correct\n"),
	stream: Equal("node arg=correct\n"),
}
test_case! { slashdash_single_line_comment_entry,
	"node 1 /- // stuff\n2 3\n",
	ref: Equal("node 1 3\n"),
	dom: Equal("node 1 3\n"),
	stream: Equal("node 1 3\n"),
}
test_case! { slashdash_single_line_comment_node,
	"/- // this is a comment\nnode1\nnode2\n",
	ref: Equal("node2\n"),
	dom: Equal("node2\n"),
	stream: Equal("node2\n"),
}
test_case! { space_after_arg_type,
	"node (type) 10\n",
	ref: Equal("node (type)10\n"),
	dom: Equal("node (type)10\n"),
	stream: Equal("node (type)10\n"),
}
test_case! { space_after_node_type,
	"(type) node\n",
	ref: Equal("(type)node\n"),
	dom: Equal("(type)node\n"),
	stream: Equal("(type)node\n"),
}
test_case! { space_after_prop_type,
	"node key=(type) #false\n",
	ref: Equal("node key=(type)#false\n"),
	dom: Equal("node key=(type)#false\n"),
	stream: Equal("node key=(type)#false\n"),
}
test_case! { space_around_prop_marker,
	"node foo = bar\n",
	ref: Equal("node foo=bar\n"),
	dom: Equal("node foo=bar\n"),
	stream: Equal("node foo=bar\n"),
}
test_case! { space_in_arg_type,
	"node (type )#false\n",
	ref: Equal("node (type)#false\n"),
	dom: Equal("node (type)#false\n"),
	stream: Equal("node (type)#false\n"),
}
test_case! { space_in_node_type,
	"( type)node\n",
	ref: Equal("(type)node\n"),
	dom: Equal("(type)node\n"),
	stream: Equal("(type)node\n"),
}
test_case! { space_in_prop_type,
	"node key=(type )#false\n",
	ref: Equal("node key=(type)#false\n"),
	dom: Equal("node key=(type)#false\n"),
	stream: Equal("node key=(type)#false\n"),
}
test_case! { square_bracket_in_bare_id_fail,
	"foo123[bar]foo weeee\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { string_arg,
	"node \"arg\"",
	ref: Equal("node arg\n"),
	dom: Equal("node arg\n"),
	stream: Equal("node arg\n"),
}
test_case! { string_escaped_literal_whitespace,
	"node \"Hello \\\nWorld \\          Stuff\"\n",
	ref: Equal("node \"Hello World Stuff\"\n"),
	dom: Equal("node \"Hello World Stuff\"\n"),
	stream: Equal("node \"Hello World Stuff\"\n"),
}
test_case! { string_prop,
	"node prop=\"val\"",
	ref: Equal("node prop=val\n"),
	dom: Equal("node prop=val\n"),
	stream: Equal("node prop=val\n"),
}
test_case! { tab_space,
	"node\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { trailing_crlf,
	"node\r\n",
	ref: Equal("node\n"),
	dom: Equal("node\n"),
	stream: Equal("node\n"),
}
test_case! { trailing_underscore_hex,
	"node 0x123abc_",
	ref: Equal("node 1194684\n"),
	dom: Equal("node 1194684\n"),
	stream: Equal("node 1194684\n"),
}
test_case! { trailing_underscore_octal,
	"node 0o123_\n",
	ref: Equal("node 83\n"),
	dom: Equal("node 83\n"),
	stream: Equal("node 83\n"),
}
test_case! { true_prefix_in_bare_id,
	"true_id\n",
	ref: Equal("true_id\n"),
	dom: Equal("true_id\n"),
	stream: Equal("true_id\n"),
}
test_case! { true_prefix_in_prop_key,
	"node true_id=1\n",
	ref: Equal("node true_id=1\n"),
	dom: Equal("node true_id=1\n"),
	stream: Equal("node true_id=1\n"),
}
test_case! { true_prop_key_fail,
	"node true=1\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { two_nodes,
	"node1\nnode2\n",
	ref: Equal("node1\nnode2\n"),
	dom: Equal("node1\nnode2\n"),
	stream: Equal("node1\nnode2\n"),
}
test_case! { type_before_prop_key_fail,
	"node (type)key=10\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unbalanced_raw_hashes_fail,
	"node ##\"foo\"#\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { underscore_at_start_of_fraction_fail,
	"node 1._7",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { underscore_at_start_of_hex_fail,
	"node 0x_10",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { underscore_before_number,
	"node _15\n",
	ref: Equal("node _15\n"),
	dom: Equal("node _15\n"),
	stream: Equal("node _15\n"),
}
test_case! { underscore_in_exponent,
	// wrong number repr
	"node 1.0e-10_0\n",
	// Equal("node 1.0E-100\n")
	ref: Equal("node 1e-100\n"),
	dom: Equal("node 1e-100\n"),
	stream: Equal("node 1e-100\n"),
}
test_case! { underscore_in_float,
	"node 1_1.0\n",
	ref: Equal("node 11.0\n"),
	dom: Equal("node 11.0\n"),
	stream: Equal("node 11.0\n"),
}
test_case! { underscore_in_fraction,
	"node 1.0_2",
	ref: Equal("node 1.02\n"),
	dom: Equal("node 1.02\n"),
	stream: Equal("node 1.02\n"),
}
test_case! { underscore_in_int,
	"node 1_0\n",
	ref: Equal("node 10\n"),
	dom: Equal("node 10\n"),
	stream: Equal("node 10\n"),
}
test_case! { underscore_in_octal,
	"node 0o012_3456_7",
	ref: Equal("node 342391\n"),
	dom: Equal("node 342391\n"),
	stream: Equal("node 342391\n"),
}
test_case! { unicode_delete_fail,
	"// 0x007F (Delete)\nnode1 \u{7f}arg\n",
	ref: Equal("node1 \u{7f}arg\n"), // Bad reference :)
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_fsi_fail,
	"// 0x2068\nnode1 \u{2068}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_lre_fail,
	"// 0x202A\nnode1 \u{202a}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_lri_fail,
	"// 0x2066\nnode1\u{2066}\u{7f}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_lrm_fail,
	"// 0x200E\nnode \u{200e}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_lro_fail,
	"// 0x202D\nnode \u{202d}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_pdf_fail,
	"// 0x202C\nnode \u{202c}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_pdi_fail,
	"// 0x2069\nnode \u{2069}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_rle_fail,
	"// 0x202B\nnode1 \u{202b}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_rli_fail,
	"// 0x2067\nnode1 \u{2067}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_rlm_fail,
	"// 0x200F\nnode \u{200f}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_rlo_fail,
	"// 0x202E\nnode \u{202e}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unicode_silly,
	"ノード\u{3000}お名前=ฅ^•ﻌ•^ฅ\n",
	ref: Equal("ノード お名前=ฅ^•ﻌ•^ฅ\n"),
	dom: Equal("ノード お名前=ฅ^•ﻌ•^ฅ\n"),
	stream: Equal("ノード お名前=ฅ^•ﻌ•^ฅ\n"),
}
test_case! { unicode_under_0x20_fail,
	"// 0x0019\nnode1 \u{19}arg\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unterminated_empty_node_fail,
	"node {\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { unusual_bare_id_chars_in_quoted_id,
	"\"foo123~!@$%^&*.:'|?+<>,`-_\" weeee\n",
	ref: Equal("foo123~!@$%^&*.:'|?+<>,`-_ weeee\n"),
	dom: Equal("foo123~!@$%^&*.:'|?+<>,`-_ weeee\n"),
	stream: Equal("foo123~!@$%^&*.:'|?+<>,`-_ weeee\n"),
}
test_case! { unusual_chars_in_bare_id,
	"foo123~!@$%^&*.:'|?+<>,`-_ weeee\n",
	ref: Equal("foo123~!@$%^&*.:'|?+<>,`-_ weeee\n"),
	dom: Equal("foo123~!@$%^&*.:'|?+<>,`-_ weeee\n"),
	stream: Equal("foo123~!@$%^&*.:'|?+<>,`-_ weeee\n"),
}
test_case! { zero_float,
	"node 0.0\n",
	ref: Equal("node 0.0\n"),
	dom: Equal("node 0.0\n"),
	stream: Equal("node 0.0\n"),
}
test_case! { zero_int,
	"node 0\n",
	ref: Equal("node 0\n"),
	dom: Equal("node 0\n"),
	stream: Equal("node 0\n"),
}
test_case! { zero_space_before_first_arg_fail,
	"node\"string\"\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { zero_space_before_prop_fail,
	"node foo=\"value\"bar=5\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { zero_space_before_second_arg_fail,
	"node \"string\"1\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { brackets_in_bare_id_fail,
	"foo123{bar}foo weeee\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
test_case! { vertical_tab_whitespace,
	// test doesn't match spec (\u{b} is a newline)
	"node\u{b}arg\n",
	// Equal("node arg\n")
	ref: Equal("node\narg\n"),
	dom: Equal("node\narg\n"),
	stream: Equal("node\narg\n"),
}
test_case! { zero_space_before_slashdash_arg_fail,
	"node \"string\"/-1\n",
	ref: Panic,
	dom: Panic,
	stream: Panic,
}
