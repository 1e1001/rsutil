// SPDX-License-Identifier: MIT OR Apache-2.0
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use log::Level;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, StandardStreamLock, WriteColor};

use crate::sys_abstract::SystemImpl;
use crate::{Header, Panic, Record};

fn color_spec(color: Color, bold: bool) -> ColorSpec {
	let mut res = ColorSpec::new();
	res.set_fg(Some(color));
	if bold {
		res.set_bold(bold).set_intense(bold);
	}
	res
}

fn color_reset() -> ColorSpec { ColorSpec::new() }

// run a function on every newline
struct Indent<T, F>(T, F);
impl<T: Write, F: FnMut(&mut T) -> io::Result<()>> Write for Indent<T, F> {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		let line = buf.split_inclusive(|v| *v == b'\n').next().unwrap_or(buf);
		let n = self.0.write(line)?;
		if n == line.len() && line.last() == Some(&b'\n') {
			self.1(&mut self.0)?;
		}
		Ok(n)
	}
	fn flush(&mut self) -> io::Result<()> { self.0.flush() }
}

pub struct System;
impl SystemImpl for System {
	type File = (&'static Path, File);
	#[inline]
	fn file_new(path: &'static Path) -> Self::File {
		// TODO: rename & compress old log files
		(path, File::create(path).expect("Failed to open log file!"))
	}
	#[inline]
	fn file_p_header(this: &Self::File, message: &Header) {
		let Header { title, date } = message;
		let mut f = &this.1;
		_ = writeln!(f, "== {title} - {date:?} ==");
	}
	#[inline]
	fn file_p_record(this: &Self::File, message: &Record) {
		let Record {
			date,
			module,
			line,
			thread,
			args,
			hmsms: (h, m, s, ms),
			level,
		} = message;
		let mut f = &this.1;
		if let Some(date) = date {
			_ = write!(f, " = {date:?} =");
		}
		_ = write!(f, "{h:>02}:{m:>02}:{s:>02}.{ms:>03} {module}");
		if let Some(line) = line {
			_ = write!(f, ":{line}");
		}
		_ = write!(f, " {thread}\n{} ", match level {
			Level::Error => "Error",
			Level::Warn => " Warn",
			Level::Info => " Info",
			Level::Debug => "Debug",
			Level::Trace => "Trace",
		});
		let mut indent = Indent(f, |f: &mut &File| f.write_all(b"    | "));
		_ = indent.write_fmt(*args);
		_ = writeln!(indent.0);
	}
	#[inline]
	fn file_p_panic(this: &Self::File, message: &Panic) {
		let Panic {
			thread,
			title,
			trace,
			..
		} = message;
		let mut f = &this.1;
		_ = writeln!(
			f,
			"== {title} - {thread} Panic ==\n{}\n→ {}",
			message.message_str(),
			message.location_display(),
		);
		#[cfg(feature = "backtrace")]
		let _ = color_backtrace::BacktracePrinter::new()
			.print_trace(&trace.data, &mut termcolor::NoColor::new(f));
		#[cfg(not(feature = "backtrace"))]
		let _ = trace;
	}
	#[inline]
	fn file_flush(this: &Self::File) { _ = (&this.1).flush(); }
	#[inline]
	fn file_path(this: Option<&(&'static Path, File)>) -> Option<&'static Path> {
		this.map(|&(path, _)| path)
	}
	type Console = StandardStream;
	#[inline]
	fn console_new() -> Self::Console {
		#[cfg(windows)]
		if stderr().is_terminal() {
			// open a console for output
			use windows_sys::Win32::System::Console;
			if unsafe { Console::AttachConsole(u32::MAX) } == 0 {
				unsafe { Console::AllocConsole() };
			}
		}
		StandardStream::stderr(ColorChoice::Auto)
	}
	#[inline]
	fn console_p_header(this: &Self::Console, message: &Header) {
		let Header { title, date } = message;
		let mut f = this.lock();
		_ = f.set_color(&color_spec(Color::Yellow, true));
		_ = writeln!(f, "== {title} - {date:?} ==");
		_ = f.set_color(&color_reset());
	}
	#[inline]
	fn console_p_record(this: &Self::Console, message: &Record) {
		let Record {
			date,
			module,
			line,
			thread,
			args,
			hmsms: (h, m, s, ms),
			level,
		} = message;
		let mut f = this.lock();
		if let Some(date) = date {
			_ = f.set_color(&color_spec(Color::Yellow, true));
			_ = writeln!(f, "= {date:?} =");
		}
		_ = f.set_color(&color_spec(Color::Blue, true));
		_ = write!(f, "{h:>02}:{m:>02}:{s:>02}.{ms:>03} ");
		_ = f.set_color(&color_spec(Color::Green, true));
		_ = write!(f, "{module}");
		if let Some(line) = line {
			_ = write!(f, ":{line}");
		}
		_ = f.set_color(&color_spec(Color::Magenta, true));
		_ = writeln!(f, " {thread}");
		let (color, name) = match level {
			Level::Error => (Color::Red, "Error"),
			Level::Warn => (Color::Yellow, " Warn"),
			Level::Info => (Color::Blue, " Info"),
			Level::Debug => (Color::Green, "Debug"),
			Level::Trace => (Color::Magenta, "Trace"),
		};
		let color = color_spec(color, false);
		_ = f.set_color(&color);
		_ = write!(f, "{name} ");
		_ = f.set_color(&color_reset());
		let mut indent = Indent(f, |f: &mut StandardStreamLock| {
			f.set_color(&color)?;
			f.write_all(b"    | ")?;
			f.set_color(&color_reset())
		});
		_ = indent.write_fmt(*args);
		_ = writeln!(indent.0);
	}
	#[inline]
	fn console_p_panic(this: &Self::Console, message: &Panic) {
		let Panic {
			thread,
			title,
			trace,
			..
		} = message;
		let mut f = this.lock();
		_ = f.set_color(&color_spec(Color::Red, true));
		_ = write!(f, "== {title} - ");
		_ = f.set_color(&color_spec(Color::Magenta, true));
		_ = write!(f, "{thread}");
		_ = f.set_color(&color_spec(Color::Red, true));
		_ = writeln!(f, " Panic ==");
		_ = f.set_color(&color_reset());
		_ = writeln!(f, "{}", message.message_str());
		_ = f.set_color(&color_spec(Color::Green, true));
		_ = writeln!(f, "→ {}", message.location_display());
		_ = f.set_color(&color_reset());
		#[cfg(feature = "backtrace")]
		// TODO: set a fun message here!
		let _ = color_backtrace::BacktracePrinter::new().print_trace(&trace.data, &mut f);
		let _ = trace;
	}
	#[inline]
	fn console_flush(this: &Self::Console) { _ = this.lock().flush(); }
	#[inline]
	fn fallback_p_panic(message: &Panic) {
		Self::console_p_panic(&StandardStream::stderr(ColorChoice::Never), message);
	}
	#[cfg(feature = "backtrace")]
	type Backtrace = backtrace::Backtrace;
	#[cfg(feature = "backtrace")]
	#[inline]
	fn backtrace_new() -> Self::Backtrace { backtrace::Backtrace::new() }
	#[cfg(feature = "backtrace")]
	fn backtrace_write<W: std::io::Write>(
		trace: &Self::Backtrace,
		writer: W,
	) -> std::io::Result<()> {
		color_backtrace::BacktracePrinter::new()
			.print_trace(trace, &mut termcolor::NoColor::new(writer))
	}
	#[cfg(feature = "backtrace")]
	fn backtrace_string(trace: &Self::Backtrace) -> String {
		let mut writer = Vec::new();
		_ = Self::backtrace_write(trace, &mut writer);
		String::from_utf8(writer).unwrap_or_default()
	}
}
