mod searching_set;
mod searching_tree;

pub use searching_set::*;
pub use searching_tree::*;

use std::{collections::HashMap, hash::Hash, iter::FromIterator, vec::IntoIter};

use crate::{Board, BoardIdxType, IndexMap};

// TODO: efficient set for boards with normal indizes

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct HashIndexMap<I: BoardIdxType + Hash, T = ()> {
    map: HashMap<I, T>,
}

impl<I: BoardIdxType + Hash, T> HashIndexMap<I, T> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
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
        self.map.insert(i, el)
    }

    fn retain(&mut self, mut filter: impl FnMut(Self::IndexType, &mut T) -> bool) {
        self.map.retain(|&i, t| filter(i, t));
    }

    // TODO: this is a bit ugly, waiting for GATs..
    fn iter_indices(&self) -> Self::Iter {
        self.map.keys().copied().collect::<Vec<_>>().into_iter()
    }

    fn clear(&mut self) {
        self.map.clear()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub(crate) struct SetWrapper<M: IndexMap<Item = ()>> {
    map: M,
}

impl<M: IndexMap<Item = ()>> SetWrapper<M> {
    pub fn size(&self) -> usize {
        self.map.size()
    }

    pub fn contains(&self, i: M::IndexType) -> bool {
        self.map.contains(i)
    }

    pub fn insert(&mut self, i: M::IndexType) -> bool {
        self.map.insert(i, ()).is_none()
    }

    // TODO: this is a bit ugly, waiting for GATs..
    pub fn iter(&self) -> M::Iter {
        self.map.iter_indices()
    }

    pub fn clear(&mut self) {
        self.map.clear()
    }

    pub fn into_map(self) -> M {
        self.map
    }
}

impl<M: IndexMap<Item = ()>> From<M> for SetWrapper<M> {
    fn from(map: M) -> Self {
        Self { map }
    }
}

// ----- result type -----

pub struct FieldSearchResult<I: BoardIdxType> {
    data: Vec<I>,
}

impl<I: BoardIdxType> FieldSearchResult<I> {
    fn iter(&self) -> impl Iterator<Item = I> + '_ {
        self.data.iter().copied()
    }
}

impl<I: BoardIdxType, T: Into<I>> FromIterator<T> for FieldSearchResult<I> {
    fn from_iter<Iter: IntoIterator<Item = T>>(iter: Iter) -> Self {
        Self {
            data: iter.into_iter().map(|val| val.into()).collect(),
        }
    }
}

impl<I: BoardIdxType, T: Into<I>> From<Vec<T>> for FieldSearchResult<I> {
    fn from(vec: Vec<T>) -> Self {
        Self::from_iter(vec.into_iter())
    }
}

impl<I: BoardIdxType> IntoIterator for FieldSearchResult<I> {
    type Item = I;
    type IntoIter = IntoIter<I>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}
