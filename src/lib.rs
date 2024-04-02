#![forbid(unsafe_code)]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", env!("CARGO_PKG_README")))]

pub mod map;
pub mod set;

pub use map::{PrefixTreeMap, Entry, VacantEntry, OccupiedEntry};
pub use set::PrefixTreeSet;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let mut pt: PrefixTreeMap<String, u64> = PrefixTreeMap::new();

        assert!(pt.is_empty());
        assert_eq!(pt.len(), 0);

        assert!(!pt.contains_key("\0")); // the root node has the key 0
        assert!(!pt.contains_key(""));

        assert!(pt.insert("foo".into(), 42).is_none());
        assert!(pt.contains_key("foo"));
        assert_eq!(pt.len(), 1);
        assert_eq!(pt.get("foo").copied(), Some(42));
        assert_eq!(pt.insert("foo".into(), 43), Some(42));
        assert_eq!(pt.len(), 1);

        pt.extend([
            ("bar".into(), 137),
            ("baz".into(), 4224),
        ]);

        assert_eq!(pt.len(), 3);
        assert!(pt.contains_key("bar"));
        assert!(pt.contains_key("baz"));

        assert_eq!(pt.remove("bar"), Some(137));
        assert_eq!(pt.len(), 2);
        assert!(!pt.contains_key("bar"));
        assert!(pt.contains_key("baz"));
        assert_eq!(pt.remove("bar"), None);
        assert_eq!(pt.len(), 2);

        pt.compact();

        assert_eq!(pt.len(), 2);
        assert!(pt.contains_key("baz"));
        assert!(!pt.contains_key("bar"));

        assert_eq!(pt.get_mut("baz").copied(), Some(4224));
        *pt.get_mut("baz").unwrap() = 999;
        assert_eq!(pt["baz"], 999);
    }

    #[test]
    fn insertion_order_does_not_matter() {
        use std::hash::{Hash, Hasher, DefaultHasher};

        let mut strings = [
            ("foo",    1),
            ("bar",    2),
            ("baz",    3),
            ("qux",    4),
            ("abc",    5),
            ("def",    6),
            ("abcdef", 7),
            ("lol",    8),
            ("bazwut", 9),
        ];
        let pt1 = PrefixTreeMap::from(strings);
        let pt2: PrefixTreeMap<&str, u64> = strings.into_iter().rev().collect();

        strings.sort();
        let pt3 = PrefixTreeMap::from(strings);

        assert_eq!(pt1, pt2);
        assert_eq!(pt1, pt3);
        assert_eq!(pt2, pt3);

        assert_eq!(pt1.len(), strings.len());
        assert_eq!(pt2.len(), strings.len());
        assert_eq!(pt3.len(), strings.len());

        // the hashes must be equal regardless of insertion order
        let hashes = [pt1, pt2, pt3].map(|pt| {
            let mut hasher = DefaultHasher::new();
            pt.hash(&mut hasher);
            hasher.finish()
        });

        assert_eq!(
            hashes.iter().min().unwrap(),
            hashes.iter().max().unwrap(),
        );
    }

    #[test]
    fn entry_api() {
        let mut pt = PrefixTreeMap::<[u8; 4], Vec<u32>>::default();

        // since the entry API inserts nodes, double-check
        // that it doesn't accidentally insert spurious values
        assert!(matches!(pt.entry([42, 43, 44, 45]), Entry::Vacant(_)));
        assert!(matches!(pt.entry([42, 43, 44, 45]), Entry::Vacant(_)));


        let val = pt
            .entry([42, 43, 44, 45])
            .and_modify(|_| panic!("and_modify() shouldn't fire for a vacant entry"))
            .or_insert(vec![9, 8, 7]);

        assert_eq!(*val, &[9, 8, 7]);

        val.push(6);
        assert_eq!(*val, &[9, 8, 7, 6]);
        assert_eq!(pt.get(b"*+,-").map(Vec::as_slice), Some([9, 8, 7, 6].as_slice()));

        let empty = pt.entry(*b"wxyz").or_default();
        assert_eq!(empty.len(), 0);

        assert!(pt.entry(*b"nope").remove().is_none());
    }

    #[test]
    fn iteration() {
        let data = [
            ("don", 314),
            ("linus", 1337),
            ("bill", 666),
            ("steve", 1984),
            ("larry", 600613),
            ("lattner", u32::from_le_bytes(*b"LLVM")),
        ];
        let tree = PrefixTreeMap::from(data);

        let keys: Vec<_> = tree.keys().copied().collect();
        assert_eq!(keys, ["bill", "don", "larry", "lattner", "linus", "steve"]);

        let values: Vec<_> = tree.values().copied().collect();
        assert_eq!(values, [666, 314, 600613, 1297501260, 1337, 1984]);

        let mut iter = tree.iter();
        assert_eq!(iter.len(), data.len());

        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert_eq!(iter.len(), data.len() - 2);

        let mut iter = tree.clone().into_iter();
        assert_eq!(iter.len(), data.len());

        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert_eq!(iter.len(), data.len() - 3);

        // Prefix search
        assert_eq!(
            tree.prefix_iter("la").map(|(&k, &v)| (k, v)).collect::<Vec<_>>(),
            [("larry", 600613), ("lattner", 1297501260)],
        );
        assert_eq!(
            tree.clone().into_prefix_iter("l").collect::<Vec<_>>(),
            [("larry", 600613), ("lattner", 1297501260), ("linus", 1337)],
        );

        assert!(tree.prefix_iter("a").next().is_none());
        assert!(tree.prefix_iter("b").next().is_some());
        assert!(tree.prefix_iter("c").next().is_none());

        // the empty prefix should yield the entire tree
        assert!(tree.prefix_iter("").eq(&tree));
        assert!(tree.clone().into_prefix_iter("").eq(tree));
    }

    #[test]
    fn set_operations() {
        let x = PrefixTreeSet::from(["abc", "def", "abc", "qux"]);
        let y = PrefixTreeSet::from(["def", "qux", "what", "4lulz"]);

        assert!(
            x.clone().union(y.clone()).iter().eq(&["4lulz", "abc", "def", "qux", "what"])
        );
        assert_eq!(x.clone().union(x.clone()), x);

        assert!(
            x.clone().intersection(y.clone()).iter().eq(&["def", "qux"])
        );
        assert_eq!(x.clone().intersection(x.clone()), x);

        assert!(
            x.clone().difference(y.clone()).iter().eq(&["abc"])
        );
        assert!(x.clone().difference(x.clone()).is_empty());

        assert!(
            x.clone().symmetric_difference(y.clone()).iter().eq(&["4lulz", "abc", "what"])
        );
        assert!(x.clone().symmetric_difference(x.clone()).is_empty());
    }
}
