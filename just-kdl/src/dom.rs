// SPDX-License-Identifier: MIT OR Apache-2.0
//! Document tree structures.
//!
//! You probably want to start at [`Document`].
//!
//! [`Display`] implementations are equivalent to serialization.
//!
//! [`Display`]: fmt::Display

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::Cell;
use core::convert::Infallible;
use core::fmt;
use core::ops::{Index, IndexMut};
use core::ptr::eq as ptr_eq;

use hashbrown::HashSet;
use smol_str::SmolStr;

use crate::IdentDisplay;

pub mod iter;
pub mod number;

/// debug an `Option<T>` as just `T` or `None`
fn option_debug<T: fmt::Debug>(value: Option<&T>) -> &dyn fmt::Debug {
	match value {
		Some(value) => value,
		None => &None::<Infallible>,
	}
}

/// A `document` or `nodes` element, an ordered collection of nodes.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct Document {
	/// The ordered nodes in this document.
	pub nodes: Vec<Node>,
}

impl Document {
	/// Create a document with no children.
	pub const fn new() -> Self { Self { nodes: Vec::new() } }
	/// Iterator over every node with a particular name.
	pub fn get(&self, name: &str) -> impl Iterator<Item = &Node> {
		self.nodes.iter().filter(move |node| node.name() == name)
	}
	/// Mutable iterator over every node with a particular name.
	pub fn get_mut(&mut self, name: &str) -> impl Iterator<Item = &mut Node> {
		self.nodes
			.iter_mut()
			.filter(move |node| node.name() == name)
	}
	/// Normalize document to kdl spec by [`normalize`]-ing child nodes.
	///
	/// [`normalize`]: Node::normalize
	pub fn normalize(&mut self) {
		for node in &mut self.nodes {
			node.normalize();
		}
	}
}

impl fmt::Debug for Document {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str("Document ")?;
		f.debug_list().entries(&self.nodes).finish()
	}
}
impl fmt::Display for Document {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		let mut iter = self.nodes.iter();
		if let Some(first) = iter.next() {
			write!(f, "{first}")?;
			for node in iter {
				write!(f, "\n{node}")?;
			}
		}
		Ok(())
	}
}

impl From<Vec<Node>> for Document {
	fn from(nodes: Vec<Node>) -> Self { Self { nodes } }
}
impl From<Document> for Vec<Node> {
	fn from(value: Document) -> Self { value.nodes }
}

/// A `node` element, a collection of entries with optional children.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Node {
	/// The node's type hint.
	pub r#type: Option<SmolStr>,
	/// The node's name.
	pub name: SmolStr,
	// TODO: consider splitting args / props (makes args indexing constant)
	// official kdl doesn't do this, and it breaks Events → Document → Events parity
	/// The node's entries in order.
	pub entries: Vec<Entry>,
	/// The node's child document.
	pub children: Option<Document>,
}

/// The default node is `-`
impl Default for Node {
	fn default() -> Self {
		Self {
			r#type: None,
			name: SmolStr::new_inline("-"),
			entries: Vec::new(),
			children: None,
		}
	}
}

impl Node {
	/// Create a new node with a name.
	pub fn new<I: Into<SmolStr>>(name: I) -> Self {
		Self {
			r#type: None,
			name: name.into(),
			entries: Vec::new(),
			children: None,
		}
	}
	/// Get the node's name.
	pub fn name(&self) -> &str { &self.name }
	/// Set the node's name.
	pub fn set_name<T: Into<SmolStr>>(&mut self, name: T) { self.name = name.into(); }
	/// Get the node's type hint.
	pub fn type_hint(&self) -> Option<&str> { self.r#type.as_deref() }
	/// Set the node's type hint.
	pub fn set_type_hint<T: Into<SmolStr>>(&mut self, r#type: Option<T>) {
		self.r#type = r#type.map(Into::into);
	}
	/// Get a specific entry.
	pub fn entry<'key, T: Into<EntryKey<'key>>>(&self, key: T) -> Option<&Entry> {
		key.into()
			.seek(self.entries.iter(), |ent| ent.name.as_deref())
	}
	/// Mutably get a specific entry.
	pub fn entry_mut<'key, T: Into<EntryKey<'key>>>(&mut self, key: T) -> Option<&mut Entry> {
		key.into()
			.seek(self.entries.iter_mut(), |ent| ent.name.as_deref())
	}
	/// Normalize node to kdl spec:
	/// - Empty children block gets removed
	/// - Normalize child document
	/// - Duplicate properties are removed
	pub fn normalize(&mut self) {
		if let Some(children) = &mut self.children {
			if children.nodes.is_empty() {
				self.children = None;
			} else {
				children.normalize();
			}
		}
		// TODO: the only use for hashmaps in the entire library, can it be removed?
		// TODO: this is simply an unlikely string-pointer
		// consider a real way to get a fake/random string pointer
		// or otherwise mark indexes as used with few allocatioons
		let marker = SmolStr::new_static(&"\0temp"[5..]);
		// two-pass approach to remove duplicate props
		let mut seen = HashSet::new();
		for entry in self.entries.iter_mut().rev() {
			if let Some(name) = &mut entry.name {
				if seen.contains(&**name) {
					*name = marker.clone();
				} else {
					seen.insert(&**name);
				}
			}
		}
		drop(seen);
		self.entries.retain(|ent| {
			!ent.name
				.as_ref()
				.is_some_and(|name| ptr_eq(name.as_ptr(), marker.as_ptr()) && name.is_empty())
		});
	}
}

impl fmt::Debug for Node {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Node")
			.field("type", option_debug(self.type_hint().as_ref()))
			.field("name", &self.name)
			.field("props", &self.entries)
			.field("children", option_debug(self.children.as_ref()))
			.finish()
	}
}
impl fmt::Display for Node {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if let Some(r#type) = &self.r#type {
			write!(f, "({})", IdentDisplay(r#type))?;
		}
		fmt::Display::fmt(&IdentDisplay(&self.name), f)?;
		for entry in &self.entries {
			write!(f, " {entry}")?;
		}
		if let Some(children) = &self.children {
			// make rust fmt do indents for me
			struct Children<'this>(&'this Document, Cell<bool>);
			impl fmt::Debug for Children<'_> {
				fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
					fmt::Display::fmt(self.0, f)?;
					// really stupid hack to have debug_set not print the trailing comma
					// (while not ignoring real errors!)
					self.1.set(true);
					Err(fmt::Error)
				}
			}
			struct Block<'this>(&'this Document);
			impl fmt::Debug for Block<'_> {
				fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
					let children = Children(self.0, Cell::new(false));
					let result = f.debug_set().entry(&children).finish();
					if children.1.get() { Ok(()) } else { result }
				}
			}
			f.write_str(" ")?;
			write!(f, "{:#?}\n}}", Block(children))?;
		}
		Ok(())
	}
}
impl<'key, T: Into<EntryKey<'key>>> Index<T> for Node {
	type Output = Entry;
	fn index(&self, index: T) -> &Self::Output {
		let key = index.into();
		self.entry(key)
			.unwrap_or_else(|| panic!("Key {key:?} does not exist in node"))
	}
}
impl<'key, T: Into<EntryKey<'key>>> IndexMut<T> for Node {
	fn index_mut(&mut self, index: T) -> &mut Self::Output {
		let key = index.into();
		self.entry_mut(key)
			.unwrap_or_else(|| panic!("Key {key:?} does not exist in node"))
	}
}

/// A `prop` or `value` element, a piece of labelled information.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Entry {
	/// The name ("key"), if this property is an entry.
	pub name: Option<SmolStr>,
	/// The entry's type hint.
	pub r#type: Option<SmolStr>,
	/// The entry's value.
	pub value: Value,
}

impl Entry {
	/// Create an entry that represents a plain value.
	pub fn new_value(value: Value) -> Self {
		Self {
			name: None,
			r#type: None,
			value,
		}
	}
	/// Create an entry that represents a named property.
	pub fn new_prop<T: Into<SmolStr>>(name: T, value: Value) -> Self {
		Self {
			name: Some(name.into()),
			r#type: None,
			value,
		}
	}
	/// Get the entry's name, if it has one.
	pub fn name(&self) -> Option<&str> { self.name.as_deref() }
	/// Set or clear the entry's name.
	pub fn set_name<T: Into<SmolStr>>(&mut self, name: Option<T>) {
		self.name = name.map(Into::into);
	}
	/// Get the entry's type hint.
	pub fn type_hint(&self) -> Option<&str> { self.r#type.as_deref() }
	/// Set the entry's type hint.
	pub fn set_type_hint<T: Into<SmolStr>>(&mut self, r#type: Option<T>) {
		self.r#type = r#type.map(Into::into);
	}
}

impl fmt::Debug for Entry {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Entry")
			.field("name", &self.name)
			.field("type", option_debug(self.type_hint().as_ref()))
			.field("value", &self.value)
			.finish()
	}
}
impl fmt::Display for Entry {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if let Some(name) = &self.name {
			write!(f, "{}=", IdentDisplay(name))?;
		}
		if let Some(r#type) = &self.r#type {
			write!(f, "({})", IdentDisplay(r#type))?;
		}
		fmt::Display::fmt(&self.value, f)
	}
}
impl<K: Into<SmolStr>, V: Into<Value>> From<(K, V)> for Entry {
	fn from((name, value): (K, V)) -> Self { Self::new_prop(name.into(), value.into()) }
}
impl<V: Into<Value>> From<V> for Entry {
	fn from(value: V) -> Self { Self::new_value(value.into()) }
}

/// A numeric or textual key to index an [`Entry`] in a [`Node`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKey<'key> {
	/// Index the nth value
	Value(usize),
	/// Index the right-most property with this name
	Property(&'key str),
}
impl EntryKey<'_> {
	fn seek<T>(
		self,
		mut iter: impl DoubleEndedIterator<Item = T>,
		name: impl Fn(&T) -> Option<&str>,
	) -> Option<T> {
		match self {
			EntryKey::Value(key) => iter.filter(|ent| name(ent).is_none()).nth(key),
			// right-most property overrides value
			EntryKey::Property(key) => iter.rfind(|ent| name(ent) == Some(key)),
		}
	}
}
impl From<usize> for EntryKey<'_> {
	fn from(value: usize) -> Self { Self::Value(value) }
}
impl<'key> From<&'key str> for EntryKey<'key> {
	fn from(value: &'key str) -> Self { Self::Property(value) }
}

/// A `value`, a piece of information.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub enum Value {
	/// A textual value, quoted or unquoted.
	String(SmolStr),
	/// A numeric value, including `#nan`, `#inf`, and `#-inf`.
	Number(Number),
	/// A boolean value, `#true` or `#false`.
	Bool(bool),
	/// The `#null` value.
	#[default]
	Null,
}

// TODO: maybe some value helper methods? e.g. type casting

impl fmt::Debug for Value {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::String(value) => fmt::Debug::fmt(&**value, f),
			Self::Number(value) => fmt::Debug::fmt(value, f),
			Self::Bool(true) => f.write_str("#true"),
			Self::Bool(false) => f.write_str("#false"),
			Self::Null => f.write_str("#null"),
		}
	}
}
impl fmt::Display for Value {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::String(value) => fmt::Display::fmt(&IdentDisplay(value), f),
			Self::Number(value) => fmt::Display::fmt(value, f),
			Self::Bool(true) => f.write_str("#true"),
			Self::Bool(false) => f.write_str("#false"),
			Self::Null => f.write_str("#null"),
		}
	}
}
impl<'text> From<&'text str> for Value {
	fn from(value: &'text str) -> Self { Self::String(value.into()) }
}
impl From<SmolStr> for Value {
	fn from(value: SmolStr) -> Self { Self::String(value) }
}
impl From<String> for Value {
	fn from(value: String) -> Self { Self::String(value.into()) }
}
impl<T: Into<Number>> From<T> for Value {
	fn from(value: T) -> Self { Self::Number(value.into()) }
}
impl From<bool> for Value {
	fn from(value: bool) -> Self { Self::Bool(value) }
}
impl From<()> for Value {
	fn from((): ()) -> Self { Self::Null }
}
impl<T: Into<Value>> From<Option<T>> for Value {
	fn from(value: Option<T>) -> Self {
		match value {
			Some(v) => v.into(),
			_ => Self::Null,
		}
	}
}

/// An arbitrary-size document-formatted number,
/// can convert to/from standard number types as needed.
// implementations in `number` module
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Number(SmolStr);

/// A document-stream event.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Event {
	/// Beginning of a node, terminated by a matching `End` event.
	Node {
		/// Optional node type hint.
		r#type: Option<SmolStr>,
		/// Node name.
		name: SmolStr,
	},
	/// A property or value on the `Node`.
	Entry(Entry),
	/// The beginning of the `Node`s children.
	Children,
	/// The end of the `Node` and its children block.
	End,
}

impl fmt::Debug for Event {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Node { r#type, name } => f
				.debug_struct("Node")
				.field("type", option_debug(r#type.as_ref()))
				.field("name", name)
				.finish(),
			Self::Entry(entry) => write!(f, "{entry:?}"),
			Self::Children => write!(f, "Children"),
			Self::End => write!(f, "End"),
		}
	}
}
