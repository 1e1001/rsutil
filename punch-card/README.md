# punch-card
[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/rsutil/tree/main/punch-card)
[![Crates.io](https://img.shields.io/crates/v/punch-card)](https://crates.io/crates/punch-card)
[![docs.rs](https://img.shields.io/docsrs/punch-card)](https://docs.rs/punch-card)
[![MIT OR Apache-2.0](https://img.shields.io/crates/l/punch-card)](#License)

A library for making punched cards like this:

```rust
use punch_card::PunchCard;

#[rustfmt::skip]
println!("{}", std::str::from_utf8(&(
    .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..,
    ..=..=..=..=..=.. .. .. ..=..=..=..=..=..=.. ..=..=.. ..=..=..=..=..=..=.. ..=..=..=..=..=.. ..=..=..=..=..,
    ..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..=..,
    .. ..=..=..=..=..=.. .. .. ..=.. ..=.. ..=.. .. .. .. .. ..=.. ..=.. ..=.. ..=..=.. .. .. .. .. .. ..=.. ..,
    ..=.. .. .. .. ..=..=..=.. .. .. .. .. .. ..=..=..=..=.. .. .. .. .. .. ..=.. .. ..=.. ..=..=.. .. .. .. ..,
    .. ..=..=.. .. .. ..=..=.. .. .. ..=..=.. ..=.. ..=..=.. .. .. ..=..=.. ..=.. ..=..=.. .. ..=.. .. .. ..=..,
    .. .. .. .. ..=..=..=..=..=..=.. .. .. ..=..=.. ..=..=..=..=.. .. .. ..=..=.. .. ..=..=.. .. ..=.. ..=.. ..,
    .. .. .. .. ..=.. ..=..=..=.. ..=.. ..=..=.. ..=..=..=..=.. ..=.. ..=..=..=.. ..=.. ..=.. ..=..=..=.. .. ..,
).punch_card()).unwrap());
```

For more information, [read the docs](https://docs.rs/punch-card).

## Changelog
### 1.2.0
- Made internals private, technically breaking but I do not care
- `rsutil` merge documentation overhaul

### 1.1.0
- Added `no_std` support
- Better testing and documentation

### 1.0.2
- Added another badge I forgot

### 1.0.1
- Added badges and the like

### 1.0.0
- Added everything
- Fixed metadata

## License
[MIT](../LICENSE-MIT) or [Apache 2.0](../LICENSE-APACHE)
