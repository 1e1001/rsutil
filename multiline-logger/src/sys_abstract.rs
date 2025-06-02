// SPDX-License-Identifier: MIT OR Apache-2.0
//! Common system abstractions
#[cfg(not(feature = "backtrace"))]
use std::marker::PhantomData;
use std::path::Path;

use crate::{Header, Panic, Record};

/// functions required to be implemented by a platform
pub trait SystemImpl {
	type File;
	fn file_new(path: &'static Path) -> Self::File;
	fn file_p_header(this: &Self::File, message: &Header);
	fn file_p_record(this: &Self::File, message: &Record);
	fn file_p_panic(this: &Self::File, message: &Panic);
	fn file_flush(this: &Self::File);
	fn file_path(this: Option<&Self::File>) -> Option<&'static Path>;
	type Console;
	fn console_new() -> Self::Console;
	fn console_p_header(this: &Self::Console, message: &Header);
	fn console_p_record(this: &Self::Console, message: &Record);
	fn console_p_panic(this: &Self::Console, message: &Panic);
	fn console_flush(this: &Self::Console);
	fn fallback_p_panic(message: &Panic);
	#[cfg(feature = "backtrace")]
	type Backtrace;
	#[cfg(feature = "backtrace")]
	fn backtrace_new() -> Self::Backtrace;
	#[cfg(not(feature = "backtrace"))]
	fn backtrace_new() -> PhantomData<Self> { PhantomData }
}

#[cfg(feature = "backtrace")]
pub type MaybeBacktrace<T> = <T as SystemImpl>::Backtrace;
#[cfg(not(feature = "backtrace"))]
pub type MaybeBacktrace<T> = PhantomData<T>;
