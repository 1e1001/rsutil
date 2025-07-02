// SPDX-License-Identifier: MIT OR Apache-2.0
use std::fmt::format;
use std::num::NonZeroU32;
use std::path::Path;

use log::Level;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::sys_abstract::SystemImpl;
use crate::{Header, Panic, Record};

#[wasm_bindgen(module = "/src/sys_web.js")]
extern "C" {
	fn js_header(title: &str, year: i32, month: u8, day: u8);
	fn js_new_day(year: i32, month: u8, day: u8);
	fn js_record(
		module: &str,
		thread: &str,
		text: &str,
		line: u32,
		level: u8,
		hour: u8,
		minute: u8,
		second: u8,
		millisecond: u16,
	);
	fn js_panic(title: &str, thread: &str, text: &str, location: &str, trace: JsValue);
	fn js_trace() -> JsValue;
}

pub struct System;
impl SystemImpl for System {
	type File = ();
	#[inline]
	fn file_new(_path: &'static Path) -> Self::File {}
	#[inline]
	fn file_p_header(_this: &Self::File, _message: &Header) {}
	#[inline]
	fn file_p_record(_this: &Self::File, _message: &Record) {}
	#[inline]
	fn file_p_panic(_this: &Self::File, _message: &Panic) {}
	#[inline]
	fn file_flush(_this: &Self::File) {}
	#[inline]
	fn file_path(_this: Option<&Self::File>) -> Option<&'static Path> { None }
	type Console = ();
	#[inline]
	fn console_new() -> Self::Console {}
	#[inline]
	fn console_p_header(_this: &Self::Console, message: &Header) {
		let Header { title, date } = message;
		let (y, m, d) = date.to_calendar_date();
		js_header(title, y, m as u8, d);
	}
	// since web devtools already have message framing, deviate from format here
	#[inline]
	fn console_p_record(_this: &Self::Console, message: &Record) {
		let Record {
			date,
			module,
			line,
			thread,
			args,
			hmsms: (h, m, s, ms),
			level,
		} = message;
		if let Some(date) = date {
			let (y, m, d) = date.to_calendar_date();
			js_new_day(y, m as u8, d);
		}
		js_record(
			module,
			&thread.to_string(),
			&format(*args),
			line.map_or(0, NonZeroU32::get),
			match level {
				Level::Error => 0,
				Level::Warn => 1,
				Level::Info => 2,
				Level::Debug => 3,
				Level::Trace => 4,
			},
			*h,
			*m,
			*s,
			*ms,
		);
	}
	#[inline]
	fn console_p_panic(_this: &Self::Console, message: &Panic) { Self::fallback_p_panic(message); }
	#[inline]
	fn console_flush(_this: &Self::Console) {}
	#[inline]
	fn fallback_p_panic(message: &Panic) {
		let Panic {
			thread,
			title,
			trace,
			..
		} = message;
		js_panic(
			title,
			&thread.to_string(),
			message.message_str(),
			&format!("{}", message.location_display()),
			trace.data.clone(),
		);
	}
	#[cfg(feature = "backtrace")]
	type Backtrace = JsValue;
	#[cfg(feature = "backtrace")]
	#[inline]
	fn backtrace_new() -> Self::Backtrace { js_trace() }
	#[cfg(feature = "backtrace")]
	fn backtrace_write<W: std::io::Write>(
		trace: &Self::Backtrace,
		mut writer: W,
	) -> std::io::Result<()> {
		std::io::Write::write_all(&mut writer, Self::backtrace_string(trace).as_bytes())
	}
	#[cfg(feature = "backtrace")]
	fn backtrace_string(trace: &Self::Backtrace) -> String {
		// TODO: this shit don't work, get js-sys since it's there already
		trace.as_string().unwrap_or_default()
	}
}
