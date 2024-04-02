//! A map from byte strings to arbitrary values, based on a prefix tree.

use core::mem;
use core::iter::FusedIterator;
use core::ops::{Index, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};


/// An ordered map from byte strings to arbitrary values, based on a prefix tree.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PrefixTreeMap<K, V> {
    root: Node<K, V>,
    len: usize,
}

impl<K, V> Default for PrefixTreeMap<K, V> {
    fn default() -> Self {
        PrefixTreeMap::new()
    }
}

impl<K, V> PrefixTreeMap<K, V> {
    /// Creates an empty map. The same as `Default`.
    pub const fn new() -> Self {
        PrefixTreeMap { root: Node::root(), len: 0 }
    }

    /// Returns the number of entries (key-value pairs) in the map.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if and only if this map contains no key-value pairs.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return a reference to the original key and value, if found.
    pub fn get_entry<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        Q: ?Sized + AsRef<[u8]>,
    {
        self.root
            .search(key.as_ref().iter().copied())
            .and_then(Node::item)
    }

    /// Return a reference to the original key and a mutable reference to the value, if found.
    pub fn get_entry_mut<Q>(&mut self, key: &Q) -> Option<(&K, &mut V)>
    where
        Q: ?Sized + AsRef<[u8]>,
    {
        self.root
            .search_mut(key.as_ref().iter().copied())
            .and_then(Node::item_mut)
    }

    /// Return a reference to the value, if found.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: ?Sized + AsRef<[u8]>,
    {
        self.root
            .search(key.as_ref().iter().copied())
            .and_then(Node::value)
    }

    /// Return a mutable reference to the value, if found.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        Q: ?Sized + AsRef<[u8]>,
    {
        self.root
            .search_mut(key.as_ref().iter().copied())
            .and_then(Node::value_mut)
    }

    /// Returns `true` if and only if the given key is found in the map.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + AsRef<[u8]>,
    {
        self.root
            .search(key.as_ref().iter().copied())
            .is_some_and(|node| node.item.is_some())
    }

    /// If the key exists in the map, return the original key and the correpsonding value.
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        Q: ?Sized + AsRef<[u8]>,
    {
        let node = self.root.search_mut(key.as_ref().iter().copied())?;
        let item = node.item.take()?;
        self.len -= 1;
        Some(item)
    }

    /// If the key exists in the map, return the corresponding value.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        Q: ?Sized + AsRef<[u8]>,
    {
        self.remove_entry(key).map(|(_key, value)| value)
    }

    /// An iterator over pairs of references to keys and the corresponding values.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { iter: self.root.iter(), len: self.len }
    }

    /// An iterator over the owned keys.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn into_keys(self) -> IntoKeys<K, V> {
        IntoKeys { iter: self.into_iter() }
    }

    /// An iterator over the borrowed keys.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { iter: self.iter() }
    }

    /// An iterator over the owned values.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn into_values(self) -> IntoValues<K, V> {
        IntoValues { iter: self.into_iter() }
    }

    /// An iterator over the borrowed values.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn values(&self) -> Values<'_, K, V> {
        Values { iter: self.iter() }
    }

    /// An iterator over owned key-value pairs of which the key starts with the given prefix.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn into_prefix_iter<Q>(mut self, prefix: &Q) -> NodeIntoIter<K, V>
    where
        Q: ?Sized + AsRef<[u8]>
    {
        self.root.search_mut(prefix.as_ref().iter().copied()).map_or(
            NodeIntoIter {
                item: None,
                children_iter: Vec::new().into_iter(),
                curr_child_iter: None,
            },
            |node| mem::take(node).into_iter()
        )
    }

    /// An iterator over borrowed key-value pairs of which the key starts with the given prefix.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn prefix_iter<Q>(&self, prefix: &Q) -> NodeIter<'_, K, V>
    where
        Q: ?Sized + AsRef<[u8]>
    {
        self.root.search(prefix.as_ref().iter().copied()).map_or(
            NodeIter {
                item: None,
                children_iter: [].iter(),
                curr_child_iter: None,
            },
            Node::iter
        )
    }

    /// Removes all internal nodes that do not contain an entry.
    ///
    /// This is useful for freeing up memory and speeding up iteration after
    /// removing many key-value pairs from the map and/or after creating many
    /// spurious nodes using the entry API (by not inserting into the nodes
    /// created by `.entry()`).
    pub fn compact(&mut self) {
        self.root.compact();
    }
}

impl<K, V> PrefixTreeMap<K, V>
where
    K: AsRef<[u8]>
{
    /// Return an object representing the (vacant or occupied) node of the tree
    /// corresponding to the given key.
    ///
    /// This always creates a new node, even if you don't end up inserting into
    /// it. Avoid creating many spurious entries, or call [`PrefixTreeMap::compact`]
    /// to remove useless (empty) nodes.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        let node = self.root.search_or_insert(key.as_ref().iter().copied());
        let slot = &mut node.item;
        let len = &mut self.len;

        if slot.is_some() {
            Entry::Occupied(OccupiedEntry { slot, len })
        } else {
            Entry::Vacant(VacantEntry { key, slot, len })
        }
    }

    /// Replaces and returns the previous value, if any.
    ///
    /// This leaves the key in the map untouched if it already exists.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(value);
                None
            }
            Entry::Occupied(mut entry) => Some(entry.insert(value))
        }
    }

    /// Takes the union of `self` with another set of elements.
    /// Elements that already exist in `self` will be overwritten by `other`.
    pub fn union<I>(mut self, other: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        self.union_in_place(other);
        self
    }

    /// Takes the union of `self` with another set of elements.
    /// Elements that already exist in `self` will be overwritten by `other`.
    pub fn union_in_place<I>(&mut self, other: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, value) in other {
            self.insert(key, value);
        }
    }

    /// Takes the intersection of `self` with another set of elements.
    /// The intersection is solely based on the keys.
    pub fn intersection<I>(mut self, other: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        other
            .into_iter()
            .filter_map(|key| self.remove_entry(&key))
            .collect()
    }

    /// Removes the items corresponding to keys in `other` from `self`.
    pub fn difference<I>(mut self, other: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        self.difference_in_place(other);
        self
    }

    /// Removes the items corresponding to keys in `other` from `self`.
    pub fn difference_in_place<I>(&mut self, other: I)
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        for key in other {
            self.remove(&key);
        }
    }

    /// Add elements that are missing from `self`, and remove elements contained in `self`.
    ///
    /// Containment is tested by comparing keys only. Values are not checked for equality.
    pub fn symmetric_difference<I>(mut self, other: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        self.symmetric_difference_in_place(other);
        self
    }

    /// Add elements that are missing from `self`, and remove elements contained in `self`.
    ///
    /// Containment is tested by comparing keys only. Values are not checked for equality.
    pub fn symmetric_difference_in_place<I>(&mut self, other: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, value) in other {
            match self.entry(key) {
                Entry::Occupied(entry) => { entry.remove(); }
                Entry::Vacant(entry) => { entry.insert(value); }
            }
        }
    }
}

impl<K, V, Q> Index<&Q> for PrefixTreeMap<K, V>
where
    K: AsRef<[u8]>,
    Q: ?Sized + AsRef<[u8]>
{
    type Output = V;

    fn index(&self, key: &Q) -> &Self::Output {
        self.get(key).expect("key not found in PrefixTreeMap")
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for PrefixTreeMap<K, V>
where
    K: AsRef<[u8]>
{
    fn from(items: [(K, V); N]) -> Self {
        items.into_iter().collect()
    }
}

impl<K, V> FromIterator<(K, V)> for PrefixTreeMap<K, V>
where
    K: AsRef<[u8]>
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>
    {
        let mut map = PrefixTreeMap::default();
        map.extend(iter);
        map
    }
}

impl<K, V> Extend<(K, V)> for PrefixTreeMap<K, V>
where
    K: AsRef<[u8]>
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (K, V)>
    {
        self.union_in_place(iter);
    }
}

impl<K, V> IntoIterator for PrefixTreeMap<K, V> {
    type IntoIter = IntoIter<K, V>;
    type Item = (K, V);

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.root.into_iter(),
            len: self.len,
        }
    }
}

impl<'a, K, V> IntoIterator for &'a PrefixTreeMap<K, V> {
    type IntoIter = Iter<'a, K, V>;
    type Item = (&'a K, &'a V);

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Creates the intersection of `self` and `other`.
impl<I, K, V> BitAndAssign<I> for PrefixTreeMap<K, V>
where
    I: IntoIterator,
    I::Item: AsRef<[u8]>,
    K: AsRef<[u8]>,
{
    fn bitand_assign(&mut self, other: I) {
        let map = mem::take(self);
        *self = map.intersection(other);
    }
}

/// Creates the union of `self` and `other`.
impl<I, K, V> BitOrAssign<I> for PrefixTreeMap<K, V>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<[u8]>,
{
    fn bitor_assign(&mut self, other: I) {
        self.union_in_place(other);
    }
}

/// Creates the symmetric difference of `self` and `other`.
impl<I, K, V> BitXorAssign<I> for PrefixTreeMap<K, V>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<[u8]>,
{
    fn bitxor_assign(&mut self, other: I) {
        self.symmetric_difference_in_place(other);
    }
}

/// Creates the intersection of `self` and `other`.
impl<I, K, V> BitAnd<I> for PrefixTreeMap<K, V>
where
    I: IntoIterator,
    I::Item: AsRef<[u8]>,
    K: AsRef<[u8]>,
{
    type Output = Self;

    fn bitand(self, other: I) -> Self::Output {
        self.intersection(other)
    }
}

/// Creates the union of `self` and `other`.
impl<I, K, V> BitOr<I> for PrefixTreeMap<K, V>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<[u8]>,
{
    type Output = Self;

    fn bitor(mut self, other: I) -> Self::Output {
        self |= other;
        self
    }
}

/// Creates the symmetric difference of `self` and `other`.
impl<I, K, V> BitXor<I> for PrefixTreeMap<K, V>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<[u8]>,
{
    type Output = Self;

    fn bitxor(mut self, other: I) -> Self::Output {
        self ^= other;
        self
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct Node<K, V> {
    item: Option<(K, V)>,
    key_fragment: u8,
    children: Vec<Node<K, V>>,
}

impl<K, V> Node<K, V> {
    const fn root() -> Self {
        // key of root doesn't matter so we are free to use any value
        Node::with_key_fragment(0)
    }

    const fn with_key_fragment(key_fragment: u8) -> Self {
        Node {
            item: None,
            key_fragment,
            children: Vec::new(),
        }
    }

    /// Deletes leaves/subtrees with only empty nodes. A node is empty
    /// if its item is `None` and all of its children are empty.
    fn compact(&mut self) -> bool {
        let mut has_useful_children = false;

        self.children.retain_mut(|child| {
            let is_useful = child.compact();
            has_useful_children |= is_useful;
            is_useful
        });

        has_useful_children || self.item.is_some()
    }
}

impl<K, V> Node<K, V> {
    fn value(&self) -> Option<&V> {
        self.item.as_ref().map(|(_key, value)| value)
    }

    fn value_mut(&mut self) -> Option<&mut V> {
        self.item.as_mut().map(|(_key, value)| value)
    }

    fn item(&self) -> Option<(&K, &V)> {
        self.item.as_ref().map(|(key, value)| (key, value))
    }

    fn item_mut(&mut self) -> Option<(&K, &mut V)> {
        self.item.as_mut().map(|(key, value)| (&*key, value))
    }

    fn search<B>(&self, mut bytes: B) -> Option<&Self>
    where
        B: Iterator<Item = u8>,
    {
        let Some(byte) = bytes.next() else {
            return Some(self);
        };

        let index = self.children.binary_search_by_key(&byte, |node| node.key_fragment).ok()?;

        self.children[index].search(bytes)
    }

    fn search_mut<B>(&mut self, mut bytes: B) -> Option<&mut Self>
    where
        B: Iterator<Item = u8>,
    {
        let Some(byte) = bytes.next() else {
            return Some(self);
        };

        let index = self.children.binary_search_by_key(&byte, |node| node.key_fragment).ok()?;

        self.children[index].search_mut(bytes)
    }

    fn search_or_insert<B>(&mut self, mut bytes: B) -> &mut Self
    where
        B: Iterator<Item = u8>,
    {
        let Some(byte) = bytes.next() else {
            return self;
        };

        let index = match self.children.binary_search_by_key(&byte, |node| node.key_fragment) {
            Ok(index) => index,
            Err(index) => {
                self.children.insert(index, Node::with_key_fragment(byte));
                index
            }
        };

        self.children[index].search_or_insert(bytes)
    }

    fn into_iter(self) -> NodeIntoIter<K, V> {
        let item = self.item;
        let mut children_iter = self.children.into_iter();
        let curr_child_iter = children_iter.next().map(|node| {
            Box::new(node.into_iter())
        });

        NodeIntoIter {
            item,
            children_iter,
            curr_child_iter,
        }
    }

    fn iter(&self) -> NodeIter<'_, K, V> {
        let item = self.item.as_ref();
        let mut children_iter = self.children.iter();
        let curr_child_iter = children_iter.next().map(|node| {
            Box::new(node.iter())
        });

        NodeIter {
            item,
            children_iter,
            curr_child_iter,
        }
    }
}

/// The default impl returns the same value as `Node::root()`,
/// and its only purpose is to make `mem::take()` work.
impl<K, V> Default for Node<K, V> {
    fn default() -> Self {
        Node::root()
    }
}

/// An entry, representing a vacant or occupied node in the tree,
/// corresponding to a specific key.
///
/// The API is almost exactly the same as that of [`std::collections::btree_map::Entry`].
#[derive(Debug)]
pub enum Entry<'a, K, V> {
    Vacant(VacantEntry<'a, K, V>),
    Occupied(OccupiedEntry<'a, K, V>),
}

impl<'a, K, V> Entry<'a, K, V> {
    pub fn key(&self) -> &K {
        match self {
            Entry::Vacant(entry) => entry.key(),
            Entry::Occupied(entry) => entry.key(),
        }
    }

    pub fn or_insert_with_key<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce(&K) -> V
    {
        match self {
            Entry::Vacant(entry) => {
                let value = default(&entry.key);
                entry.insert(value)
            }
            Entry::Occupied(entry) => entry.into_mut(),
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce() -> V
    {
        self.or_insert_with_key(|_| default())
    }

    // this trips Clippy up for some reason? Clearly I can't just call myself unconditionally...
    #[allow(clippy::unwrap_or_default)]
    pub fn or_default(self) -> &'a mut V
    where
        V: Default
    {
        self.or_insert_with(V::default)
    }

    pub fn or_insert(self, value: V) -> &'a mut V {
        self.or_insert_with_key(|_| value)
    }

    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V)
    {
        if let Entry::Occupied(mut entry) = self {
            f(entry.get_mut());
            Entry::Occupied(entry)
        } else {
            self
        }
    }

    pub fn remove_entry(self) -> Option<(K, V)> {
        if let Entry::Occupied(entry) = self {
            Some(entry.remove_entry())
        } else {
            None
        }
    }

    pub fn remove(self) -> Option<V> {
        if let Entry::Occupied(entry) = self {
            Some(entry.remove())
        } else {
            None
        }
    }
}

/// An entry that does not yet correspond to a value.
#[derive(Debug)]
pub struct VacantEntry<'a, K, V> {
    key: K,
    /// always starts out as `None` upon construction
    slot: &'a mut Option<(K, V)>,
    len: &'a mut usize,
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn insert(self, value: V) -> &'a mut V {
        let (_key, value) = self.slot.insert((self.key, value));
        *self.len += 1;
        value
    }

    pub fn into_key(self) -> K {
        self.key
    }

    pub fn key(&self) -> &K {
        &self.key
    }
}

/// An entry that already contains a value.
#[derive(Debug)]
pub struct OccupiedEntry<'a, K, V> {
    /// always starts out as `Some` upon construction
    slot: &'a mut Option<(K, V)>,
    len: &'a mut usize,
}

impl<'a, K, V> OccupiedEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        &self.slot.as_ref().expect("item in occupied entry").0
    }

    pub fn get(&self) -> &V {
        &self.slot.as_ref().expect("item in occupied entry").1
    }

    pub fn get_mut(&mut self) -> &mut V {
        &mut self.slot.as_mut().expect("item in occupied entry").1
    }

    pub fn into_mut(self) -> &'a mut V {
        &mut self.slot.as_mut().expect("item in occupied entry").1
    }

    /// Replaces the inner value with `value` and returns the old value.
    pub fn insert(&mut self, value: V) -> V {
        mem::replace(self.get_mut(), value)
    }

    pub fn remove_entry(self) -> (K, V) {
        *self.len -= 1;
        self.slot.take().expect("item in occupied entry")
    }

    pub fn remove(self) -> V {
        self.remove_entry().1
    }
}

/// Iterator over an owned subtree.
pub struct NodeIntoIter<K, V> {
    item: Option<(K, V)>,
    children_iter: std::vec::IntoIter<Node<K, V>>,
    curr_child_iter: Option<Box<NodeIntoIter<K, V>>>,
}

impl<K, V> Iterator for NodeIntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        // First, we yield our own item
        if let Some(item) = self.item.take() {
            return Some(item);
        }

        // Failing that (either because there was no value in the first place,
        // or because we already emitted the item), we recurse into our current
        // child.
        if let Some(curr_child_next_item) = self.curr_child_iter.as_mut().and_then(Iterator::next) {
            return Some(curr_child_next_item);
        }

        // Once we exhaused the current child, move on to the next child.
        // If there aren't more children left, terminate the iteration.
        // Otherwise, find the next child with recurse and call next once more, to try again.
        //
        let next_child = self.children_iter.next()?;
        let next_child_into_iter = next_child.into_iter();

        // reuse the allocation if possible
        if let Some(curr_child_iter) = self.curr_child_iter.as_mut() {
            **curr_child_iter = next_child_into_iter;
        } else {
            self.curr_child_iter = Some(Box::new(next_child_into_iter));
        }

        self.next()
    }
}

impl<K, V> FusedIterator for NodeIntoIter<K, V> {}

/// Iterator over a borrowed subtree.
pub struct NodeIter<'a, K, V> {
    item: Option<&'a (K, V)>,
    children_iter: core::slice::Iter<'a, Node<K, V>>,
    curr_child_iter: Option<Box<NodeIter<'a, K, V>>>,
}

impl<'a, K, V> Iterator for NodeIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        // First, we yield our own item
        if let Some((key, value)) = self.item.take() {
            return Some((key, value));
        }

        // Failing that (either because there was no value in the first place,
        // or because we already emitted the item), we recurse into our current
        // child.
        if let Some(curr_child_next_item) = self.curr_child_iter.as_mut().and_then(Iterator::next) {
            return Some(curr_child_next_item);
        }

        // Once we exhaused the current child, move on to the next child.
        // If there aren't more children left, terminate the iteration.
        // Otherwise, find the next child with recurse and call next once more, to try again.
        //
        let next_child = self.children_iter.next()?;
        let next_child_iter = next_child.iter();

        // reuse the allocation if possible
        if let Some(curr_child_iter) = self.curr_child_iter.as_mut() {
            **curr_child_iter = next_child_iter;
        } else {
            self.curr_child_iter = Some(Box::new(next_child_iter));
        }

        self.next()
    }
}

impl<K, V> FusedIterator for NodeIter<'_, K, V> {}

/// Iterator over all the values of the tree.
pub struct IntoIter<K, V> {
    iter: NodeIntoIter<K, V>,
    len: usize,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        self.len -= 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> FusedIterator for IntoIter<K, V> {}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    fn len(&self) -> usize {
        self.len
    }
}

/// Iterator over references to the values of the tree.
pub struct Iter<'a, K, V> {
    iter: NodeIter<'a, K, V>,
    len: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        self.len -= 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<K, V> FusedIterator for Iter<'_, K, V> {}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
    fn len(&self) -> usize {
        self.len
    }
}

/// Iterator over the owned keys.
pub struct IntoKeys<K, V> {
    iter: IntoIter<K, V>,
}

impl<K, V> Iterator for IntoKeys<K, V> {
    type Item = K;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, _v)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<K, V> FusedIterator for IntoKeys<K, V> {}

impl<K, V> ExactSizeIterator for IntoKeys<K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// Iterator over the borrowed keys.
pub struct Keys<'a, K, V> {
    iter: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, _v)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<K, V> FusedIterator for Keys<'_, K, V> {}

impl<K, V> ExactSizeIterator for Keys<'_, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// Iterator over the owned values.
pub struct IntoValues<K, V> {
    iter: IntoIter<K, V>,
}

impl<K, V> Iterator for IntoValues<K, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_k, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<K, V> FusedIterator for IntoValues<K, V> {}

impl<K, V> ExactSizeIterator for IntoValues<K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// Iterator over the borrowed values.
pub struct Values<'a, K, V> {
    iter: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(_k, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<K, V> FusedIterator for Values<'_, K, V> {}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}
