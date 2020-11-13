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

pub trait BoardToSet {
    type Set: IndexSet;

    fn get_index_set(&self) -> Self::Set;
}

impl<T, S> BoardToSet for board_impl::VecBoard<T, S> {
    type Set = HashIndexSet<Index1D>;

    fn get_index_set(&self) -> Self::Set {
        Self::Set::new()
    }
}

impl<T, S> BoardToSet for board_impl::MatrixBoard<T, S> {
    type Set = HashIndexSet<Index2D>;

    fn get_index_set(&self) -> Self::Set {
        Self::Set::new()
    }
}

pub trait Searchable<'a> {
    type Set: IndexSet;
    type Board: Board<Index = <Self::Set as IndexSet>::IndexType>;

    fn search(self) -> SearchingSet<'a, Self::Set, Self::Board>;
}

impl<'a, B: BoardToSet> Searchable<'a> for &'a B
where
    B: Board<Index = <<B as BoardToSet>::Set as IndexSet>::IndexType>,
{
    type Set = <B as BoardToSet>::Set;
    type Board = B;

    fn search(self) -> SearchingSet<'a, Self::Set, Self::Board> {
        SearchingSet::new(self.get_index_set(), self)
    }
}

impl<'a, B: BoardToSet> Searchable<'a> for Field<'a, B>
where
    B: Board<Index = <<B as BoardToSet>::Set as IndexSet>::IndexType>,
{
    type Set = <B as BoardToSet>::Set;
    type Board = B;

    fn search(self) -> SearchingSet<'a, Self::Set, Self::Board> {
        let mut set = self.board().search();
        set.insert(self);
        set
    }
}

#[derive(Debug)]
pub struct SearchingSet<'a, S: IndexSet, B: Board<Index = S::IndexType>> {
    base_set: S,
    board: &'a B,
}

impl<'a, S: IndexSet, B: Board<Index = S::IndexType>> SearchingSet<'a, S, B> {
    pub fn new(base_set: S, board: &'a B) -> Self {
        Self { base_set, board }
    }

    pub fn size(&self) -> usize {
        self.base_set.size()
    }

    pub fn contains(&self, f: Field<'_, B>) -> bool {
        self.base_set.contains(f.index())
    }

    pub fn insert(&mut self, f: Field<'_, B>) -> bool {
        if !self.board.contains(f.index()) {
            panic!("Field with invalid index.");
        }
        self.base_set.insert(f.index())
    }

    // TODO: this is a bit ugly, waiting for GATs..
    pub fn iter(&self) -> impl Iterator<Item = Field<'_, B>> {
        self.base_set
            .iter()
            .map(move |i| self.board.get_field(i).unwrap())
    }

    pub fn clear(&mut self) {
        self.base_set.clear()
    }
}
