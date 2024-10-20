use std::{
    fmt::{self, Debug},
    hash::Hash,
    mem,
    vec::IntoIter,
};

use arrayvec::ArrayVec;
use hashbrown::HashMap;

use crate::{Board, BoardIdxType, IndexMap};

// TODO: efficient set for boards with normal indizes

#[derive(PartialEq, Eq, Clone, Default)]
pub struct HashIndexMap<I: BoardIdxType + Hash, T = ()> {
    map: HashMap<I, T>,
    indizes: Vec<I>,
}

impl<I: BoardIdxType + Hash, T> Debug for HashIndexMap<I, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HashIndexMap {{ {:#?} }}", &self.map)
    }
}

impl<I: BoardIdxType + Hash, T> HashIndexMap<I, T> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            indizes: Vec::new(),
        }
    }
}

impl<'a, B: Board, T> From<&'a B> for HashIndexMap<B::Index, T>
where
    B::Index: Hash,
{
    fn from(_: &'a B) -> Self {
        Self::new()
    }
}

// TODO: replace with HashMap?!
impl<I: BoardIdxType + Hash, T> IndexMap for HashIndexMap<I, T> {
    type IndexType = I;
    type Item = T;
    type Iter = IntoIter<I>;

    fn size(&self) -> usize {
        self.map.len()
    }

    fn contains(&self, i: Self::IndexType) -> bool {
        self.map.contains_key(&i)
    }

    fn get(&self, i: Self::IndexType) -> Option<&T> {
        self.map.get(&i)
    }

    fn get_mut(&mut self, i: Self::IndexType) -> Option<&mut T> {
        self.map.get_mut(&i)
    }

    fn insert(&mut self, i: Self::IndexType, el: T) -> Option<T> {
        let result = self.map.insert(i, el);
        if result.is_none() {
            self.indizes.push(i);
        }
        result
    }

    fn retain(&mut self, mut filter: impl FnMut(Self::IndexType, &mut T) -> bool) {
        self.map.retain(|&i, t| filter(i, t));
        let map = &self.map;
        self.indizes.retain(|i| map.contains_key(i))
    }

    // TODO: this is a bit ugly, waiting for GATs..
    fn iter_indices(&self) -> Self::Iter {
        self.indizes.clone().into_iter()
    }

    fn clear(&mut self) {
        self.map.clear();
        self.indizes.clear();
    }
}
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct ArrayIndexMap<I: BoardIdxType, T, const N: usize> {
    data: ArrayVec<(I, T), N>,
}

impl<I: BoardIdxType, T, const N: usize> ArrayIndexMap<I, T, N> {
    pub fn new() -> Self {
        Self {
            data: ArrayVec::new(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.data.is_full()
    }
}

impl<'a, B: Board, T, const N: usize> From<&'a B> for ArrayIndexMap<B::Index, T, N> {
    fn from(_: &'a B) -> Self {
        Self::new()
    }
}

impl<I: BoardIdxType, T, const N: usize> IndexMap for ArrayIndexMap<I, T, N> {
    type IndexType = I;
    type Item = T;
    type Iter = IntoIter<I>;

    fn size(&self) -> usize {
        self.data.len()
    }

    fn contains(&self, i: Self::IndexType) -> bool {
        self.data.iter().any(|&(j, _)| i == j)
    }

    fn get(&self, i: Self::IndexType) -> Option<&T> {
        self.data.iter().find(|(j, _)| i == *j).map(|(_, val)| val)
    }

    fn get_mut(&mut self, i: Self::IndexType) -> Option<&mut T> {
        self.data
            .iter_mut()
            .find(|(j, _)| i == *j)
            .map(|(_, val)| val)
    }

    fn insert(&mut self, i: Self::IndexType, el: T) -> Option<T> {
        if let Some(contained) = self.get_mut(i) {
            Some(mem::replace(contained, el))
        } else {
            self.data.push((i, el));
            None
        }
    }

    fn retain(&mut self, mut filter: impl FnMut(Self::IndexType, &mut T) -> bool) {
        self.data.retain(|(i, t)| filter(*i, t));
    }

    // TODO: this is a bit ugly, waiting for GATs..
    fn iter_indices(&self) -> Self::Iter {
        self.data
            .iter()
            .map(|(i, _)| i)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn clear(&mut self) {
        self.data.clear()
    }
}

#[cfg(test)]
mod test {
    use crate::{index_map::HashIndexMap, BoardIdxType, IndexMap};

    use super::ArrayIndexMap;

    impl BoardIdxType for usize {}

    #[test]
    fn hash_index_map_test() {
        let mut map = HashIndexMap::<usize, i32>::new();
        assert_eq!(map.size(), 0);

        map.insert(0, 3);
        map.insert(1, 2);
        assert!(map.contains(0));
        assert!(map.contains(1));
        assert!(!map.contains(2));
        assert_eq!(map.get(0), Some(&3));
        assert_eq!(map.get(1), Some(&2));
        map.insert(0, 0);
        assert_eq!(map.get(0), Some(&0));
        map.insert(2, 2);
        assert_eq!(map.get(2), Some(&2));
        assert_eq!(map.size(), 3);
        map.retain(|i, _| i != 1);
        assert_eq!(map.get(0), Some(&0));
        assert_eq!(map.get(1), None);
        assert_eq!(map.get(2), Some(&2));
        assert_eq!(map.iter_indices().collect::<Vec<_>>(), vec![0, 2]);
        map.clear();
        assert!(!map.contains(0) && map.iter_indices().count() == 0);
    }

    #[test]
    fn array_index_map_test() {
        let mut map = ArrayIndexMap::<usize, i32, 3>::new();
        assert_eq!(map.size(), 0);

        map.insert(0, 3);
        map.insert(1, 2);
        assert!(map.contains(0));
        assert!(map.contains(1));
        assert!(!map.contains(2));
        assert_eq!(map.get(0), Some(&3));
        assert_eq!(map.get(1), Some(&2));
        map.insert(0, 0);
        assert!(!map.is_full());
        assert_eq!(map.get(0), Some(&0));
        map.insert(2, 2);
        assert!(map.is_full());
        assert_eq!(map.get(2), Some(&2));
        assert_eq!(map.size(), 3);
        map.retain(|i, _| i != 1);
        assert_eq!(map.get(0), Some(&0));
        assert_eq!(map.get(1), None);
        assert_eq!(map.get(2), Some(&2));
        assert_eq!(map.iter_indices().collect::<Vec<_>>(), vec![0, 2]);
        map.clear();
        assert!(!map.contains(0) && map.iter_indices().count() == 0);
    }
}
