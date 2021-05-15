mod searching_set;
mod searching_tree;

pub use searching_set::*;
pub use searching_tree::*;

use std::{iter::FromIterator, vec::IntoIter};

use crate::{BoardIdxType, IndexMap};

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

#[derive(Debug, Clone, PartialEq, Eq)]
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
