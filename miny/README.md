# miny
[![Repository](https://img.shields.io/badge/repository-GitHub-brightgreen.svg)](https://github.com/1e1001/miny)
[![Crates.io](https://img.shields.io/crates/v/miny)](https://crates.io/crates/miny)
[![docs.rs](https://img.shields.io/docsrs/miny)](https://docs.rs/miny)
[![MIT OR Apache-2.0](https://img.shields.io/crates/l/miny)](#LICENSE)

a `Box<T>` with `T` stored inline for values less than a pointer in size.
```rs
let small = Miny::new(1u8);
let large = Miny::new([1usize; 32]);
// small is stored inline on the stack
assert!(small.on_stack());
// large is stored with an allocation
assert!(!large.on_stack());
// consume the miny and get back a value
let original = large.into_inner();
assert_eq!(original, [1; 32]);
```

for more information, [read the docs](https://docs.rs/miny).
