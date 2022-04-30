# punch-card

A library for making punched cards like this:

```rs

use punch_card::PunchCard;
fn main() {
    println!("{}", std::str::from_utf8((
        .. .. .. .. .. .. .. .. .. .. .. .. .. .. ..,
        ..=..=..=..=..=.. .. ..=..=..=..=..=.. .. ..,
        .. ..=..=..=..=..=..=.. ..=..=..=..=..=.. ..,
        .. .. .. .. .. .. .. ..=.. ..=.. .. .. .. ..,
        ..=.. ..=..=..=..=.. .. ..=.. ..=.. .. ..=..,
    .. ..=..=..=..=..=.. ..=..=.. ..=..=.. .. ..,
    .. .. .. .. ..=.. .. ..=..=..=.. .. .. ..=..,
    .. ..=.. .. ..=.. .. ..=..=.. .. .. ..=.. ..,
  ).punch_card()).unwrap());
}
```

## Usage

punch-card supports the following sizes of card:

- *w* &times; 1 &rarr; list of `bool`
- *w* &times; 8 &rarr; list of `u8`
- *w* &times; 16 &rarr; list of `u16`
- *w* &times; 32 &rarr; list of `u32`
- *w* &times; 64 &rarr; list of `u64`
- *w* &times; 128 &rarr; list of `u128`

A card is simply a tuple of *h* "lines", where each line is a chain of `..`'s or `..=`'s terminated by a `..`.
