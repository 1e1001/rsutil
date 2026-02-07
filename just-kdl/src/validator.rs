// SPDX-License-Identifier: MIT OR Apache-2.0
//! Verification of event streams.
//!
//! Most library functions produce and expect to receive structurally-valid
//! event streams, use this to ensure untrusted event streams are valid.
//!
//! You probably want to start at [`Validator`].

use thiserror::Error;

use crate::dom::Event;

/// Error in validation
#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub enum ValidatorError {
	#[error("Too many End events")]
	#[doc = "Too many End events"]
	TooManyEnd,
	#[error("Unclosed nodes at end")]
	#[doc = "Unclosed nodes at end"]
	Unclosed,
	#[error("Expected {0}, got {1:?}")]
	#[doc = "Expected {0}, got {1}"]
	Expected(&'static str, Event),
}

#[derive(Debug, Clone, Copy)]
enum State {
	/// Node, End, Done
	Block,
	/// Entry, Children, End
	Node,
}

/// Event stream validator
#[derive(Debug)]
pub struct Validator {
	nest: usize,
	state: State,
}

impl Default for Validator {
	fn default() -> Self { Self::new() }
}

impl Validator {
	/// Create a new validator.
	pub const fn new() -> Self {
		Self {
			nest: 0,
			state: State::Block,
		}
	}
	/// Feed an event into validator.
	/// # Errors
	/// Returns any validation errors.
	pub fn push(&mut self, event: &Event) -> Result<(), ValidatorError> {
		self.state = match (self.state, event) {
			(_, Event::End) => {
				if let Some(nest) = self.nest.checked_sub(1) {
					self.nest = nest;
					State::Block
				} else {
					return Err(ValidatorError::TooManyEnd);
				}
			}
			(State::Node, Event::Entry { .. }) => State::Node,
			(State::Node, Event::Children) => State::Block,
			(State::Block, Event::Node { .. }) => {
				self.nest += 1;
				State::Node
			}
			(State::Node, event) => {
				return Err(ValidatorError::Expected(
					"Entry, Children, or End",
					event.clone(),
				));
			}
			(State::Block, event) => {
				return Err(ValidatorError::Expected(
					"Node, End, or Done",
					event.clone(),
				));
			}
		};
		Ok(())
	}
	/// Mark the end of the event stream.
	/// # Errors
	/// Returns a validation error from any unclosed nodes
	pub fn done(self) -> Result<(), ValidatorError> {
		if self.nest > 0 {
			Err(ValidatorError::Unclosed)
		} else {
			Ok(())
		}
	}
}
