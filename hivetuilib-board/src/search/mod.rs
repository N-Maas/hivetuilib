mod searching_set;
mod searching_tree;

pub use searching_set::*;
pub use searching_tree::*;

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

pub trait FieldSearchIter<'a, I: BoardIdxType> {
    fn into(self) -> impl Iterator<Item = I> + 'a;
}

impl<'a, I: BoardIdxType + 'a, T: Into<I> + 'a, Iter> FieldSearchIter<'a, I> for Iter
where
    Iter: Iterator<Item = T> + 'a,
{
    fn into(self) -> impl Iterator<Item = I> + 'a {
        self.map(T::into)
    }
}
