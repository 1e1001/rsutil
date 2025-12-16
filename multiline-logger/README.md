# multiline-logger
[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/rsutil/tree/main/multiline-logger)
[![Crates.io](https://img.shields.io/crates/v/multiline-logger)](https://crates.io/crates/multiline-logger)
[![docs.rs](https://img.shields.io/docsrs/multiline-logger)](https://docs.rs/multiline-logger)
[![MIT OR Apache-2.0](https://img.shields.io/crates/l/multiline-logger)](https://github.com/1e1001/rsutil/blob/main/multiline-logger/README.md#License)

Fancy lightweight debug output

```rs
fn main() {
	multiline_logger::Settings {
		title: "logger test",
		filters: &[("", LevelFilter::Trace)],
		file_out: Some(Path::new("target/test.log")),
		console_out: true,
		panic_hook: Some(|_| ()),
	}
	.init();
	log::trace!("Trace\n");
	log::debug!("Debug\n{:?}", [1, 2, 3, 4]);
	log::info!("Info: {}", 7);
	log::warn!("Warn {:#?}", [0, 9, 8, 7]);
	log::error!("Error");
	panic!("Panic Message");
}
```

For more information, [read the docs](https://docs.rs/multiline-logger).

## Changelog
### 0.2.1
- Fix Windows support & nightly changes
### 0.2.0
- Turn `panic_hook` into a handler function, user-side panic information is very incomplete
### 0.1.0
- Initial (unstable) release
