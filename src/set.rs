//! A set of byte strings, based on a prefix tree.

use core::iter::FusedIterator;
use core::fmt::{self, Debug, Formatter};
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};
use crate::map::{PrefixTreeMap, NodeIntoIter, NodeIter, Keys, IntoKeys};


/// An ordered set based on a prefix tree.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrefixTreeSet<T> {
    map: PrefixTreeMap<T, ()>,
}

impl<T> PrefixTreeSet<T> {
    /// Creates an empty set. The same as `Default`.
    pub const fn new() -> Self {
        PrefixTreeSet { map: PrefixTreeMap::new() }
    }

    /// Returns the number of items in this set.
    pub const fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if and only if this set is empty.
    pub const fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Returns `true` if the item is found in the set, `false` otherwise.
    pub fn contains<Q>(&self, item: &Q) -> bool
    where
        Q: ?Sized + AsRef<[u8]>
    {
        self.map.contains_key(item)
    }

    /// Returns `true` iff there are any keys with the given prefix in the set.
    /// This is more efficient than creating a prefix iterator and checking
    /// whether it is empty.
    pub fn contains_prefix<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + AsRef<[u8]>,
    {
        self.map.contains_prefix(key)
    }

    /// Removes a key if it existed. Returns `true` if a removal happened,
    /// and `false` if the key did not exist in the first place.
    pub fn remove<Q>(&mut self, key: &Q) -> bool
    where
        Q: ?Sized + AsRef<[u8]>
    {
        self.map.remove(key).is_some()
    }

    /// Returns an iterator over the borrowed items.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { keys: self.map.keys() }
    }

    /// An iterator over owned keys that start with the given prefix.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn into_prefix_iter<Q>(self, key: &Q) -> IntoPrefixIter<T>
    where
        Q: ?Sized + AsRef<[u8]>
    {
        IntoPrefixIter { iter: self.map.into_prefix_iter(key) }
    }

    /// An iterator over borrowed keys that start with the given prefix.
    ///
    /// Iteration proceeds in lexicographic order, as determined by the byte sequence of keys.
    pub fn prefix_iter<Q>(&self, key: &Q) -> PrefixIter<'_, T>
    where
        Q: ?Sized + AsRef<[u8]>
    {
        PrefixIter { iter: self.map.prefix_iter(key) }
    }

    /// Removes all internal nodes which are not useful.
    /// See the documentation of [`crate::map::PrefixTreeMap::compact`]
    /// for more details on why this is useful.
    pub fn compact(&mut self) {
        self.map.compact();
    }
}

impl<T: AsRef<[u8]>> PrefixTreeSet<T> {
    /// Inserts the key if it did not exist.
    ///
    /// Returns `true` if an insertion happened, and `false` if the key already existed.
    pub fn insert(&mut self, key: T) -> bool {
        self.map.insert(key, ()).is_none()
    }

    /// Takes the union of `self` with another set of elements.
    /// Elements that already exist in `self` will be overwritten by `other`.
    pub fn union<I>(mut self, other: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.union_in_place(other);
        self
    }

    /// Takes the union of `self` with another set of elements.
    /// Elements that already exist in `self` will be overwritten by `other`.
    pub fn union_in_place<I>(&mut self, other: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.map.union_in_place(other.into_iter().map(|item| (item, ())));
    }

    /// Takes the intersection of `self` with another set of elements.
    ///
    /// This takes `&self` by reference and not `self` by value because
    /// computing the intersection always incurs the allocation of a new
    /// set. For the same reason, there is no `intersection_in_place()`
    /// method, either.
    pub fn intersection<I>(&self, other: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        other
            .into_iter()
            .filter(|key| self.contains(key))
            .collect()
    }

    /// Removes the items of `other` from `self`.
    pub fn difference<I>(mut self, other: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        self.difference_in_place(other);
        self
    }

    /// Removes the items of `other` from `self`.
    pub fn difference_in_place<I>(&mut self, other: I)
    where
        I: IntoIterator,
        I::Item: AsRef<[u8]>,
    {
        self.map.difference_in_place(other);
    }

    /// Add elements that are missing from `self`, and remove elements contained in `self`.
    pub fn symmetric_difference<I>(mut self, other: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.symmetric_difference_in_place(other);
        self
    }

    /// Add elements that are missing from `self`, and remove elements contained in `self`.
    pub fn symmetric_difference_in_place<I>(&mut self, other: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.map.symmetric_difference_in_place(other.into_iter().map(|item| (item, ())));
    }
}

impl<T> Default for PrefixTreeSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> From<[T; N]> for PrefixTreeSet<T>
where
    T: AsRef<[u8]>
{
    fn from(items: [T; N]) -> Self {
        items.into_iter().collect()
    }
}

impl<T: AsRef<[u8]>> FromIterator<T> for PrefixTreeSet<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>
    {
        let mut set = PrefixTreeSet::default();
        set.extend(iter);
        set
    }
}

impl<T: AsRef<[u8]>> Extend<T> for PrefixTreeSet<T> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>
    {
        self.union_in_place(iter);
    }
}

impl<T> IntoIterator for PrefixTreeSet<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter { keys: self.map.into_keys() }
    }
}

impl<'a, T> IntoIterator for &'a PrefixTreeSet<T> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Produces the intersection of `self` and `other`.
impl<T, I> BitAndAssign<I> for PrefixTreeSet<T>
where
    T: AsRef<[u8]>,
    I: IntoIterator<Item = T>,
{
    fn bitand_assign(&mut self, other: I) {
        *self = self.intersection(other);
    }
}

/// Produces the union of `self` and `other`.
impl<T, I> BitOrAssign<I> for PrefixTreeSet<T>
where
    T: AsRef<[u8]>,
    I: IntoIterator<Item = T>,
{
    fn bitor_assign(&mut self, other: I) {
        self.union_in_place(other);
    }
}

/// Produces the symmetric difference of `self` and `other`.
impl<T, I> BitXorAssign<I> for PrefixTreeSet<T>
where
    T: AsRef<[u8]>,
    I: IntoIterator<Item = T>,
{
    fn bitxor_assign(&mut self, other: I) {
        self.symmetric_difference_in_place(other);
    }
}

/// Produces the intersection of `self` and `other`.
impl<T, I> BitAnd<I> for PrefixTreeSet<T>
where
    T: AsRef<[u8]>,
    I: IntoIterator<Item = T>,
{
    type Output = Self;

    fn bitand(self, other: I) -> Self::Output {
        self.intersection(other)
    }
}

/// Produces the intersection of `self` and `other`.
impl<T, I> BitAnd<I> for &PrefixTreeSet<T>
where
    T: AsRef<[u8]>,
    I: IntoIterator<Item = T>,
{
    type Output = PrefixTreeSet<T>;

    fn bitand(self, other: I) -> Self::Output {
        self.intersection(other)
    }
}

/// Produces the union of `self` and `other`.
impl<T, I> BitOr<I> for PrefixTreeSet<T>
where
    T: AsRef<[u8]>,
    I: IntoIterator<Item = T>,
{
    type Output = Self;

    fn bitor(self, other: I) -> Self::Output {
        self.union(other)
    }
}

/// Produces the symmetric difference of `self` and `other`.
impl<T, I> BitXor<I> for PrefixTreeSet<T>
where
    T: AsRef<[u8]>,
    I: IntoIterator<Item = T>,
{
    type Output = Self;

    fn bitxor(self, other: I) -> Self::Output {
        self.symmetric_difference(other)
    }
}

impl<T: Debug> Debug for PrefixTreeSet<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self).finish()
    }
}

/// An iterator over the owned items of this set.
#[derive(Debug)]
pub struct IntoIter<T> {
    keys: IntoKeys<T, ()>,
}

impl<T> Default for IntoIter<T> {
    fn default() -> Self {
        IntoIter { keys: IntoKeys::default() }
    }
}

impl<T: Clone> Clone for IntoIter<T> {
    fn clone(&self) -> Self {
        IntoIter { keys: self.keys.clone() }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl<T> FusedIterator for IntoIter<T> {}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.keys.len()
    }
}

/// An iterator over the borrowed items of this set.
#[derive(Debug)]
pub struct Iter<'a, T> {
    keys: Keys<'a, T, ()>,
}

impl<T> Default for Iter<'_, T> {
    fn default() -> Self {
        Iter { keys: Keys::default() }
    }
}

impl<T> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Iter { keys: self.keys.clone() }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.keys.size_hint()
    }
}

impl<T> FusedIterator for Iter<'_, T> {}

impl<T> ExactSizeIterator for Iter<'_, T> {
    fn len(&self) -> usize {
        self.keys.len()
    }
}

/// An iterator over values of a subtree, i.e., a set of elements sharing a common prefix.
#[derive(Debug)]
pub struct IntoPrefixIter<T> {
    iter: NodeIntoIter<T, ()>,
}

impl<T> Default for IntoPrefixIter<T> {
    fn default() -> Self {
        IntoPrefixIter { iter: NodeIntoIter::default() }
    }
}

impl<T: Clone> Clone for IntoPrefixIter<T> {
    fn clone(&self) -> Self {
        IntoPrefixIter { iter: self.iter.clone() }
    }
}

impl<T> Iterator for IntoPrefixIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let (key, ()) = self.iter.next()?;
        Some(key)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> FusedIterator for IntoPrefixIter<T> {}

/// An iterator over references in a subtree, i.e., a set of elements sharing a common prefix.
#[derive(Debug)]
pub struct PrefixIter<'a, T> {
    iter: NodeIter<'a, T, ()>,
}

impl<T> Default for PrefixIter<'_, T> {
    fn default() -> Self {
        PrefixIter { iter: NodeIter::default() }
    }
}

impl<T> Clone for PrefixIter<'_, T> {
    fn clone(&self) -> Self {
        PrefixIter { iter: self.iter.clone() }
    }
}

impl<'a, T> Iterator for PrefixIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let (key, ()) = self.iter.next()?;
        Some(key)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> FusedIterator for PrefixIter<'_, T> {}

#[cfg(feature = "serde")]
#[doc(hidden)]
pub mod serde {
    use core::marker::PhantomData;
    use serde::{
        ser::{Serialize, Serializer},
        de::{Deserialize, Deserializer, Visitor, SeqAccess},
    };
    use crate::set::PrefixTreeSet;


    impl<T: Serialize> Serialize for PrefixTreeSet<T> {
        fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
            ser.collect_seq(self)
        }
    }

    impl<'de, T> Deserialize<'de> for PrefixTreeSet<T>
    where
        T: Deserialize<'de> + AsRef<[u8]>,
    {
        fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
            de.deserialize_seq(PrefixTreeSetVisitor(PhantomData))
        }
    }


    struct PrefixTreeSetVisitor<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for PrefixTreeSetVisitor<T>
    where
        T: Deserialize<'de> + AsRef<[u8]>,
    {
        type Value = PrefixTreeSet<T>;

        fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str("set")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut acc: A) -> Result<Self::Value, A::Error> {
            let mut set = PrefixTreeSet::new();

            while let Some(item) = acc.next_element()? {
                set.insert(item);
            }

            Ok(set)
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::set::PrefixTreeSet;

        #[test]
        fn serde_roundtrip() {
            let orig = PrefixTreeSet::from([
                [1, 3, 5, 7],
                [2, 4, 6, 8],
                [9, 7, 5, 3],
            ]);
            let json = serde_json::to_string_pretty(&orig).unwrap();
            let dupe: PrefixTreeSet<[u8; 4]> = serde_json::from_str(&json).unwrap();

            assert_eq!(orig, dupe);
        }

        #[test]
        fn std_to_pfx() {
            let std_seq = vec![
                *b"abcdef",
                *b"defghi",
                *b"lkjhgf",
                *b"pqrstu",
                *b"uvwxyz",
            ];
            let json = serde_json::to_string_pretty(&std_seq).unwrap();
            let pfx_seq: PrefixTreeSet<[u8; 6]> = serde_json::from_str(&json).unwrap();

            assert!(pfx_seq.iter().eq(&std_seq));
        }

        #[test]
        fn pfx_to_std() {
            let pfx_seq = PrefixTreeSet::from([
                *b"abdef",
                *b"uvxyz",
                *b"pqstu",
                *b"deghi",
                *b"lkhgf",
            ]);
            let json = serde_json::to_string_pretty(&pfx_seq).unwrap();
            let std_seq: Vec<[u8; 5]> = serde_json::from_str(&json).unwrap();

            assert!(std_seq.iter().eq(&pfx_seq));
        }
    }
}
