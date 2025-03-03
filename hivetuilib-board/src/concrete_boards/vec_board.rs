use crate::{
    index_map::HashIndexMap,
    search::{SearchingSet, SearchingTree},
    structures::directions::{DirectionOffset, Offset, OffsetableIndex},
    Board, BoardIdxType, BoardIndexable, BoardMut, BoardToMap, ContiguousBoard, Field,
};

use std::{
    iter,
    ops::{Add, Index, IndexMut},
};

/// A one-dimensional immutable board.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VecBoard<T, S = ()> {
    content: Box<[T]>,
    structure: S,
}

// TODO: default structure
impl<T, S> VecBoard<T, S> {
    pub fn from_value(count: usize, val: T, structure: S) -> Self
    where
        T: Clone,
    {
        Self {
            content: iter::repeat(val).take(count).collect(),
            structure,
        }
    }

    pub fn with_default(count: usize, structure: S) -> Self
    where
        T: Default,
    {
        Self {
            content: iter::repeat_with(Default::default).take(count).collect(),
            structure,
        }
    }

    pub fn search(&self) -> SearchingSet<HashIndexMap<Index1D>, Self> {
        SearchingSet::new(self)
    }

    pub fn search_tree(&self) -> SearchingTree<HashIndexMap<Index1D>, Self> {
        SearchingTree::new(self)
    }
}

impl<T: Default, S> VecBoard<T, S> {}

// TODO: builder

impl<T, I: Into<Index1D>, S> Index<I> for VecBoard<T, S> {
    type Output = T;

    fn index(&self, index: I) -> &T {
        self.content.index(index.into().val)
    }
}

impl<T, I: Into<Index1D>, S> IndexMut<I> for VecBoard<T, S> {
    fn index_mut(&mut self, index: I) -> &mut T {
        self.content.index_mut(index.into().val)
    }
}

impl<T, S> BoardIndexable for VecBoard<T, S> {
    type Index = Index1D;

    #[inline(always)]
    fn all_indices(&self) -> impl Iterator<Item = Index1D> {
        (0..self.content.len()).map(Index1D::from)
    }
}

impl<T, S> Board for VecBoard<T, S> {
    type Content = T;
    type Structure = S;

    fn size(&self) -> usize {
        self.content.len()
    }

    fn structure(&self) -> &S {
        &self.structure
    }

    fn get(&self, index: Index1D) -> Option<&T> {
        self.content.get(index.val)
    }
}

impl<T, S> BoardMut for VecBoard<T, S> {
    fn get_mut(&mut self, index: Index1D) -> Option<&mut T> {
        self.content.get_mut(index.val)
    }
}

impl<T, S> ContiguousBoard for VecBoard<T, S> {
    type Offset = Offset;

    fn bound(&self) -> Index1D {
        Index1D::from(self.content.len())
    }

    fn wrapped(&self, Offset(index): Offset) -> Index1D {
        let rem = index.rem_euclid(self.content.len() as isize);
        assert!(rem >= 0);
        Index1D::from(rem as usize)
    }
}

// TODO: more efficient set
impl<T, S, E> BoardToMap<E> for VecBoard<T, S> {
    type Map = HashIndexMap<Index1D, E>;

    fn get_index_map(&self) -> Self::Map {
        Self::Map::new()
    }
}

// ----- the index belonging to a VecBoard -----

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Index1D {
    pub val: usize,
}

impl BoardIdxType for Index1D {}

impl From<usize> for Index1D {
    fn from(val: usize) -> Self {
        Self { val }
    }
}

impl<B: Board<Index = Index1D>> From<Field<'_, B>> for Index1D {
    fn from(f: Field<'_, B>) -> Self {
        f.index()
    }
}

impl OffsetableIndex for Index1D {
    type Offset = Offset;

    fn apply_offset(&self, Offset(delta): Offset) -> Offset {
        Offset(self.val as isize + delta)
    }

    fn from_offset(Offset(index): Offset) -> Option<Self> {
        if index >= 0 {
            Some(Self::from(index as usize))
        } else {
            None
        }
    }
}

impl<D> Add<D> for Index1D
where
    D: DirectionOffset<<Self as OffsetableIndex>::Offset>,
{
    type Output = Option<Self>;

    fn add(self, rhs: D) -> Self::Output {
        Self::from_offset(self.apply_offset(rhs.offset()))
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn basic_test() {
        use super::*;

        let board = VecBoard::<usize, ()>::with_default(1, ());
        assert_eq!(board.size(), 1);
        assert_eq!(board[0], 0);
    }
}
