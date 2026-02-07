//! large-scale benchmark to compare the reference `kdl` with `just-kdl`
//! for size reasons, the benchmark files aren't provided in this repository,
//! download them from <https://github.com/kdl-org/kdl/tree/main/tests/benchmarks>
#![expect(clippy::print_stdout, reason = "binary")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::env::args;
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use just_kdl::dom::Document;
#[cfg(feature = "std")]
use just_kdl::lexer::ReadInput;
use just_kdl::reader::Reader;

const HTML_STANDARD: &str = include_str!("html-standard.kdl");
const HTML_COMPACT: &str = include_str!("html-standard-compact.kdl");

fn main() {
	let mode = args().nth(1).expect("argv[1] must be {Debug,Release} mode");
	println!("|Opt.|Parser|Benchmark|Time|Alloc|Resize|Free|Net|");
	println!("|:-|:-|:-|:-|:-|:-|:-|:-|");
	print!("|{mode}|`kdl-org/kdl`|`html-standard.kdl`");
	run_kdl_rs(HTML_STANDARD);
	print!("|{mode}|`just-kdl`|`html-standard.kdl`");
	run_just_kdl(HTML_STANDARD);
	print!("|{mode}|`just-kdl` (Read)|`html-standard.kdl`");
	run_just_kdl_read(HTML_STANDARD);
	print!("|{mode}|`kdl-org/kdl`|`html-standard-compact.kdl`");
	run_kdl_rs(HTML_COMPACT);
	print!("|{mode}|`just-kdl`|`html-standard-compact.kdl`");
	run_just_kdl(HTML_COMPACT);
	print!("|{mode}|`just-kdl` (Read)|`html-standard-compact.kdl`");
	run_just_kdl_read(HTML_COMPACT);
}

struct CounterAlloc {
	alloc: AtomicUsize,
	resize: AtomicUsize,
	free: AtomicUsize,
}

#[global_allocator]
static ALLOC: CounterAlloc = CounterAlloc {
	alloc: AtomicUsize::new(0),
	resize: AtomicUsize::new(0),
	free: AtomicUsize::new(0),
};

impl CounterAlloc {
	fn state(&self) -> [usize; 3] {
		[
			self.alloc.load(Ordering::Relaxed),
			self.resize.load(Ordering::Relaxed),
			self.free.load(Ordering::Relaxed),
		]
	}
}

// SAFETY: forwards to System
unsafe impl GlobalAlloc for CounterAlloc {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		self.alloc.fetch_add(layout.size(), Ordering::Relaxed);
		// SAFETY: caller's issue
		unsafe { System.alloc(layout) }
	}
	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		self.free.fetch_add(layout.size(), Ordering::Relaxed);
		// SAFETY: caller's issue
		unsafe { System.dealloc(ptr, layout) }
	}
	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		self.alloc.fetch_add(layout.size(), Ordering::Relaxed);
		// SAFETY: caller's issue
		unsafe { System.alloc_zeroed(layout) }
	}
	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		self.resize
			.fetch_add(layout.size().abs_diff(new_size), Ordering::Relaxed);
		// SAFETY: caller's issue
		unsafe { System.realloc(ptr, layout, new_size) }
	}
}

fn benchmark<T>(f: impl FnOnce() -> T) {
	fn format_mem(bytes: usize) -> String {
		#[expect(clippy::cast_precision_loss, reason = "already a precision loss :)")]
		match bytes {
			1_073_741_824.. => format!("{:.1}GiB", bytes as f64 / 1_073_741_824.0),
			1_048_576.. => format!("{:.1}MiB", bytes as f64 / 1_048_576.0),
			1_024.. => format!("{:.1}kiB", bytes as f64 / 1_024.0),
			_ => format!("{bytes}B"),
		}
	}
	let start_mem = ALLOC.state();
	let start = Instant::now();
	let result = f();
	let end = Instant::now();
	let end_mem = ALLOC.state();
	black_box(result);
	let mem_diff = [
		end_mem[0] - start_mem[0],
		end_mem[1] - start_mem[1],
		end_mem[2] - start_mem[2],
	];
	println!(
		"|{:.03}s|{}|{}|{}|{}|",
		(end - start).as_secs_f64(),
		format_mem(mem_diff[0]),
		format_mem(mem_diff[1]),
		format_mem(mem_diff[2]),
		format_mem(mem_diff[0] + mem_diff[1] - mem_diff[2])
	);
}

fn run_kdl_rs(file: &str) {
	let file = black_box(file);
	benchmark(|| {
		let mut document = kdl::KdlDocument::parse_v2(file).unwrap();
		document.clear_format_recursive();
		document
	});
}

fn run_just_kdl(file: &str) {
	let file = black_box(file);
	benchmark(|| {
		Reader::new(file.as_bytes())
			.collect::<Result<Document, _>>()
			.unwrap()
	});
}

fn run_just_kdl_read(file: &str) {
	#[cfg_attr(not(feature = "std"), expect(unused, reason = "feature-gated"))]
	let file = black_box(file);
	#[cfg(feature = "std")]
	{
		benchmark(|| {
			Reader::new(ReadInput::new(file.as_bytes()))
				.collect::<Result<Document, _>>()
				.unwrap()
		});
	}
}
