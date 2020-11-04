use super::*;

pub trait IndexSet {
    type IndexType: BoardIdxType;
    type Iter: ExactSizeIterator<Item = Self::IndexType>;

    // fn from_board<B: Board<Self::IndexType>>(board: &'a B) -> Self;
    fn size(&self) -> usize;

    fn contains(&self, i: Self::IndexType) -> bool;

    fn insert(&mut self, i: Self::IndexType) -> bool;

    fn iter(&self) -> Self::Iter;

    fn clear(&mut self);

    // TODO: subset and further helper methods?
}

// TODO: efficient set for boards with normal indizes

#[derive(Debug, PartialEq, Eq)]
pub struct HashIndexSet<I: BoardIdxType + Hash> {
    set: HashSet<I>,
}

impl<I: BoardIdxType + Hash> HashIndexSet<I> {
    pub fn new() -> Self {
        Self {
            set: HashSet::new(),
        }
    }
}

impl<I: BoardIdxType + Hash> IndexSet for HashIndexSet<I> {
    type IndexType = I;

    type Iter = IntoIter<I>;

    fn size(&self) -> usize {
        self.set.len()
    }

    fn contains(&self, i: Self::IndexType) -> bool {
        self.set.contains(&i)
    }

    fn insert(&mut self, i: Self::IndexType) -> bool {
        self.set.insert(i)
    }

    // TODO: this is a bit ugly, waiting for GATs..
    fn iter(&self) -> Self::Iter {
        self.set.iter().copied().collect::<Vec<_>>().into_iter()
    }

    fn clear(&mut self) {
        self.set.clear()
    }
}

pub trait Searchable {
    type Set: IndexSet;
    type Board: Board<<Self::Set as IndexSet>::IndexType>;

    fn search(&self) -> SearchingSet<'_, Self::Set, Self::Board>;
}

// TODO: Searchable implementations by hand..

#[derive(Debug)]
pub struct SearchingSet<'a, S: IndexSet, B: Board<S::IndexType>> {
    base_set: S,
    board: &'a B,
}

impl<'a, S: IndexSet, B: Board<S::IndexType>> SearchingSet<'a, S, B> {
    pub fn new(base_set: S, board: &'a B) -> Self {
        Self { base_set, board }
    }

    pub fn size(&self) -> usize {
        self.base_set.size()
    }

    pub fn contains(&self, i: S::IndexType) -> bool {
        self.base_set.contains(i)
    }

    pub fn insert(&mut self, i: S::IndexType) -> bool {
        self.base_set.insert(i)
    }

    // TODO: this is a bit ugly, waiting for GATs..
    pub fn iter(&self) -> S::Iter {
        self.base_set.iter()
    }

    pub fn clear(&mut self) {
        self.base_set.clear()
    }
}
