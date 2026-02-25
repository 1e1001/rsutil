// SPDX-License-Identifier: MIT OR Apache-2.0
//! Iterator types.

use alloc::vec::{IntoIter as VecIter, Vec};
use core::slice::{Iter as SliceIter, from_ref as slice_from_ref};

use crate::dom::{Document, Entry, Event, Node};
use crate::reader::Spanned;

/// Build a document from an [`Event`] stream.
///
/// This is a push-based builder, for a pull-based use (e.g. taking from a
/// [`Reader`]), see the [`FromIterator`] implementations on [`Node`] and
/// [`Document`].
///
/// This does not validate the event stream.
///
/// [`Reader`]: crate::reader::Reader
#[derive(Debug)]
pub struct DocumentBuilder {
	stack: Vec<Node>,
}

impl Default for DocumentBuilder {
	fn default() -> Self { Self::new() }
}
impl DocumentBuilder {
	/// Create a new builder.
	pub fn new() -> Self { Self { stack: Vec::new() } }
	/// Add a single event to the document. Returns a possible top-level node.
	/// # Panics
	/// On an invalid event.
	#[expect(clippy::unwrap_in_result, reason = "option is not success information")]
	pub fn push(&mut self, event: Event) -> Option<Node> {
		match event {
			Event::Node { r#type, name } => self.stack.push(Node {
				r#type,
				name,
				..Default::default()
			}),
			Event::Entry(entry) => self.stack.last_mut().unwrap().entries.push(entry),
			Event::Children => self.stack.last_mut().unwrap().children = Some(Document::new()),
			Event::End => {
				let finished = self.stack.pop().unwrap();
				if let Some(next) = self.stack.last_mut() {
					next.children.as_mut().unwrap().nodes.push(finished);
				} else {
					return Some(finished);
				}
			}
		}
		None
	}
}
/// Assumes event stream is valid.
impl FromIterator<Event> for Document {
	fn from_iter<T: IntoIterator<Item = Event>>(iter: T) -> Self {
		let mut builder = DocumentBuilder::new();
		let mut nodes = Vec::new();
		for event in iter {
			// TODO/perf: compare codegen w/ manual option check
			nodes.extend(builder.push(event));
		}
		Document { nodes }
	}
}
/// Assumes event stream is valid, only takes first node from iterator.
impl FromIterator<Event> for Node {
	fn from_iter<T: IntoIterator<Item = Event>>(iter: T) -> Self {
		let mut builder = DocumentBuilder::new();
		for event in iter {
			if let Some(result) = builder.push(event) {
				return result;
			}
		}
		// fallback node
		Node::default()
	}
}

// Spanned iterator shorthands, just drop the spans
/// Assumes event stream is valid.
impl FromIterator<Spanned<Event>> for Document {
	fn from_iter<T: IntoIterator<Item = Spanned<Event>>>(iter: T) -> Self {
		iter.into_iter().map(|(event, _)| event).collect()
	}
}
/// Assumes event stream is valid, only takes first node from iterator.
impl FromIterator<Spanned<Event>> for Node {
	fn from_iter<T: IntoIterator<Item = Spanned<Event>>>(iter: T) -> Self {
		iter.into_iter().map(|(event, _)| event).collect()
	}
}

// owned / borrowed has different iterator (and iterator value) types
trait GenericIterKind: Sized {
	type Node;
	type Entry;
	type NodeIter: Iterator<Item = Self::Node>;
	type EntryIter: Iterator<Item = Self::Entry>;
	fn entry_event(entry: Self::Entry) -> Event;
	fn document_iter(doc: Self) -> Self::NodeIter;
	fn node_iter(node: Self::Node) -> Self::NodeIter;
	fn node_event(node: Self::Node) -> (Event, Self::EntryIter, Option<Self>);
}

/// owned case
impl GenericIterKind for Document {
	type Node = Node;
	type Entry = Entry;
	type NodeIter = VecIter<Node>;
	type EntryIter = VecIter<Entry>;
	fn entry_event(entry: Self::Entry) -> Event { Event::Entry(entry) }
	fn document_iter(doc: Self) -> Self::NodeIter { doc.nodes.into_iter() }
	fn node_iter(node: Self::Node) -> Self::NodeIter { vec![node].into_iter() }
	fn node_event(node: Self::Node) -> (Event, Self::EntryIter, Option<Self>) {
		let Node {
			r#type,
			name,
			entries,
			children,
		} = node;
		(Event::Node { r#type, name }, entries.into_iter(), children)
	}
}

/// borrowed case
impl<'doc> GenericIterKind for &'doc Document {
	type Node = &'doc Node;
	type Entry = &'doc Entry;
	type NodeIter = SliceIter<'doc, Node>;
	type EntryIter = SliceIter<'doc, Entry>;
	fn entry_event(entry: Self::Entry) -> Event { Event::Entry(entry.clone()) }
	fn document_iter(doc: Self) -> Self::NodeIter { doc.nodes.iter() }
	fn node_iter(node: Self::Node) -> Self::NodeIter { slice_from_ref(node).iter() }
	fn node_event(node: Self::Node) -> (Event, Self::EntryIter, Option<Self>) {
		let entries = node.entries.iter();
		let children = node.children.as_ref();
		let r#type = node.r#type.clone();
		let name = node.name.clone();
		(Event::Node { r#type, name }, entries, children)
	}
}

struct GenericIter<K: GenericIterKind> {
	stack: Vec<K::NodeIter>,
	top: Option<(K::EntryIter, Option<K>)>,
}

impl<K: GenericIterKind> GenericIter<K> {
	fn from_document(doc: K) -> Self {
		let stack = vec![K::document_iter(doc)];
		Self { stack, top: None }
	}
	fn from_node(node: K::Node) -> Self {
		let stack = vec![K::node_iter(node)];
		Self { stack, top: None }
	}
}

impl<K: GenericIterKind> Iterator for GenericIter<K> {
	type Item = Event;
	fn next(&mut self) -> Option<Self::Item> {
		if let Some((entries, children)) = &mut self.top {
			// currently in a node
			if let Some(entry) = entries.next() {
				Some(K::entry_event(entry))
			} else if let Some(nodes) = children.take() {
				self.stack.push(K::document_iter(nodes));
				self.top = None;
				Some(Event::Children)
			} else {
				self.top = None;
				Some(Event::End)
			}
		} else if let Some(iter) = self.stack.last_mut() {
			// currently in a document
			if let Some(node) = iter.next() {
				let (event, entries, nodes) = K::node_event(node);
				self.top = Some((entries, nodes));
				Some(event)
			} else {
				self.stack.pop();
				// final stack item needs no End event
				(!self.stack.is_empty()).then_some(Event::End)
			}
		} else {
			// done
			None
		}
	}
}

/// Owning iterator over a [`Document`] or [`Node`].
pub struct IntoIter(GenericIter<Document>);

impl Iterator for IntoIter {
	type Item = Event;
	fn next(&mut self) -> Option<Self::Item> { self.0.next() }
}

impl IntoIterator for Document {
	type Item = Event;
	type IntoIter = IntoIter;
	fn into_iter(self) -> Self::IntoIter { IntoIter(GenericIter::from_document(self)) }
}

impl IntoIterator for Node {
	type Item = Event;
	type IntoIter = IntoIter;
	fn into_iter(self) -> Self::IntoIter { IntoIter(GenericIter::from_node(self)) }
}

/// Borrowing iterator over a [`Document`] or [`Node`].
pub struct Iter<'doc>(GenericIter<&'doc Document>);

impl Iterator for Iter<'_> {
	type Item = Event;
	fn next(&mut self) -> Option<Self::Item> { self.0.next() }
}

impl<'doc> IntoIterator for &'doc Document {
	type Item = Event;
	type IntoIter = Iter<'doc>;
	fn into_iter(self) -> Self::IntoIter { Iter(GenericIter::from_document(self)) }
}

impl<'doc> IntoIterator for &'doc Node {
	type Item = Event;
	type IntoIter = Iter<'doc>;
	fn into_iter(self) -> Self::IntoIter { Iter(GenericIter::from_node(self)) }
}
