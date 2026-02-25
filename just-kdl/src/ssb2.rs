// SPDX-License-Identifier: MIT OR Apache-2.0
//! `SmolStrBuilder` doesn't have enough features for my use.
//! This is a re-implementation of it that has those features

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::iter::repeat_n;

use smol_str::SmolStr;

const MAX_INLINE: usize = 23;

/// do not construct variants manually
pub enum SmolStrBuilder2 {
	Inline {
		size: usize,
		// buffer must always be valid utf-8
		buffer: [u8; MAX_INLINE],
	},
	Heap(String),
}

impl Default for SmolStrBuilder2 {
	fn default() -> Self {
		Self::Inline {
			size: 0,
			buffer: [0; MAX_INLINE],
		}
	}
}

impl SmolStrBuilder2 {
	pub fn new() -> Self { Self::default() }
	pub fn finish(self) -> SmolStr {
		match self {
			SmolStrBuilder2::Inline { size, buffer } => {
				let buffer = &buffer[..size];
				// SAFETY: struct invariant
				SmolStr::new_inline(unsafe { str::from_utf8_unchecked(buffer) })
			}
			SmolStrBuilder2::Heap(text) => SmolStr::from(Arc::from(text.into_boxed_str())),
		}
	}
	pub fn len(&self) -> usize {
		match self {
			SmolStrBuilder2::Inline { size, .. } => *size,
			SmolStrBuilder2::Heap(text) => text.len(),
		}
	}
	fn reserve(&mut self, n: usize) {
		match self {
			SmolStrBuilder2::Inline { size, buffer } => {
				if *size + n > MAX_INLINE {
					let mut bytes = Vec::with_capacity(*size + n);
					bytes.extend_from_slice(&buffer[..*size]);
					// SAFETY: struct invariant
					*self = SmolStrBuilder2::Heap(unsafe { String::from_utf8_unchecked(bytes) });
				}
			}
			SmolStrBuilder2::Heap(text) => text.reserve(n),
		}
	}
	pub fn push(&mut self, c: char) {
		let len = c.len_utf8();
		self.reserve(len);
		match self {
			SmolStrBuilder2::Inline { size, buffer } => {
				// preserves utf-8: writes one codepoint (encoded)
				debug_assert_eq!(
					c.encode_utf8(&mut buffer[*size..]).len(),
					len,
					"utf8 horrors"
				);
				*size += len;
			}
			SmolStrBuilder2::Heap(text) => text.push(c),
		}
	}
	pub fn push_str(&mut self, s: &str) {
		self.reserve(s.len());
		match self {
			SmolStrBuilder2::Inline { size, buffer } => {
				// preserves utf-8: writes a &str
				buffer[*size..][..s.len()].copy_from_slice(s.as_bytes());
				*size += s.len();
			}
			SmolStrBuilder2::Heap(text) => text.push_str(s),
		}
	}
	pub fn as_bytes(&self) -> &[u8] {
		match self {
			SmolStrBuilder2::Inline { size, buffer } => &buffer[..*size],
			SmolStrBuilder2::Heap(text) => text.as_bytes(),
		}
	}
	// self.truncate(self.floor_char_boundary(len))
	pub fn truncate_floor(&mut self, len: usize) {
		// copy of `core::str::floor_char_boundary` for bytes
		#[expect(
			clippy::cast_possible_wrap,
			clippy::missing_assert_message,
			reason = "code from libcore"
		)]
		fn floor_char_boundary(text: &[u8], index: usize) -> usize {
			if index >= text.len() {
				text.len()
			} else {
				let mut i = index;
				while i > 0 && (text[i] as i8) < -0x40 {
					i -= 1;
				}
				debug_assert!(i >= index.saturating_sub(3));
				i
			}
		}
		match self {
			SmolStrBuilder2::Inline { size, buffer } => {
				*size = floor_char_boundary(buffer, len.min(*size));
			}
			SmolStrBuilder2::Heap(text) => text.truncate(text.floor_char_boundary(len)),
		}
	}
	// TODO: use ascii::Char once it's stable
	pub fn push_repeated(&mut self, c: u8, n: usize) {
		assert!(c < 128, "bad char write");
		self.reserve(n);
		match self {
			SmolStrBuilder2::Inline { size, buffer } => {
				// preserves utf-8: writes ascii
				buffer[*size..][..n].fill(c);
				*size += n;
			}
			SmolStrBuilder2::Heap(text) => text.extend(repeat_n(c as char, n)),
		}
	}
	/// panics if first character isn't a byte
	pub fn swap0(&mut self, c: u8) {
		assert!(c < 128, "bad char write");
		match self {
			SmolStrBuilder2::Inline { size, buffer } => {
				assert!(*size > 0, "out of bounds swap0");
				assert!(buffer[0] < 128, "bad swap0");
				buffer[0] = c;
			}
			SmolStrBuilder2::Heap(s) => {
				let substr = &mut s[..1];
				// SAFETY: existing value is a ascii character (asserted by string slice),
				// replaced with new ascii character
				unsafe {
					substr.as_bytes_mut()[0] = c;
				}
			}
		}
	}
}
