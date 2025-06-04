//! large-scale benchmark to compare the reference `kdl` with `just-kdl`
//! for size reasons, the benchmark files aren't provided in this repository,
//! download them from https://github.com/kdl-org/kdl/tree/main/tests/benchmarks

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Instant;

const HTML_STANDARD: &str = include_str!("html-standard.kdl");
const HTML_COMPACT: &str = include_str!("html-standard-compact.kdl");

fn main() {
	let mode = std::env::args().nth(1).unwrap();
	println!("|Opt.|Parser|Benchmark|Time|Alloc|Resize|Free|Net|");
	println!("|:-|:-|:-|:-|:-|:-|:-|:-|");
	// TODO:: move modes to front, always owned, rerun benchmarks with more battery.
	print!("|{mode}|`kdl-org/kdl`|`html-standard.kdl`");
	run_kdl_rs(HTML_STANDARD);
	print!("|{mode}|`just-kdl`|`html-standard.kdl`");
	run_just_kdl(HTML_STANDARD);
	print!("|{mode}|`kdl-org/kdl`|`html-standard-compact.kdl`");
	run_kdl_rs(HTML_COMPACT);
	print!("|{mode}|`just-kdl`|`html-standard-compact.kdl`");
	run_just_kdl(HTML_COMPACT);
}

struct CounterAlloc {
	alloc: AtomicIsize,
	resize: AtomicIsize,
	free: AtomicIsize,
}

#[global_allocator]
static ALLOC: CounterAlloc = CounterAlloc {
	alloc: AtomicIsize::new(0),
	resize: AtomicIsize::new(0),
	free: AtomicIsize::new(0),
};

impl CounterAlloc {
	fn state(&self) -> (isize, isize, isize) {
		(
			self.alloc.load(Ordering::Relaxed),
			self.resize.load(Ordering::Relaxed),
			self.free.load(Ordering::Relaxed),
		)
	}
}

unsafe impl GlobalAlloc for CounterAlloc {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		self.alloc
			.fetch_add(layout.size() as isize, Ordering::Relaxed);
		unsafe { System.alloc(layout) }
	}
	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		self.free
			.fetch_add(layout.size() as isize, Ordering::Relaxed);
		unsafe { System.dealloc(ptr, layout) }
	}
	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		self.alloc
			.fetch_add(layout.size() as isize, Ordering::Relaxed);
		unsafe { System.alloc_zeroed(layout) }
	}
	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		self.resize.fetch_add(
			new_size as isize - layout.size() as isize,
			Ordering::Relaxed,
		);
		unsafe { System.realloc(ptr, layout, new_size) }
	}
}

fn benchmark<T>(f: impl FnOnce() -> T) {
	fn format_mem(bytes: isize) -> String {
		match bytes {
			1073741824.. => format!("{:.1}GiB", bytes as f64 / 1073741824.0),
			1048576.. => format!("{:.1}MiB", bytes as f64 / 1048576.0),
			1024.. => format!("{:.1}kiB", bytes as f64 / 1024.0),
			_ => format!("{}B", bytes),
		}
	}
	let start_mem = ALLOC.state();
	let start = Instant::now();
	let result = f();
	let end = Instant::now();
	let end_mem = ALLOC.state();
	black_box(result);
	let mem_diff = (
		end_mem.0 - start_mem.0,
		end_mem.1 - start_mem.1,
		end_mem.2 - start_mem.2,
	);
	println!(
		"|{:.03}s|{}|{}|{}|{}|",
		(end - start).as_secs_f64(),
		format_mem(mem_diff.0),
		format_mem(mem_diff.1),
		format_mem(mem_diff.2),
		format_mem(mem_diff.0 + mem_diff.1 - mem_diff.2)
	)
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
		let document = just_kdl::dom::Document::parse(file).unwrap();
		document.into_owned()
	});
}
