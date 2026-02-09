# just-kdl
[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/rsutil/tree/main/just-kdl)
[![Crates.io](https://img.shields.io/crates/v/just-kdl)](https://crates.io/crates/just-kdl)
[![docs.rs](https://img.shields.io/docsrs/just-kdl)](https://docs.rs/just-kdl)
[![MIT OR Apache-2.0](https://img.shields.io/crates/l/just-kdl)](https://github.com/1e1001/rsutil/blob/main/just-kdl/README.md#License)

Implementation of a [KDL] v2.0.1 parser.
```rust
let text = "an example; kdl {document}";
let document = Reader::new(text.as_bytes())
    .collect::<Result<Document, _>>()
    .expect("syntax error");
assert_eq!(document.to_string(), "
an example
kdl {
    document
}
".trim());
```

For more information, [read the docs](https://docs.rs/miny).

## Changelog
### Unreleased
- Remove `hashbrown` by making `Node::normalize` and `Document::normalize` std-gated

### 0.2.0
- Rewrite to be good (but still unstable)

### 0.1.0
- Initial (unstable) release

[KDL]: <https://kdl.dev>
