# punch-card

A library for making punched cards like this:

```rust
use punch_card::PunchCard;
fn main() {
    println!("{}", std::str::from_utf8(&(
        .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..,
        ..=..=..=..=..=.. .. ..=..=..=..=..=.. .. ..,
        .. ..=..=..=..=..=..=.. ..=..=..=..=..=.. ..,
        .. .. .. .. .. .. .. ..=.. ..=.. .. .. .. ..,
        ..=.. ..=..=..=..=.. .. ..=.. ..=.. .. ..=..,
        .. ..=..=..=..=..=.. ..=..=.. ..=..=.. .. ..,
        .. .. .. .. ..=.. .. ..=..=..=.. .. .. ..=..,
        .. ..=.. .. ..=.. .. ..=..=.. .. .. ..=.. ..,
  ).punch_card::<Vec<_>>()).unwrap());
}
```

For more information, read the docs.

[Licensed under the MIT license](LICENSE)
