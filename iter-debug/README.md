# iter-debug
[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/iter-debug)
[![Crates.io](https://img.shields.io/crates/v/iter-debug)](https://crates.io/crates/iter-debug)
[![docs.rs](https://img.shields.io/docsrs/iter-debug)](https://docs.rs/iter-debug)
[![MIT OR Apache-2.0](https://img.shields.io/crates/l/iter-debug)](#License)

Allows debugging iterators without collecting them to a `Vec` first, such as for `no_std` environments.
```rust
println!("{:?}", [1, 2, 3, 4].map(|v| v * 2).debug());
// => [2, 4, 6, 8]
```

For more information, [read the docs](https://docs.rs/iter-debug).

## Changelog
### 1.0.0
- Initial release

## License
[MIT](./LICENSE-MIT) or [Apache 2.0](./LICENSE-APACHE)
