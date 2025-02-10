# punch-card

[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/punch-card)
[![Crates.io](https://img.shields.io/crates/v/punch-card)](https://crates.io/crates/punch-card)
[![docs.rs](https://img.shields.io/docsrs/punch-card)](https://docs.rs/punch-card)
[![MIT](https://img.shields.io/crates/l/punch-card)](./LICENSE)

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

### 1.1.0

- added no_std support
- better testing and documentation

### 1.0.2

- added another badge i forgot

### 1.0.1

- added badges and the like

### 1.0.0

- added everything
- fixed metadata
