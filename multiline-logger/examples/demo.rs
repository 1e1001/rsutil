// SPDX-License-Identifier: MIT OR Apache-2.0
//! example log use

use std::path::Path;

use log::LevelFilter;

fn main() {
	multiline_logger::Settings {
		title: "logger test",
		filters: &[("", LevelFilter::Trace)],
		file_out: Some(Path::new("target/test.log")),
		console_out: true,
		panic_hook: true,
	}
	.init();
	log::trace!("Trace\n");
	log::debug!("Debug\n{:?}", [1, 2, 3, 4]);
	log::info!("Info: {}", 7);
	log::warn!("Warn {:#?}", [0, 9, 8, 7]);
	log::error!("Error");
	panic!("Panic Message");
}
