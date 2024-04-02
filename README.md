# PFX: A 100% safe, blob-oriented prefix tree

This crate provides a prefix tree map and set data structure, implemented purely in safe Rust.

The API is very similar to `std::collections::{HashMap, BTreeMap}`, including iteration and
an entry API. Iteration proceeds in lexicographical order as determined by the keys.

A notable addition is Prefix search, allowing iteration over all entries whose key starts with
a specified prefix.

## Example

```rust
use pfx::PrefixTreeMap;

fn main() {
    let mut map: PrefixTreeMap<String, u64> = PrefixTreeMap::new();

    map.insert("abc".into(), 123);
    map.insert("def".into(), 456);
    map.insert("defghi".into(), 789);
    
    assert_eq!(map.get("abc").copied(), Some(123));
    assert_eq!(map.get("abcdef").copied(), None);
    assert_eq!(map.get("ab").copied(), None);

    for (key, value) in map.prefix_iter("de") {
        println!("{key} => {value}");
    }
}
```
