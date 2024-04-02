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
