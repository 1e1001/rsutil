// SPDX-License-Identifier: MIT OR Apache-2.0
// TODO: it's very possible to make this stable-able:
// - ptr_metadata: remove if log#666 gets resolved
#![feature(ptr_metadata)]
//! [![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/rsutil/tree/main/multiline-logger)
//! [![Crates.io](https://img.shields.io/crates/v/multiline-logger)](https://crates.io/crates/multiline-logger)
//! [![docs.rs](https://img.shields.io/docsrs/multiline-logger)](https://docs.rs/multiline-logger)
//! [![MIT OR Apache-2.0](https://img.shields.io/crates/l/multiline-logger)](https://github.com/1e1001/rsutil/blob/main/multiline-logger/README.md#License)
//!
//! Fancy lightweight debug output:
//! - Not excessively dynamic but still configurable
//! - Logs messages and crashes
//! - Looks very nice (in my opinion)
//!
//! | Platform | Console output | File output | Backtraces |
//! |-:|-|:-:|-|
//! | Native | `stderr` (colored!) | &check; | `backtrace` feature |
//! | Web | web `console` (colored!) | &cross; | `backtrace` feature |
//!
//! Get started by creating a [`Settings`] and calling [`init`].
//!
//! [`init`]: Settings::init

use std::io::{self, Write};
use std::mem::replace;
use std::num::NonZeroU32;
use std::panic::Location;
use std::path::Path;
use std::sync::Mutex;
use std::thread::{self, Thread, ThreadId};
use std::{fmt, panic, ptr};

/// For convenience :)
pub use log;
use log::{Level, LevelFilter, Log, set_logger, set_max_level};
use sys_abstract::SystemImpl;
use time::{Date, OffsetDateTime};

mod sys_abstract;

#[cfg(target_arch = "wasm32")]
mod sys_web;
#[cfg(target_arch = "wasm32")]
use sys_web::System;

#[cfg(not(target_arch = "wasm32"))]
mod sys_native;
#[cfg(not(target_arch = "wasm32"))]
use sys_native::System;

/// Settings for the logger
pub struct Settings {
	/// A human-readable name for the application
	pub title: &'static str,
	/// List of module-prefix filters to match against,
	/// earlier filters get priority
	pub filters: &'static [(&'static str, LevelFilter)],
	/// Optional file path to output to (desktop only)
	pub file_out: Option<&'static Path>,
	/// Set to `true` to output to an appropriate console
	pub console_out: bool,
	/// Enables the formatted panic hook, and calls the supplied function.
	/// Use `|_| ()` if you don't have anything to run
	pub panic_hook: Option<fn(Panic<'_>)>,
}

impl Settings {
	/// Initializes the logger
	///
	/// # Panics
	/// will panic if initialization fails in any way
	pub fn init(self) {
		let Self {
			title,
			filters,
			file_out,
			console_out,
			panic_hook,
		} = self;
		let max_level = filters
			.iter()
			.map(|&(_, level)| level)
			.max()
			.unwrap_or(LevelFilter::Off);
		if let Some(handler) = panic_hook {
			// set the hook before installing the logger,
			// to show panic messages if logger initialization breaks
			panic::set_hook(Box::new(panic_handler(handler)));
		}
		let date = now().date();
		let logger = Logger {
			title,
			filters,
			file_out: file_out.map(System::file_new),
			console_out: console_out.then(System::console_new),
			prev_day: Mutex::new(date.to_julian_day()),
		};
		let message = Header { title, date };
		if let Some(out) = &logger.file_out {
			System::file_p_header(out, &message);
		}
		if let Some(out) = &logger.console_out {
			System::console_p_header(out, &message);
		}
		set_logger(upcast_log(Box::leak(Box::new(logger)))).expect("Failed to apply logger");
		set_max_level(max_level);
	}
}

// TODO: remove this once log#666 gets resolved
fn as_dyn_ref(logger: *const Logger) -> *const dyn Log {
	// split into one function to always attach the same metadata
	logger as *const dyn Log
}
fn upcast_log(logger: &'static Logger) -> &'static dyn Log {
	// SAFETY: as_dyn_ref returns a reference to the same object as passed in
	unsafe { &*as_dyn_ref(logger) }
}
fn downcast_log(log: &'static dyn Log) -> Option<&'static Logger> {
	// horribly cursed implementation to fetch a reference to the installed logger
	let (logger_ptr, logger_meta) = (&raw const *log).to_raw_parts();
	let (_, fake_logger_meta) = as_dyn_ref(ptr::null::<Logger>()).to_raw_parts();
	(logger_meta == fake_logger_meta).then(|| {
		// SAFETY: v-tables match so it's probably ours!
		unsafe { &*logger_ptr.cast::<Logger>() }
	})
}

// logger context
struct Logger {
	title: &'static str,
	filters: &'static [(&'static str, LevelFilter)],
	file_out: Option<<System as SystemImpl>::File>,
	console_out: Option<<System as SystemImpl>::Console>,
	prev_day: Mutex<i32>,
}

impl Log for Logger {
	fn enabled(&self, meta: &log::Metadata) -> bool {
		for (name, level) in self.filters {
			if meta.target().starts_with(name) {
				return *level >= meta.level();
			}
		}
		false
	}
	fn log(&self, record: &log::Record) {
		if self.enabled(record.metadata()) {
			let now = now();
			let date = now.date();
			let day = date.to_julian_day();
			let date = match self.prev_day.lock() {
				Ok(mut lock) => (replace(&mut *lock, day) != day).then_some(date),
				Err(_) => None,
			};
			let thread = thread::current();
			let message = Record {
				date,
				module: record.module_path().unwrap_or("?"),
				line: NonZeroU32::new(record.line().unwrap_or(0)),
				thread: ThreadName::new(&thread),
				args: *record.args(),
				hmsms: now.time().as_hms_milli(),
				level: record.level(),
			};
			if let Some(out) = &self.file_out {
				System::file_p_record(out, &message);
			}
			if let Some(out) = &self.console_out {
				System::console_p_record(out, &message);
			}
		}
	}
	fn flush(&self) {
		self.file_out.as_ref().map(System::file_flush);
		self.console_out.as_ref().map(System::console_flush);
	}
}

fn now() -> OffsetDateTime {
	OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

/// Name / Id of a thread
#[derive(Debug)]
pub enum ThreadName<'data> {
	/// Thread has a name
	Name(&'data str),
	/// Thread is ID only
	Id(ThreadId),
}
impl<'data> ThreadName<'data> {
	fn new(thread: &'data Thread) -> Self {
		if let Some(name) = thread.name() {
			Self::Name(name)
		} else {
			Self::Id(thread.id())
		}
	}
}
impl fmt::Display for ThreadName<'_> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			ThreadName::Name(name) => write!(f, "Thread {name:?}"),
			ThreadName::Id(id) => write!(f, "{id:?}"),
		}
	}
}

/// ```text
/// {BY}== title - date ==
/// ```
struct Header {
	title: &'static str,
	date: Date,
}

/// ```text
/// {BY}= date =
/// {BB}h:m:s.ms {BG}module:line{BM} thread
/// {L}level {0}message
/// {L}    | {0}message
/// ```
struct Record<'data> {
	date: Option<Date>,
	module: &'data str,
	line: Option<NonZeroU32>,
	thread: ThreadName<'data>,
	args: fmt::Arguments<'data>,
	hmsms: (u8, u8, u8, u16),
	level: Level,
}

/// System-agnostic backtrace type
#[derive(Debug)]
pub struct Backtrace {
	#[cfg(feature = "backtrace")]
	data: <System as SystemImpl>::Backtrace,
	#[cfg(not(feature = "backtrace"))]
	data: (),
}

impl Backtrace {
	fn capture() -> Self {
		Self {
			data: System::backtrace_new(),
		}
	}
	// TODO: platform-specific converters
	// TODO: some way to get color printing
	// TODO: impl Display via some fucked up io::Write → fmt::Write adapter
	// for now I just implemented all I need
	/// Print backtrace to a writer
	/// # Errors
	/// if the writer errors, or backtrace error
	pub fn write<W: Write>(&self, writer: W) -> io::Result<()> {
		System::backtrace_write(&self.data, writer)
	}
	/// Get the backtrace as a string
	pub fn as_string(&self) -> String { System::backtrace_string(&self.data) }
}

/// Panic handler information. This structure will change with updates!
/// ```text
/// {BR}== title - {BM}thread{BR} Panic ==
/// {0}message
/// {BG}→ location
/// {0}backtrace```
pub struct Panic<'data> {
	/// Panicking thread
	pub thread: ThreadName<'data>,
	/// Panic text
	pub message: Option<&'data str>,
	/// Panic location
	pub location: Option<Location<'data>>,
	/// Application title
	pub title: &'data str,
	/// Log file path, if you want to show it to the user
	pub path: Option<&'data Path>,
	/// Backtrace (or not)
	pub trace: Backtrace,
}

impl fmt::Debug for Panic<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Panic")
			.field("thread", &self.thread)
			.field("message", &self.message)
			.field("location", &self.location)
			.field("title", &self.title)
			.field("path", &self.path)
			.finish_non_exhaustive()
	}
}

impl Panic<'_> {
	fn message_str(&self) -> &str { self.message.unwrap_or("[non-string message]") }
	fn location_display(&self) -> &dyn fmt::Display {
		self.location.as_ref().map_or(&"[citation needed]", |v| v)
	}
}

fn panic_handler(handler: fn(Panic)) -> impl Fn(&panic::PanicHookInfo) {
	move |info: &panic::PanicHookInfo| {
		let logger = downcast_log(log::logger());
		let thread = thread::current();
		let mut message = Panic {
			thread: ThreadName::new(&thread),
			message: info.payload_as_str(),
			location: info.location().copied(),
			title: "[unknown?]",
			path: None,
			trace: Backtrace::capture(),
		};
		if let Some(logger) = logger {
			message.title = logger.title;
			message.path = System::file_path(logger.file_out.as_ref());
			if let Some(out) = &logger.file_out {
				System::file_p_panic(out, &message);
			}
			if let Some(out) = &logger.console_out {
				System::console_p_panic(out, &message);
			}
			handler(message);
		} else {
			System::fallback_p_panic(&message);
		}
	}
}
