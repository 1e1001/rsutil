# multiline-logger
Fancy lightweight debug output

```rs
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
```

For more information, [read the docs](https://docs.rs/multiline-logger).

## Changelog
### 0.1.0
- Initial (unstable) release
