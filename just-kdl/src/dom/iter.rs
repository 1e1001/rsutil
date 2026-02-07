// SPDX-License-Identifier: MIT OR Apache-2.0
//! Iterator types.
// TODO: versions that don't consume the iterator

use alloc::boxed::Box;
use alloc::vec::{IntoIter as VecIter, Vec};
use core::mem;

use crate::dom::{Document, Entry, Event, Node};
use crate::reader::Spanned;

/// Build a document from an [`Event`] stream.
///
/// This is a fairly low-level operation, for normal use, see the
/// [`FromIterator`] implementations on [`Node`] and [`Document`].
///
/// This does not validate the event stream.
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
	/// Add a single event to the document.
	/// # Panics
	/// on an invalid event stream.
	#[expect(clippy::unwrap_in_result, reason = "validity checks")]
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

// TODO/perf: these iterator implementations are stupidly recursive,
// some sort of explicit stack walking approach might be better
// Once that is done, use these iterators in `Display` impls

/// Iterator over a [`Document`].
pub struct DocumentIter {
	current: Box<NodeIter>,
	tail: VecIter<Node>,
}

impl Iterator for DocumentIter {
	type Item = Event;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			if let Some(next) = self.current.next() {
				return Some(next);
			} else if let Some(node) = self.tail.next() {
				*self.current = node.into_iter();
			} else {
				return None;
			}
		}
	}
}

impl IntoIterator for Document {
	type Item = Event;
	type IntoIter = DocumentIter;
	fn into_iter(self) -> Self::IntoIter {
		DocumentIter {
			current: Box::new(NodeIter(NodeIterInner::Done)),
			tail: self.nodes.into_iter(),
		}
	}
}

/// Iterator over a [`Node`].
pub struct NodeIter(NodeIterInner);

enum NodeIterInner {
	Start(Node),
	Body {
		entries: VecIter<Entry>,
		children: Option<DocumentIter>,
	},
	Children(DocumentIter),
	Done,
}

impl Iterator for NodeIter {
	type Item = Event;
	fn next(&mut self) -> Option<Self::Item> {
		Some(match &mut self.0 {
			NodeIterInner::Start(_) => {
				// this looks terrible
				let NodeIterInner::Start(Node {
					r#type,
					name,
					entries,
					children,
				}) = mem::replace(&mut self.0, NodeIterInner::Done)
				else {
					unreachable!()
				};
				self.0 = NodeIterInner::Body {
					entries: entries.into_iter(),
					children: children.map(Document::into_iter),
				};
				Event::Node { r#type, name }
			}
			NodeIterInner::Body { entries, children } => {
				if let Some(next) = entries.next() {
					Event::Entry(next)
				} else if let Some(iter) = children.take() {
					self.0 = NodeIterInner::Children(iter);
					Event::Children
				} else {
					self.0 = NodeIterInner::Done;
					Event::End
				}
			}
			NodeIterInner::Children(iter) => {
				if let Some(next) = iter.next() {
					next
				} else {
					self.0 = NodeIterInner::Done;
					Event::End
				}
			}
			NodeIterInner::Done => return None,
		})
	}
}
impl IntoIterator for Node {
	type Item = Event;
	type IntoIter = NodeIter;
	fn into_iter(self) -> Self::IntoIter { NodeIter(NodeIterInner::Start(self)) }
}
