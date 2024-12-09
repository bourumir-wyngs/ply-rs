use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    iter::FromIterator,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::{Index, IndexMut},
    ptr::{self, NonNull},
};

use hashbrown::hash_map::DefaultHashBuilder;
use hashbrown::hash_table::{self, HashTable};

/// A version of `HashMap` that has a user controllable order for its entries.
///
/// It achieves this by keeping its entries in an internal linked list and using a `HashMap` to
/// point at nodes in this linked list.
///
/// The order of entries defaults to "insertion order", but the user can also modify the order of
/// existing entries by manually moving them to the front or back.
///
/// There are two kinds of methods that modify the order of the internal list:
///
/// * Methods that have names like `to_front` and `to_back` will unsurprisingly move an existing
///   entry to the front or back
/// * Methods that have the word `insert` will insert a new entry ot the back of the list, and if
///   that method might replace an entry, that method will *also move that existing entry to the
///   back*.
pub struct MiniMap<K, V, S = DefaultHashBuilder> {
    table: HashTable<NonNull<Node<K, V>>>,
    // We always need to keep our custom hash builder outside of the HashTable, because it doesn't
    // know how to do any hashing itself.
    hash_builder: S,
    // Circular linked list of nodes.  If `values` is non-null, it will point to a "guard node"
    // which will never have an initialized key or value, `values.prev` will contain the last key /
    // value in the list, `values.next` will contain the first key / value in the list.
    values: Option<NonNull<Node<K, V>>>,
    // *Singly* linked list of free nodes.  The `prev` pointers in the free list should be assumed
    // invalid.
    free: Option<NonNull<Node<K, V>>>,
}

impl<K, V> MiniMap<K, V> {
    #[inline]
    pub fn new() -> Self {
        Self {
            hash_builder: DefaultHashBuilder::default(),
            table: HashTable::new(),
            values: None,
            free: None,
        }
    }

    #[inline]
     fn with_capacity(capacity: usize) -> Self {
        Self {
            hash_builder: DefaultHashBuilder::default(),
            table: HashTable::with_capacity(capacity),
            values: None,
            free: None,
        }
    }
}

impl<K, V, S> MiniMap<K, V, S> {
    #[inline]
    fn with_hasher(hash_builder: S) -> Self {
        Self {
            hash_builder,
            table: HashTable::new(),
            values: None,
            free: None,
        }
    }

    #[inline]
    fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        Self {
            hash_builder,
            table: HashTable::with_capacity(capacity),
            values: None,
            free: None,
        }
    }

    #[inline]
     fn len(&self) -> usize {
        self.table.len()
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
     fn clear(&mut self) {
        self.table.clear();
        if let Some(mut values) = self.values {
            unsafe {
                drop_value_nodes(values);
                values.as_mut().links.value = ValueLinks {
                    prev: values,
                    next: values,
                };
            }
        }
    }

    #[inline]
    pub(crate) fn iter(&self) -> Iter<K, V> {
        let (head, tail) = if let Some(values) = self.values {
            unsafe {
                let ValueLinks { next, prev } = values.as_ref().links.value;
                (next.as_ptr(), prev.as_ptr())
            }
        } else {
            (ptr::null_mut(), ptr::null_mut())
        };

        Iter {
            head,
            tail,
            remaining: self.len(),
            marker: PhantomData,
        }
    }

    #[inline]
     fn iter_mut(&mut self) -> IterMut<K, V> {
        let (head, tail) = if let Some(values) = self.values {
            unsafe {
                let ValueLinks { next, prev } = values.as_ref().links.value;
                (Some(next), Some(prev))
            }
        } else {
            (None, None)
        };

        IterMut {
            head,
            tail,
            remaining: self.len(),
            marker: PhantomData,
        }
    }

    #[inline]
    fn front(&self) -> Option<(&K, &V)> {
        if self.is_empty() {
            return None;
        }
        unsafe {
            let front = (*self.values.as_ptr()).links.value.next.as_ptr();
            let (key, value) = (*front).entry_ref();
            Some((key, value))
        }
    }

    #[inline]
    fn back(&self) -> Option<(&K, &V)> {
        if self.is_empty() {
            return None;
        }
        unsafe {
            let back = &*(*self.values.as_ptr()).links.value.prev.as_ptr();
            let (key, value) = (*back).entry_ref();
            Some((key, value))
        }
    }

    #[inline]
    fn hasher(&self) -> &S {
        &self.hash_builder
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.table.capacity()
    }
}

impl<K, V, S> MiniMap<K, V, S>
where
    K: Eq + Hash,
    S: BuildHasher,
{
    #[inline]
     fn entry(&mut self, key: K) -> Entry<'_, K, V, S> {
        match self.raw_entry_mut().from_key(&key) {
            RawEntryMut::Occupied(occupied) => Entry::Occupied(OccupiedEntry {
                key,
                raw_entry: occupied,
            }),
            RawEntryMut::Vacant(vacant) => Entry::Vacant(VacantEntry {
                key,
                raw_entry: vacant,
            }),
        }
    }

    #[inline]
    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.raw_entry().from_key(k).map(|(_, v)| v)
    }

    #[inline]
     fn get_key_value<Q>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.raw_entry().from_key(k)
    }

    #[inline]
    pub(crate) fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(k).is_some()
    }

    #[inline]
    pub(crate) fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match self.raw_entry_mut().from_key(k) {
            RawEntryMut::Occupied(occupied) => Some(occupied.into_mut()),
            RawEntryMut::Vacant(_) => None,
        }
    }

    /// Inserts the given key / value pair at the *back* of the internal linked list.
    ///
    /// Returns the previously set value, if one existed prior to this call.  After this call,
    /// calling `LinkedHashMap::back` will return a reference to this key / value pair.
    #[inline]
    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        match self.raw_entry_mut().from_key(&k) {
            RawEntryMut::Occupied(mut occupied) => {
                occupied.to_back();
                Some(occupied.replace_value(v))
            }
            RawEntryMut::Vacant(vacant) => {
                vacant.insert(k, v);
                None
            }
        }
    }

    /// If the given key is not in this map, inserts the key / value pair at the *back* of the
    /// internal linked list and returns `None`, otherwise, replaces the existing value with the
    /// given value *without* moving the entry in the internal linked list and returns the previous
    /// value.
    #[inline]
     fn replace(&mut self, k: K, v: V) -> Option<V> {
        match self.raw_entry_mut().from_key(&k) {
            RawEntryMut::Occupied(mut occupied) => Some(occupied.replace_value(v)),
            RawEntryMut::Vacant(vacant) => {
                vacant.insert(k, v);
                None
            }
        }
    }

    #[inline]
    fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match self.raw_entry_mut().from_key(k) {
            RawEntryMut::Occupied(occupied) => Some(occupied.remove()),
            RawEntryMut::Vacant(_) => None,
        }
    }

    #[inline]
    fn remove_entry<Q>(&mut self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match self.raw_entry_mut().from_key(k) {
            RawEntryMut::Occupied(occupied) => Some(occupied.remove_entry()),
            RawEntryMut::Vacant(_) => None,
        }
    }

    #[inline]
    fn pop_front(&mut self) -> Option<(K, V)> {
        if self.is_empty() {
            return None;
        }
        unsafe {
            let front = (*self.values.as_ptr()).links.value.next;
            let hash = hash_node(&self.hash_builder, front);
            match self
                .raw_entry_mut()
                .from_hash(hash, |k| k.eq(front.as_ref().key_ref()))
            {
                RawEntryMut::Occupied(occupied) => Some(occupied.remove_entry()),
                RawEntryMut::Vacant(_) => None,
            }
        }
    }

    #[inline]
    pub(crate) fn pop_back(&mut self) -> Option<(K, V)> {
        if self.is_empty() {
            return None;
        }
        unsafe {
            let back = (*self.values.as_ptr()).links.value.prev;
            let hash = hash_node(&self.hash_builder, back);
            match self
                .raw_entry_mut()
                .from_hash(hash, |k| k.eq(back.as_ref().key_ref()))
            {
                RawEntryMut::Occupied(occupied) => Some(occupied.remove_entry()),
                RawEntryMut::Vacant(_) => None,
            }
        }
    }

    /// If an entry with this key exists, move it to the front of the list and return a reference to
    /// the value.
    #[inline]
    fn to_front<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match self.raw_entry_mut().from_key(k) {
            RawEntryMut::Occupied(mut occupied) => {
                occupied.to_front();
                Some(occupied.into_mut())
            }
            RawEntryMut::Vacant(_) => None,
        }
    }

    /// If an entry with this key exists, move it to the back of the list and return a reference to
    /// the value.
    #[inline]
    fn to_back<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match self.raw_entry_mut().from_key(k) {
            RawEntryMut::Occupied(mut occupied) => {
                occupied.to_back();
                Some(occupied.into_mut())
            }
            RawEntryMut::Vacant(_) => None,
        }
    }

    #[inline]
     fn reserve(&mut self, additional: usize) {
        let hash_builder = &self.hash_builder;
        self.table
            .reserve(additional, move |&n| unsafe { hash_node(hash_builder, n) });
    }

    #[inline]
     fn shrink_to_fit(&mut self) {
        let hash_builder = &self.hash_builder;
        unsafe {
            self.table
                .shrink_to_fit(move |&n| hash_node(hash_builder, n));
            drop_free_nodes(self.free.take());
        }
    }
}

impl<K, V, S> MiniMap<K, V, S>
where
    S: BuildHasher,
{
    #[inline]
     fn raw_entry(&self) -> RawEntryBuilder<'_, K, V, S> {
        RawEntryBuilder { map: self }
    }

    #[inline]
     fn raw_entry_mut(&mut self) -> RawEntryBuilderMut<'_, K, V, S> {
        RawEntryBuilderMut { map: self }
    }
}

impl<K, V, S> Default for MiniMap<K, V, S>
where
    S: Default,
{
    #[inline]
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K: Hash + Eq, V, S: BuildHasher + Default> FromIterator<(K, V)> for MiniMap<K, V, S> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut map = Self::with_capacity_and_hasher(iter.size_hint().0, S::default());
        map.extend(iter);
        map
    }
}

impl<K, V, S> fmt::Debug for MiniMap<K, V, S>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self).finish()
    }
}

impl<K: Hash + Eq, V: PartialEq, S: BuildHasher> PartialEq for MiniMap<K, V, S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other)
    }
}

impl<K: Hash + Eq, V: Eq, S: BuildHasher> Eq for MiniMap<K, V, S> {}

impl<K: Hash + Eq + PartialOrd, V: PartialOrd, S: BuildHasher> PartialOrd
    for MiniMap<K, V, S>
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other)
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        self.iter().lt(other)
    }

    #[inline]
    fn le(&self, other: &Self) -> bool {
        self.iter().le(other)
    }

    #[inline]
    fn ge(&self, other: &Self) -> bool {
        self.iter().ge(other)
    }

    #[inline]
    fn gt(&self, other: &Self) -> bool {
        self.iter().gt(other)
    }
}

impl<K: Hash + Eq + Ord, V: Ord, S: BuildHasher> Ord for MiniMap<K, V, S> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other)
    }
}

impl<K: Hash + Eq, V: Hash, S: BuildHasher> Hash for MiniMap<K, V, S> {
    #[inline]
    fn hash<H: Hasher>(&self, h: &mut H) {
        for e in self.iter() {
            e.hash(h);
        }
    }
}

impl<K, V, S> Drop for MiniMap<K, V, S> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if let Some(values) = self.values {
                drop_value_nodes(values);
                let _ = Box::from_raw(values.as_ptr());
            }
            drop_free_nodes(self.free);
        }
    }
}

unsafe impl<K: Send, V: Send, S: Send> Send for MiniMap<K, V, S> {}
unsafe impl<K: Sync, V: Sync, S: Sync> Sync for MiniMap<K, V, S> {}

impl<'a, K, V, S, Q> Index<&'a Q> for MiniMap<K, V, S>
where
    K: Hash + Eq + Borrow<Q>,
    S: BuildHasher,
    Q: Eq + Hash + ?Sized,
{
    type Output = V;

    #[inline]
    fn index(&self, index: &'a Q) -> &V {
        self.get(index).expect("no entry found for key")
    }
}

impl<'a, K, V, S, Q> IndexMut<&'a Q> for MiniMap<K, V, S>
where
    K: Hash + Eq + Borrow<Q>,
    S: BuildHasher,
    Q: Eq + Hash + ?Sized,
{
    #[inline]
    fn index_mut(&mut self, index: &'a Q) -> &mut V {
        self.get_mut(index).expect("no entry found for key")
    }
}

impl<K: Hash + Eq + Clone, V: Clone, S: BuildHasher + Clone> Clone for MiniMap<K, V, S> {
    #[inline]
    fn clone(&self) -> Self {
        let mut map = Self::with_hasher(self.hash_builder.clone());
        map.extend(self.iter().map(|(k, v)| (k.clone(), v.clone())));
        map
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for MiniMap<K, V, S> {
    #[inline]
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<'a, K, V, S> Extend<(&'a K, &'a V)> for MiniMap<K, V, S>
where
    K: 'a + Hash + Eq + Copy,
    V: 'a + Copy,
    S: BuildHasher,
{
    #[inline]
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        for (&k, &v) in iter {
            self.insert(k, v);
        }
    }
}

 enum Entry<'a, K, V, S> {
    Occupied(OccupiedEntry<'a, K, V, S>),
    Vacant(VacantEntry<'a, K, V, S>),
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for Entry<'_, K, V, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Entry::Vacant(ref v) => f.debug_tuple("Entry").field(v).finish(),
            Entry::Occupied(ref o) => f.debug_tuple("Entry").field(o).finish(),
        }
    }
}

impl<'a, K, V, S> Entry<'a, K, V, S> {
    /// If this entry is vacant, inserts a new entry with the given value and returns a reference to
    /// it.
    ///
    /// If this entry is occupied, this method *moves the occupied entry to the back of the internal
    /// linked list* and returns a reference to the existing value.
    #[inline]
     fn or_insert(self, default: V) -> &'a mut V
    where
        K: Hash,
        S: BuildHasher,
    {
        match self {
            Entry::Occupied(mut entry) => {
                entry.to_back();
                entry.into_mut()
            }
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Similar to `Entry::or_insert`, but accepts a function to construct a new value if this entry
    /// is vacant.
    #[inline]
     fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V
    where
        K: Hash,
        S: BuildHasher,
    {
        match self {
            Entry::Occupied(mut entry) => {
                entry.to_back();
                entry.into_mut()
            }
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    #[inline]
     fn key(&self) -> &K {
        match *self {
            Entry::Occupied(ref entry) => entry.key(),
            Entry::Vacant(ref entry) => entry.key(),
        }
    }

    #[inline]
     fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }
}

 struct OccupiedEntry<'a, K, V, S> {
    key: K,
    raw_entry: RawOccupiedEntryMut<'a, K, V, S>,
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for OccupiedEntry<'_, K, V, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}

impl<'a, K, V, S> OccupiedEntry<'a, K, V, S> {
    #[inline]
     fn key(&self) -> &K {
        self.raw_entry.key()
    }

    #[inline]
     fn remove_entry(self) -> (K, V) {
        self.raw_entry.remove_entry()
    }

    #[inline]
     fn get(&self) -> &V {
        self.raw_entry.get()
    }

    #[inline]
     fn get_mut(&mut self) -> &mut V {
        self.raw_entry.get_mut()
    }

    #[inline]
     fn into_mut(self) -> &'a mut V {
        self.raw_entry.into_mut()
    }

    #[inline]
     fn to_back(&mut self) {
        self.raw_entry.to_back()
    }

    #[inline]
     fn to_front(&mut self) {
        self.raw_entry.to_front()
    }

    /// Replaces this entry's value with the provided value.
    ///
    /// Similarly to `LinkedHashMap::insert`, this moves the existing entry to the back of the
    /// internal linked list.
    #[inline]
     fn insert(&mut self, value: V) -> V {
        self.raw_entry.to_back();
        self.raw_entry.replace_value(value)
    }

    #[inline]
     fn remove(self) -> V {
        self.raw_entry.remove()
    }

    /// Similar to `OccupiedEntry::replace_entry`, but *does* move the entry to the back of the
    /// internal linked list.
    #[inline]
     fn insert_entry(mut self, value: V) -> (K, V) {
        self.raw_entry.to_back();
        self.replace_entry(value)
    }

    /// Replaces the entry's key with the key provided to `LinkedHashMap::entry`, and replaces the
    /// entry's value with the given `value` parameter.
    ///
    /// Does *not* move the entry to the back of the internal linked list.
     fn replace_entry(mut self, value: V) -> (K, V) {
        let old_key = mem::replace(self.raw_entry.key_mut(), self.key);
        let old_value = mem::replace(self.raw_entry.get_mut(), value);
        (old_key, old_value)
    }

    /// Replaces this entry's key with the key provided to `LinkedHashMap::entry`.
    ///
    /// Does *not* move the entry to the back of the internal linked list.
    #[inline]
     fn replace_key(mut self) -> K {
        mem::replace(self.raw_entry.key_mut(), self.key)
    }
}

 struct VacantEntry<'a, K, V, S> {
    key: K,
    raw_entry: RawVacantEntryMut<'a, K, V, S>,
}

impl<K: fmt::Debug, V, S> fmt::Debug for VacantEntry<'_, K, V, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VacantEntry").field(self.key()).finish()
    }
}

impl<'a, K, V, S> VacantEntry<'a, K, V, S> {
    #[inline]
     fn key(&self) -> &K {
        &self.key
    }

    #[inline]
     fn into_key(self) -> K {
        self.key
    }

    /// Insert's the key for this vacant entry paired with the given value as a new entry at the
    /// *back* of the internal linked list.
    #[inline]
     fn insert(self, value: V) -> &'a mut V
    where
        K: Hash,
        S: BuildHasher,
    {
        self.raw_entry.insert(self.key, value).1
    }
}

 struct RawEntryBuilder<'a, K, V, S> {
    map: &'a MiniMap<K, V, S>,
}

impl<'a, K, V, S> RawEntryBuilder<'a, K, V, S>
where
    S: BuildHasher,
{
    #[inline]
     fn from_key<Q>(self, k: &Q) -> Option<(&'a K, &'a V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = hash_key(&self.map.hash_builder, k);
        self.from_key_hashed_nocheck(hash, k)
    }

    #[inline]
     fn from_key_hashed_nocheck<Q>(self, hash: u64, k: &Q) -> Option<(&'a K, &'a V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.from_hash(hash, move |o| k.eq(o.borrow()))
    }

    #[inline]
     fn from_hash(
        self,
        hash: u64,
        mut is_match: impl FnMut(&K) -> bool,
    ) -> Option<(&'a K, &'a V)> {
        unsafe {
            let node = self
                .map
                .table
                .find(hash, move |k| is_match((*k).as_ref().key_ref()))?;

            let (key, value) = (*node.as_ptr()).entry_ref();
            Some((key, value))
        }
    }
}

unsafe impl<'a, K, V, S> Send for RawEntryBuilder<'a, K, V, S>
where
    K: Send,
    V: Send,
    S: Send,
{
}

unsafe impl<'a, K, V, S> Sync for RawEntryBuilder<'a, K, V, S>
where
    K: Sync,
    V: Sync,
    S: Sync,
{
}

 struct RawEntryBuilderMut<'a, K, V, S> {
    map: &'a mut MiniMap<K, V, S>,
}

impl<'a, K, V, S> RawEntryBuilderMut<'a, K, V, S>
where
    S: BuildHasher,
{
    #[inline]
     fn from_key<Q>(self, k: &Q) -> RawEntryMut<'a, K, V, S>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = hash_key(&self.map.hash_builder, k);
        self.from_key_hashed_nocheck(hash, k)
    }

    #[inline]
     fn from_key_hashed_nocheck<Q>(self, hash: u64, k: &Q) -> RawEntryMut<'a, K, V, S>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.from_hash(hash, move |o| k.eq(o.borrow()))
    }

    #[inline]
     fn from_hash(
        self,
        hash: u64,
        mut is_match: impl FnMut(&K) -> bool,
    ) -> RawEntryMut<'a, K, V, S> {
        let entry = self
            .map
            .table
            .find_entry(hash, move |k| is_match(unsafe { (*k).as_ref().key_ref() }));

        match entry {
            Ok(occupied) => RawEntryMut::Occupied(RawOccupiedEntryMut {
                hash_builder: &self.map.hash_builder,
                free: &mut self.map.free,
                values: &mut self.map.values,
                entry: occupied,
            }),
            Err(absent) => RawEntryMut::Vacant(RawVacantEntryMut {
                hash_builder: &self.map.hash_builder,
                values: &mut self.map.values,
                free: &mut self.map.free,
                entry: absent,
            }),
        }
    }
}

unsafe impl<'a, K, V, S> Send for RawEntryBuilderMut<'a, K, V, S>
where
    K: Send,
    V: Send,
    S: Send,
{
}

unsafe impl<'a, K, V, S> Sync for RawEntryBuilderMut<'a, K, V, S>
where
    K: Sync,
    V: Sync,
    S: Sync,
{
}

 enum RawEntryMut<'a, K, V, S> {
    Occupied(RawOccupiedEntryMut<'a, K, V, S>),
    Vacant(RawVacantEntryMut<'a, K, V, S>),
}

impl<'a, K, V, S> RawEntryMut<'a, K, V, S> {
    /// Similarly to `Entry::or_insert`, if this entry is occupied, it will move the existing entry
    /// to the back of the internal linked list.
    #[inline]
     fn or_insert(self, default_key: K, default_val: V) -> (&'a mut K, &'a mut V)
    where
        K: Hash,
        S: BuildHasher,
    {
        match self {
            RawEntryMut::Occupied(mut entry) => {
                entry.to_back();
                entry.into_key_value()
            }
            RawEntryMut::Vacant(entry) => entry.insert(default_key, default_val),
        }
    }

    /// Similarly to `Entry::or_insert_with`, if this entry is occupied, it will move the existing
    /// entry to the back of the internal linked list.
    #[inline]
     fn or_insert_with<F>(self, default: F) -> (&'a mut K, &'a mut V)
    where
        F: FnOnce() -> (K, V),
        K: Hash,
        S: BuildHasher,
    {
        match self {
            RawEntryMut::Occupied(mut entry) => {
                entry.to_back();
                entry.into_key_value()
            }
            RawEntryMut::Vacant(entry) => {
                let (k, v) = default();
                entry.insert(k, v)
            }
        }
    }

    #[inline]
     fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut K, &mut V),
    {
        match self {
            RawEntryMut::Occupied(mut entry) => {
                {
                    let (k, v) = entry.get_key_value_mut();
                    f(k, v);
                }
                RawEntryMut::Occupied(entry)
            }
            RawEntryMut::Vacant(entry) => RawEntryMut::Vacant(entry),
        }
    }
}

 struct RawOccupiedEntryMut<'a, K, V, S> {
    hash_builder: &'a S,
    free: &'a mut Option<NonNull<Node<K, V>>>,
    values: &'a mut Option<NonNull<Node<K, V>>>,
    entry: hash_table::OccupiedEntry<'a, NonNull<Node<K, V>>>,
}

impl<'a, K, V, S> RawOccupiedEntryMut<'a, K, V, S> {
    #[inline]
     fn key(&self) -> &K {
        self.get_key_value().0
    }

    #[inline]
     fn key_mut(&mut self) -> &mut K {
        self.get_key_value_mut().0
    }

    #[inline]
     fn into_key(self) -> &'a mut K {
        self.into_key_value().0
    }

    #[inline]
     fn get(&self) -> &V {
        self.get_key_value().1
    }

    #[inline]
     fn get_mut(&mut self) -> &mut V {
        self.get_key_value_mut().1
    }

    #[inline]
     fn into_mut(self) -> &'a mut V {
        self.into_key_value().1
    }

    #[inline]
     fn get_key_value(&self) -> (&K, &V) {
        unsafe {
            let node = *self.entry.get();
            let (key, value) = (*node.as_ptr()).entry_ref();
            (key, value)
        }
    }

    #[inline]
     fn get_key_value_mut(&mut self) -> (&mut K, &mut V) {
        unsafe {
            let node = *self.entry.get_mut();
            let (key, value) = (*node.as_ptr()).entry_mut();
            (key, value)
        }
    }

    #[inline]
     fn into_key_value(self) -> (&'a mut K, &'a mut V) {
        unsafe {
            let node = *self.entry.into_mut();
            let (key, value) = (*node.as_ptr()).entry_mut();
            (key, value)
        }
    }

    #[inline]
     fn to_back(&mut self) {
        unsafe {
            let node = *self.entry.get_mut();
            detach_node(node);
            attach_before(node, NonNull::new_unchecked(self.values.as_ptr()));
        }
    }

    #[inline]
     fn to_front(&mut self) {
        unsafe {
            let node = *self.entry.get_mut();
            detach_node(node);
            attach_before(node, (*self.values.as_ptr()).links.value.next);
        }
    }

    #[inline]
     fn replace_value(&mut self, value: V) -> V {
        unsafe {
            let mut node = *self.entry.get_mut();
            mem::replace(&mut node.as_mut().entry_mut().1, value)
        }
    }

    #[inline]
     fn replace_key(&mut self, key: K) -> K {
        unsafe {
            let mut node = *self.entry.get_mut();
            mem::replace(&mut node.as_mut().entry_mut().0, key)
        }
    }

    #[inline]
     fn remove(self) -> V {
        self.remove_entry().1
    }

    #[inline]
     fn remove_entry(self) -> (K, V) {
        let node = self.entry.remove().0;
        unsafe { remove_node(self.free, node) }
    }
}

 struct RawVacantEntryMut<'a, K, V, S> {
    hash_builder: &'a S,
    values: &'a mut Option<NonNull<Node<K, V>>>,
    free: &'a mut Option<NonNull<Node<K, V>>>,
    entry: hash_table::AbsentEntry<'a, NonNull<Node<K, V>>>,
}

impl<'a, K, V, S> RawVacantEntryMut<'a, K, V, S> {
    #[inline]
     fn insert(self, key: K, value: V) -> (&'a mut K, &'a mut V)
    where
        K: Hash,
        S: BuildHasher,
    {
        let hash = hash_key(self.hash_builder, &key);
        self.insert_hashed_nocheck(hash, key, value)
    }

    #[inline]
     fn insert_hashed_nocheck(self, hash: u64, key: K, value: V) -> (&'a mut K, &'a mut V)
    where
        K: Hash,
        S: BuildHasher,
    {
        let hash_builder = self.hash_builder;
        self.insert_with_hasher(hash, key, value, |k| hash_key(hash_builder, k))
    }

    #[inline]
     fn insert_with_hasher(
        self,
        hash: u64,
        key: K,
        value: V,
        hasher: impl Fn(&K) -> u64,
    ) -> (&'a mut K, &'a mut V)
    where
        S: BuildHasher,
    {
        unsafe {
            ensure_guard_node(self.values);
            let mut new_node = allocate_node(self.free);
            new_node.as_mut().put_entry((key, value));
            attach_before(new_node, NonNull::new_unchecked(self.values.as_ptr()));

            let node = self
                .entry
                .into_table()
                .insert_unique(hash, new_node, move |k| hasher((*k).as_ref().key_ref()))
                .into_mut();

            let (key, value) = (*node.as_ptr()).entry_mut();
            (key, value)
        }
    }
}

impl<K, V, S> fmt::Debug for RawEntryBuilderMut<'_, K, V, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawEntryBuilder").finish()
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for RawEntryMut<'_, K, V, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            RawEntryMut::Vacant(ref v) => f.debug_tuple("RawEntry").field(v).finish(),
            RawEntryMut::Occupied(ref o) => f.debug_tuple("RawEntry").field(o).finish(),
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for RawOccupiedEntryMut<'_, K, V, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawOccupiedEntryMut")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}

impl<K, V, S> fmt::Debug for RawVacantEntryMut<'_, K, V, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawVacantEntryMut").finish()
    }
}

impl<K, V, S> fmt::Debug for RawEntryBuilder<'_, K, V, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawEntryBuilder").finish()
    }
}

unsafe impl<'a, K, V, S> Send for RawOccupiedEntryMut<'a, K, V, S>
where
    K: Send,
    V: Send,
    S: Send,
{
}

unsafe impl<'a, K, V, S> Sync for RawOccupiedEntryMut<'a, K, V, S>
where
    K: Sync,
    V: Sync,
    S: Sync,
{
}

unsafe impl<'a, K, V, S> Send for RawVacantEntryMut<'a, K, V, S>
where
    K: Send,
    V: Send,
    S: Send,
{
}

unsafe impl<'a, K, V, S> Sync for RawVacantEntryMut<'a, K, V, S>
where
    K: Sync,
    V: Sync,
    S: Sync,
{
}

pub struct Iter<'a, K, V> {
    head: *const Node<K, V>,
    tail: *const Node<K, V>,
    remaining: usize,
    marker: PhantomData<(&'a K, &'a V)>,
}

pub struct IterMut<'a, K, V> {
    head: Option<NonNull<Node<K, V>>>,
    tail: Option<NonNull<Node<K, V>>>,
    remaining: usize,
    marker: PhantomData<(&'a K, &'a mut V)>,
}

 pub struct IntoIter<K, V> {
    head: Option<NonNull<Node<K, V>>>,
    tail: Option<NonNull<Node<K, V>>>,
    remaining: usize,
    marker: PhantomData<(K, V)>,
}

impl<K, V> IterMut<'_, K, V> {
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            head: self.head.as_ptr(),
            tail: self.tail.as_ptr(),
            remaining: self.remaining,
            marker: PhantomData,
        }
    }
}

impl<K, V> IntoIter<K, V> {
    fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            head: self.head.as_ptr(),
            tail: self.tail.as_ptr(),
            remaining: self.remaining,
            marker: PhantomData,
        }
    }
}

unsafe impl<'a, K, V> Send for Iter<'a, K, V>
where
    K: Send,
    V: Send,
{
}

unsafe impl<'a, K, V> Send for IterMut<'a, K, V>
where
    K: Send,
    V: Send,
{
}

unsafe impl<K, V> Send for IntoIter<K, V>
where
    K: Send,
    V: Send,
{
}

unsafe impl<'a, K, V> Sync for Iter<'a, K, V>
where
    K: Sync,
    V: Sync,
{
}

unsafe impl<'a, K, V> Sync for IterMut<'a, K, V>
where
    K: Sync,
    V: Sync,
{
}

unsafe impl<K, V> Sync for IntoIter<K, V>
where
    K: Sync,
    V: Sync,
{
}

impl<'a, K, V> Clone for Iter<'a, K, V> {
    #[inline]
    fn clone(&self) -> Self {
        Iter { ..*self }
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for Iter<'_, K, V> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<K, V> fmt::Debug for IterMut<'_, K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<K, V> fmt::Debug for IntoIter<K, V>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            unsafe {
                let (key, value) = (*self.head).entry_ref();
                self.head = (*self.head).links.value.next.as_ptr();
                Some((key, value))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    #[inline]
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            unsafe {
                let head = self.head.as_ptr();
                let (key, value) = (*head).entry_mut();
                self.head = Some((*head).links.value.next);
                Some((key, value))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        unsafe {
            let head = self.head.as_ptr();
            self.head = Some((*head).links.value.next);
            let mut e = Box::from_raw(head);
            Some(e.take_entry())
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(&'a K, &'a V)> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            unsafe {
                let tail = self.tail;
                self.tail = (*tail).links.value.prev.as_ptr();
                let (key, value) = (*tail).entry_ref();
                Some((key, value))
            }
        }
    }
}

impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            unsafe {
                let tail = self.tail.as_ptr();
                self.tail = Some((*tail).links.value.prev);
                let (key, value) = (*tail).entry_mut();
                Some((key, value))
            }
        }
    }
}

impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(K, V)> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        unsafe {
            let mut e = *Box::from_raw(self.tail.as_ptr());
            self.tail = Some(e.links.value.prev);
            Some(e.take_entry())
        }
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {}

impl<K, V> Drop for IntoIter<K, V> {
    #[inline]
    fn drop(&mut self) {
        for _ in 0..self.remaining {
            unsafe {
                let tail = self.tail.as_ptr();
                self.tail = Some((*tail).links.value.prev);
                (*tail).take_entry();
                let _ = Box::from_raw(tail);
            }
        }
    }
}

impl<'a, K, V, S> IntoIterator for &'a MiniMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    #[inline]
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut MiniMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    #[inline]
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

impl<K, V, S> IntoIterator for MiniMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    #[inline]
    fn into_iter(mut self) -> IntoIter<K, V> {
        unsafe {
            let (head, tail) = if let Some(values) = self.values {
                let ValueLinks {
                    next: head,
                    prev: tail,
                } = values.as_ref().links.value;

                let _ = Box::from_raw(self.values.as_ptr());
                self.values = None;

                (Some(head), Some(tail))
            } else {
                (None, None)
            };
            let len = self.len();

            drop_free_nodes(self.free.take());

            self.table.clear();

            IntoIter {
                head,
                tail,
                remaining: len,
                marker: PhantomData,
            }
        }
    }
}

struct ValueLinks<K, V> {
    next: NonNull<Node<K, V>>,
    prev: NonNull<Node<K, V>>,
}

impl<K, V> Clone for ValueLinks<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, V> Copy for ValueLinks<K, V> {}

struct FreeLink<K, V> {
    next: Option<NonNull<Node<K, V>>>,
}

impl<K, V> Clone for FreeLink<K, V> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, V> Copy for FreeLink<K, V> {}

union Links<K, V> {
    value: ValueLinks<K, V>,
    free: FreeLink<K, V>,
}

struct Node<K, V> {
    entry: MaybeUninit<(K, V)>,
    links: Links<K, V>,
}

impl<K, V> Node<K, V> {
    #[inline]
    unsafe fn put_entry(&mut self, entry: (K, V)) {
        self.entry.as_mut_ptr().write(entry)
    }

    #[inline]
    unsafe fn entry_ref(&self) -> &(K, V) {
        &*self.entry.as_ptr()
    }

    #[inline]
    unsafe fn key_ref(&self) -> &K {
        &(*self.entry.as_ptr()).0
    }

    #[inline]
    unsafe fn entry_mut(&mut self) -> &mut (K, V) {
        &mut *self.entry.as_mut_ptr()
    }

    #[inline]
    unsafe fn take_entry(&mut self) -> (K, V) {
        self.entry.as_ptr().read()
    }
}

trait OptNonNullExt<T> {
    fn as_ptr(self) -> *mut T;
}

impl<T> OptNonNullExt<T> for Option<NonNull<T>> {
    #[inline]
    fn as_ptr(self) -> *mut T {
        match self {
            Some(ptr) => ptr.as_ptr(),
            None => ptr::null_mut(),
        }
    }
}

// Allocate a circular list guard node if not present.
#[inline]
unsafe fn ensure_guard_node<K, V>(head: &mut Option<NonNull<Node<K, V>>>) {
    if head.is_none() {
        let mut p = NonNull::new_unchecked(Box::into_raw(Box::new(Node {
            entry: MaybeUninit::uninit(),
            links: Links {
                value: ValueLinks {
                    next: NonNull::dangling(),
                    prev: NonNull::dangling(),
                },
            },
        })));
        p.as_mut().links.value = ValueLinks { next: p, prev: p };
        *head = Some(p);
    }
}

// Attach the `to_attach` node to the existing circular list *before* `node`.
#[inline]
unsafe fn attach_before<K, V>(mut to_attach: NonNull<Node<K, V>>, mut node: NonNull<Node<K, V>>) {
    to_attach.as_mut().links.value = ValueLinks {
        prev: node.as_ref().links.value.prev,
        next: node,
    };
    node.as_mut().links.value.prev = to_attach;
    (*to_attach.as_mut().links.value.prev.as_ptr())
        .links
        .value
        .next = to_attach;
}

#[inline]
unsafe fn detach_node<K, V>(mut node: NonNull<Node<K, V>>) {
    node.as_mut().links.value.prev.as_mut().links.value.next = node.as_ref().links.value.next;
    node.as_mut().links.value.next.as_mut().links.value.prev = node.as_ref().links.value.prev;
}

#[inline]
unsafe fn push_free<K, V>(
    free_list: &mut Option<NonNull<Node<K, V>>>,
    mut node: NonNull<Node<K, V>>,
) {
    node.as_mut().links.free.next = *free_list;
    *free_list = Some(node);
}

#[inline]
unsafe fn pop_free<K, V>(
    free_list: &mut Option<NonNull<Node<K, V>>>,
) -> Option<NonNull<Node<K, V>>> {
    if let Some(free) = *free_list {
        *free_list = free.as_ref().links.free.next;
        Some(free)
    } else {
        None
    }
}

#[inline]
unsafe fn allocate_node<K, V>(free_list: &mut Option<NonNull<Node<K, V>>>) -> NonNull<Node<K, V>> {
    if let Some(mut free) = pop_free(free_list) {
        free.as_mut().links.value = ValueLinks {
            next: NonNull::dangling(),
            prev: NonNull::dangling(),
        };
        free
    } else {
        NonNull::new_unchecked(Box::into_raw(Box::new(Node {
            entry: MaybeUninit::uninit(),
            links: Links {
                value: ValueLinks {
                    next: NonNull::dangling(),
                    prev: NonNull::dangling(),
                },
            },
        })))
    }
}

// Given node is assumed to be the guard node and is *not* dropped.
#[inline]
unsafe fn drop_value_nodes<K, V>(guard: NonNull<Node<K, V>>) {
    let mut cur = guard.as_ref().links.value.prev;
    while cur != guard {
        let prev = cur.as_ref().links.value.prev;
        cur.as_mut().take_entry();
        let _ = Box::from_raw(cur.as_ptr());
        cur = prev;
    }
}

// Drops all linked free nodes starting with the given node.  Free nodes are only non-circular
// singly linked, and should have uninitialized keys / values.
#[inline]
unsafe fn drop_free_nodes<K, V>(mut free: Option<NonNull<Node<K, V>>>) {
    while let Some(some_free) = free {
        let next_free = some_free.as_ref().links.free.next;
        let _ = Box::from_raw(some_free.as_ptr());
        free = next_free;
    }
}

#[inline]
unsafe fn remove_node<K, V>(
    free_list: &mut Option<NonNull<Node<K, V>>>,
    mut node: NonNull<Node<K, V>>,
) -> (K, V) {
    detach_node(node);
    push_free(free_list, node);
    node.as_mut().take_entry()
}

#[inline]
unsafe fn hash_node<S, K, V>(s: &S, node: NonNull<Node<K, V>>) -> u64
where
    S: BuildHasher,
    K: Hash,
{
    hash_key(s, node.as_ref().key_ref())
}

#[inline]
fn hash_key<S, Q>(s: &S, k: &Q) -> u64
where
    S: BuildHasher,
    Q: Hash + ?Sized,
{
    let mut hasher = s.build_hasher();
    k.hash(&mut hasher);
    hasher.finish()
}

