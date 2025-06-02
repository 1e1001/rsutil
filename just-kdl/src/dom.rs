// SPDX-License-Identifier: MIT OR Apache-2.0
//! document tree structures, start at [`Document`]

use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashSet;
use std::convert::Infallible;
use std::fmt;
use std::ops::{Index, IndexMut};

use crate::number::Number;
use crate::stream::{Error, Event, Parser};
use crate::{IdentDisplay, cow_static};

fn maybe_debug<T: fmt::Debug>(value: Option<&T>) -> &dyn fmt::Debug {
	match value {
		Some(value) => value,
		None => &None::<Infallible>,
	}
}

/// A `document` or `nodes` element, a container of [`Node`]
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct Document<'text> {
	/// The nodes in this document, in order
	pub nodes: Vec<Node<'text>>,
}

impl<'text> Document<'text> {
	/// Create a document with no children
	pub fn new() -> Self { Self::default() }
	/// Convert into an owned value
	pub fn into_owned(self) -> Document<'static> {
		Document {
			nodes: self.nodes.into_iter().map(Node::into_owned).collect(),
		}
	}
	/// Iterator over every node with a particular name
	pub fn get(&self, name: &str) -> impl Iterator<Item = &Node<'text>> {
		self.nodes.iter().filter(move |node| node.name() == name)
	}
	/// Mutable iterator over every node with a particular name
	pub fn get_mut(&mut self, name: &str) -> impl Iterator<Item = &mut Node<'text>> {
		self.nodes
			.iter_mut()
			.filter(move |node| node.name() == name)
	}
	pub fn parse(text: &'text str) -> Result<Self, Error> { Parser::new(text).collect() }
}

impl fmt::Debug for Document<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.write_str("Document ")?;
		f.debug_list().entries(&self.nodes).finish()
	}
}
impl fmt::Display for Document<'_> {
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
/// Currently panic's if the iterator is invalid, oh well
impl<'text> FromIterator<Event<'text>> for Document<'text> {
	fn from_iter<T: IntoIterator<Item = Event<'text>>>(iter: T) -> Self {
		let mut stack = vec![Document::new()];
		for event in iter {
			match event {
				Event::Node { r#type, name } => {
					let mut node = Node::new(name);
					node.set_type_hint(r#type);
					stack.last_mut().unwrap().nodes.push(node);
				}
				Event::Entry { r#type, key, value } => {
					let mut entry = Entry::new_value(value);
					entry.set_key(key);
					entry.set_type_hint(r#type);
					stack
						.last_mut()
						.unwrap()
						.nodes
						.last_mut()
						.unwrap()
						.entries
						.push(entry);
				}
				Event::Begin => stack.push(Document::new()),
				Event::End => {
					let children = stack.pop().unwrap();
					stack.last_mut().unwrap().nodes.last_mut().unwrap().children = Some(children);
				}
			}
		}
		let document = stack.pop().unwrap();
		assert!(stack.is_empty(), "invalid iterator stream");
		document
	}
}

/// A `node` element
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Node<'text> {
	r#type: Option<Cow<'text, str>>,
	name: Cow<'text, str>,
	/// The node's entries in order
	pub entries: Vec<Entry<'text>>,
	/// The node's child document
	pub children: Option<Document<'text>>,
}

impl<'text> Node<'text> {
	/// Create a new node with a name
	pub fn new(name: impl Into<Cow<'text, str>>) -> Self {
		Self {
			r#type: None,
			name: name.into(),
			entries: Vec::new(),
			children: None,
		}
	}
	/// Convert into an owned value
	pub fn into_owned(self) -> Node<'static> {
		Node {
			r#type: self.r#type.map(cow_static),
			name: cow_static(self.name),
			entries: self.entries.into_iter().map(Entry::into_owned).collect(),
			children: self.children.map(Document::into_owned),
		}
	}
	/// Get the node's name
	pub fn name(&self) -> &str { &self.name }
	/// Set the node's name
	pub fn set_name(&mut self, name: impl Into<Cow<'text, str>>) { self.name = name.into(); }
	/// Get the node's type hint
	pub fn type_hint(&self) -> Option<&str> { self.r#type.as_deref() }
	/// Set the node's type hint
	pub fn set_type_hint(&mut self, r#type: Option<impl Into<Cow<'text, str>>>) {
		self.r#type = r#type.map(Into::into);
	}
	/// Get a specific entry
	pub fn entry<'key>(&self, key: impl Into<EntryKey<'key>>) -> Option<&Entry<'text>> {
		key.into()
			.seek(self.entries.iter(), |ent| ent.key.as_deref())
	}
	/// Mutably get a specific entry
	pub fn entry_mut<'key>(&mut self, key: impl Into<EntryKey<'key>>) -> Option<&mut Entry<'text>> {
		key.into()
			.seek(self.entries.iter_mut(), |ent| ent.key.as_deref())
	}
	/// Normalize node to kdl spec:
	/// - Empty children block gets removed
	/// - Normalize child nodes
	/// - Duplicate properties are removed
	pub fn normalize(&mut self) {
		if let Some(children) = &mut self.children {
			if children.nodes.is_empty() {
				self.children = None;
			} else {
				for node in &mut children.nodes {
					node.normalize();
				}
			}
		}
		// TODO: this is simply an unlikely string-pointer
		// consider a real way to get a fake/random string pointer
		let marker = &"\0temp"[5..];
		// two-pass approach to remove duplicate props
		let mut seen = HashSet::new();
		for entry in self.entries.iter_mut().rev() {
			if let Some(key) = &mut entry.key {
				if seen.contains(key) {
					*key = Cow::Borrowed(marker);
				} else {
					seen.insert(&*key);
				}
			}
		}
		self.entries.retain(|ent| {
			!ent.key
				.as_ref()
				.is_some_and(|key| std::ptr::eq(key.as_ptr(), marker.as_ptr()) && key.is_empty())
		});
	}
}

impl fmt::Debug for Node<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Node")
			.field("type", maybe_debug(self.type_hint().as_ref()))
			.field("name", &self.name)
			.field("props", &self.entries)
			.field("children", maybe_debug(self.children.as_ref()))
			.finish()
	}
}
impl fmt::Display for Node<'_> {
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
			struct Children<'this>(&'this Document<'this>, Cell<bool>);
			impl fmt::Debug for Children<'_> {
				fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
					fmt::Display::fmt(self.0, f)?;
					// really stupid hack to have debug_set not print the trailing comma
					// (while not ignoring real errors!)
					self.1.set(true);
					Err(fmt::Error)
				}
			}
			struct Block<'this>(&'this Document<'this>);
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
impl<'key, 'text, T: Into<EntryKey<'key>>> Index<T> for Node<'text> {
	type Output = Entry<'text>;
	fn index(&self, index: T) -> &Self::Output {
		let key = index.into();
		self.entry(key)
			.unwrap_or_else(|| panic!("Key {key:?} does not exist in node"))
	}
}
impl<'key, 'text, T: Into<EntryKey<'key>>> IndexMut<T> for Node<'text> {
	fn index_mut(&mut self, index: T) -> &mut Self::Output {
		let key = index.into();
		self.entry_mut(key)
			.unwrap_or_else(|| panic!("Key {key:?} does not exist in node"))
	}
}

/// A `prop` or `value` element
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Entry<'text> {
	key: Option<Cow<'text, str>>,
	r#type: Option<Cow<'text, str>>,
	/// The value of this property
	pub value: Value<'text>,
}

impl<'text> Entry<'text> {
	/// Create an entry that represents a plain value
	pub fn new_value(value: Value<'text>) -> Self {
		Self {
			key: None,
			r#type: None,
			value,
		}
	}
	/// Create an entry that represents a named property
	pub fn new_prop(name: impl Into<Cow<'text, str>>, value: Value<'text>) -> Self {
		Self {
			key: Some(name.into()),
			r#type: None,
			value,
		}
	}
	/// Convert into an owned value
	pub fn into_owned(self) -> Entry<'static> {
		Entry {
			key: self.key.map(cow_static),
			r#type: self.r#type.map(cow_static),
			value: self.value.into_owned(),
		}
	}
	/// Get the property's key, if it has one
	pub fn key(&self) -> Option<&str> { self.key.as_deref() }
	/// Get the property's key, if it has one
	pub fn set_key(&mut self, key: Option<impl Into<Cow<'text, str>>>) {
		self.key = key.map(Into::into);
	}
	/// Get the property's type hint
	pub fn type_hint(&self) -> Option<&str> { self.r#type.as_deref() }
	/// Set the node's type hint
	pub fn set_type_hint(&mut self, r#type: Option<impl Into<Cow<'text, str>>>) {
		self.r#type = r#type.map(Into::into);
	}
}

impl fmt::Debug for Entry<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("Property")
			.field("key", &self.key)
			.field("type", maybe_debug(self.type_hint().as_ref()))
			.field("value", &self.value)
			.finish()
	}
}
impl fmt::Display for Entry<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		if let Some(key) = &self.key {
			write!(f, "{}=", IdentDisplay(key))?;
		}
		if let Some(r#type) = &self.r#type {
			write!(f, "({})", IdentDisplay(r#type))?;
		}
		fmt::Display::fmt(&self.value, f)
	}
}
impl<'text, K: Into<Cow<'text, str>>, V: Into<Value<'text>>> From<(K, V)> for Entry<'text> {
	fn from((name, value): (K, V)) -> Self { Self::new_prop(name.into(), value.into()) }
}
impl<'text, V: Into<Value<'text>>> From<V> for Entry<'text> {
	fn from(value: V) -> Self { Self::new_value(value.into()) }
}

/// A numeric or textual key to index an [`Entry`] in a [`Node`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKey<'text> {
	Pos(usize),
	Name(&'text str),
}
impl EntryKey<'_> {
	fn seek<T>(
		self,
		mut iter: impl DoubleEndedIterator<Item = T>,
		name: impl Fn(&T) -> Option<&str>,
	) -> Option<T> {
		match self {
			EntryKey::Pos(key) => iter.filter(|ent| name(ent).is_none()).nth(key),
			// right-most property overrides value
			EntryKey::Name(key) => iter.rfind(|ent| name(ent) == Some(key)),
		}
	}
}
impl From<usize> for EntryKey<'_> {
	fn from(value: usize) -> Self { Self::Pos(value) }
}
impl<'text> From<&'text str> for EntryKey<'text> {
	fn from(value: &'text str) -> Self { Self::Name(value) }
}

/// The value of an [`Entry`]
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Value<'text> {
	/// A textual value
	String(Cow<'text, str>),
	/// A numeric value
	Number(Number),
	/// A boolean value
	Bool(bool),
	/// The `#null` value
	Null,
}

impl Value<'_> {
	/// Convert into an owned value
	pub fn into_owned(self) -> Value<'static> {
		match self {
			Self::String(value) => Value::String(cow_static(value)),
			Self::Number(value) => Value::Number(value),
			Self::Bool(value) => Value::Bool(value),
			Self::Null => Value::Null,
		}
	}
	// TODO: maybe some helper methods?
}

impl fmt::Debug for Value<'_> {
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
impl fmt::Display for Value<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Value::String(value) => fmt::Display::fmt(&IdentDisplay(value), f),
			Value::Number(value) => fmt::Display::fmt(value, f),
			Value::Bool(true) => f.write_str("#true"),
			Value::Bool(false) => f.write_str("#false"),
			Value::Null => f.write_str("#null"),
		}
	}
}
impl<'text> From<&'text str> for Value<'text> {
	fn from(value: &'text str) -> Self { Self::String(Cow::Borrowed(value)) }
}
impl<'text> From<String> for Value<'text> {
	fn from(value: String) -> Self { Self::String(Cow::Owned(value)) }
}
impl<'text, T: Into<Number>> From<T> for Value<'text> {
	fn from(value: T) -> Self { Self::Number(value.into()) }
}
impl<'text> From<bool> for Value<'text> {
	fn from(value: bool) -> Self { Self::Bool(value) }
}
impl<'text> From<()> for Value<'text> {
	fn from((): ()) -> Self { Self::Null }
}
impl<'text, T: Into<Value<'text>>> From<Option<T>> for Value<'text> {
	fn from(value: Option<T>) -> Self {
		match value {
			Some(v) => v.into(),
			_ => Self::Null,
		}
	}
}
