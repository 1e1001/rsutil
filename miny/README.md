# miny
[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/rsutil/tree/main/miny)
[![Crates.io](https://img.shields.io/crates/v/miny)](https://crates.io/crates/miny)
[![docs.rs](https://img.shields.io/docsrs/miny)](https://docs.rs/miny)
[![MIT OR Apache-2.0](https://img.shields.io/crates/l/miny)](#License)

A `Box<T>` with `T` stored inline for values less than a pointer in size. Requires **nightly** Rust & `alloc`
```rust
use miny::Miny;
let small = Miny::new(1_u8);
let large = Miny::new([1_usize; 32]);
// small is stored inline on the stack
assert!(Miny::on_stack(&small));
// large is stored with an allocation
assert!(!Miny::on_stack(&large));
// consume the miny and get back a value
let original = Miny::into_inner(large);
assert_eq!(original, [1; 32]);
```

For more information, [read the docs](https://docs.rs/miny).

## Changelog
### 2.0.3
- Don't try to deallocate ZSTs when converting from a `Box` [(thanks, Cormac!)](https://github.com/1e1001/rsutil/pull/1)
- Documenting invariants more
- `rsutil` merge documentation overhaul

### 2.0.2
- Account for changes in `ptr` API's

### 2.0.1
- Documentation upgrade

### 2.0.0
- Redid the entire library to require qualified syntax, because I realized that that's probably a good idea

### 1.0.0
- Initial release

## License
[MIT](../LICENSE-MIT) or [Apache 2.0](../LICENSE-APACHE)


<sub>(also hi please give me suggestions for more features to add, this crate feels kinda small)</sub>
