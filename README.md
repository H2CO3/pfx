# PFX: A 100% safe, blob-oriented prefix tree

This crate provides a prefix tree map and set data structure, implemented purely in safe Rust.

The API is very similar to `std::collections::{HashMap, BTreeMap}`, including iteration and
an entry API. Iteration proceeds in lexicographical order as determined by the keys.

A notable addition is Prefix search, allowing iteration over all entries whose key starts with
a specified prefix.
