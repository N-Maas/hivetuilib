use std::iter::{self, FromIterator};

use super::{directions::DirectionReversable, *};

pub trait IndexSet {
    type IndexType: BoardIdxType;
    type Iter: ExactSizeIterator<Item = Self::IndexType>;

    // fn from_board<B: Board<Self::IndexType>>(board: &'a B) -> Self;
    fn size(&self) -> usize;

    fn contains(&self, i: Self::IndexType) -> bool;

    /// Returns true if the index was not contained in the set before.
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

impl<'a, B: Board> From<&'a B> for HashIndexSet<B::Index>
where
    B::Index: Hash,
{
    fn from(_: &'a B) -> Self {
        Self::new()
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
        SearchingSet::with_set(self.get_index_set(), self)
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
        set.insert(self.index());
        set
    }
}

impl<'a, B: BoardToSet + 'a> FromIterator<Field<'a, B>> for Option<SearchingSet<'a, B::Set, B>>
where
    B: Board<Index = <<B as BoardToSet>::Set as IndexSet>::IndexType>,
{
    fn from_iter<T: IntoIterator<Item = Field<'a, B>>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        let first = iter.next();
        first.map(|f| {
            let mut set = f.search();
            for field in iter {
                set.insert(field.index());
            }
            set
        })
    }
}

impl<'a, S, B> Field<'a, B>
where
    S: DirectionStructure<B>,
    B: BoardToSet,
    B: Board<Structure = S, Index = <<B as BoardToSet>::Set as IndexSet>::IndexType>,
{
    /// Note that the first element is self.
    ///
    /// It is guaranteed that no field is visited twice.
    pub fn iter_line(&self, direction: S::Direction) -> impl Iterator<Item = Field<'a, B>> {
        let mut set = self.board().get_index_set();
        iter::successors(Some(*self), move |f| f.get_successor(direction, &mut set))
    }

    /// The iterator will first follow the line of the given direction
    /// while the predicate return true. Afterwards, it continues with the reverse direction.
    ///
    /// Note that the first element is self. In the case that the predicate rejects self, iteration still continues.
    ///
    /// It is guaranteed that no field is visited twice.
    pub fn iter_bidirectional<P>(
        &self,
        direction: S::Direction,
        predicate: P,
    ) -> Bidirectional<'a, S, B, P>
    where
        P: FnMut(Self) -> bool,
        S::Direction: DirectionReversable,
    {
        Bidirectional::new(*self, direction, predicate)
    }

    fn get_successor(&self, direction: S::Direction, set: &mut B::Set) -> Option<Self> {
        let field = self.next(direction)?;
        if set.insert(field.index()) {
            Some(field)
        } else {
            None
        }
    }
}

pub struct Bidirectional<'a, S, B, P>
where
    S: DirectionStructure<B>,
    B: Board<Structure = S> + BoardToSet,
{
    root: Option<Field<'a, B>>,
    previous: Option<Field<'a, B>>,
    direction: S::Direction,
    set: B::Set,
    predicate: P,
}

impl<'a, S, B, P> Bidirectional<'a, S, B, P>
where
    S: DirectionStructure<B>,
    B: Board<Structure = S> + BoardToSet,
{
    pub fn new(root: Field<'a, B>, direction: S::Direction, predicate: P) -> Self {
        Self {
            root: Some(root),
            previous: None,
            direction,
            set: root.board().get_index_set(),
            predicate,
        }
    }
}

impl<'a, S, B, P> Iterator for Bidirectional<'a, S, B, P>
where
    S: DirectionStructure<B>,
    S::Direction: DirectionReversable,
    B: Board<Structure = S, Index = <<B as BoardToSet>::Set as IndexSet>::IndexType> + BoardToSet,
    P: FnMut(Field<'a, B>) -> bool,
{
    type Item = Field<'a, B>;

    fn next(&mut self) -> Option<Field<'a, B>> {
        match self.previous {
            // handle some edge cases for the root element
            None => {
                // unwrap is safe due to initialization
                let root = self.root.unwrap();
                debug_assert!(self.set.insert(root.index()));
                self.previous = Some(root);
                if (self.predicate)(root) {
                    Some(root)
                } else {
                    self.next()
                }
            }
            Some(field) => {
                let next = field
                    .get_successor(self.direction, &mut self.set)
                    .filter(|f| (self.predicate)(*f));
                match next {
                    Some(_) => {
                        self.previous = next;
                        next
                    }
                    None => {
                        // When the first line is finished: reset to root, switch direction and continue.
                        let root = self.root.take()?;
                        self.previous = Some(root);
                        self.direction = self.direction.reversed();
                        self.next()
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SearchingSet<'a, S: IndexSet, B: Board<Index = S::IndexType>> {
    base_set: S,
    board: &'a B,
}

impl<'a, S: IndexSet, B: Board<Index = S::IndexType>> SearchingSet<'a, S, B> {
    pub fn new(board: &'a B) -> Self
    where
        S: From<&'a B>,
    {
        Self {
            base_set: S::from(board),
            board,
        }
    }

    pub fn with_set(base_set: S, board: &'a B) -> Self {
        Self { base_set, board }
    }

    pub fn size(&self) -> usize {
        self.base_set.size()
    }

    pub fn contains<T: Into<B::Index>>(&self, el: T) -> bool {
        self.base_set.contains(el.into())
    }

    pub fn insert<T: Into<B::Index>>(&mut self, el: T) -> bool {
        self.base_insert(el.into())
    }

    pub fn iter(&self) -> impl Iterator<Item = Field<'_, B>> {
        self.base_set
            .iter()
            .map(move |i| self.board.get_field(i).unwrap())
    }

    pub fn clear(&mut self) {
        self.base_set.clear()
    }

    // ----- the search API -----
    pub fn extend<F>(&mut self, map_fields: F) -> bool
    where
        F: FnMut(Field<B>) -> FieldSearchResult<B::Index>,
    {
        self.extend_helper(Self::apply_mapping(self.iter(), map_fields))
    }

    /// The closure must not use interior mutability.
    pub fn extend_repeated<F>(&mut self, map_fields: F) -> bool
    where
        F: Fn(Field<B>) -> FieldSearchResult<B::Index>,
    {
        let mut success = false;
        let mut result = Self::apply_mapping(self.iter(), &map_fields);
        let mut queued = Vec::new();
        loop {
            queued.clear();
            // insert into set and queue in parallel
            queued.extend(result.iter().filter(|i| self.base_insert(*i)));
            if queued.is_empty() {
                return success;
            } else {
                success = true;
            }

            // only read the values from the last iteration to avoid quadratic complexity
            result = Self::apply_mapping(
                // unwrap can not fail as the index was checked in base_insert
                queued.iter().map(|i| self.board.get_field(*i).unwrap()),
                &map_fields,
            );
        }
    }

    pub fn extend_with<F>(&mut self, collect: F) -> bool
    where
        F: FnOnce(&Self) -> FieldSearchResult<B::Index>,
    {
        self.extend_helper(collect(self))
    }

    pub fn extend_with_repeated<F>(&mut self, mut collect: F) -> bool
    where
        F: FnMut(&Self) -> FieldSearchResult<B::Index>,
    {
        let mut success = false;
        while self.extend_with(&mut collect) {
            success = true;
        }
        success
    }

    pub fn grow<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        self.extend_helper(self.apply_growth(predicate))
    }

    pub fn grow_repeated<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        self.extend_repeated(|f| {
            f.neighbors()
                .filter(|f| predicate(*f))
                .map(|f| f.index())
                .collect()
        })
    }

    pub fn replace<F>(&mut self, map_fields: F)
    where
        F: FnMut(Field<B>) -> FieldSearchResult<B::Index>,
    {
        let fields = Self::apply_mapping(self.iter(), map_fields);
        self.replace_helper(fields)
    }

    pub fn replace_with<F>(&mut self, collect: F)
    where
        F: FnOnce(&Self) -> FieldSearchResult<B::Index>,
    {
        self.replace_helper(collect(self))
    }

    pub fn step<F>(&mut self, predicate: F)
    where
        F: Fn(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        self.replace_helper(self.apply_growth(predicate))
    }

    fn base_insert(&mut self, i: B::Index) -> bool {
        if !self.board.contains(i) {
            panic!("Field with invalid index.");
        }
        self.base_set.insert(i)
    }

    fn apply_mapping<'b, F>(
        iter: impl Iterator<Item = Field<'b, B>>,
        mut map_fields: F,
    ) -> FieldSearchResult<B::Index>
    where
        F: FnMut(Field<B>) -> FieldSearchResult<B::Index>,
        B: 'b,
    {
        iter.flat_map(|f| map_fields(f).into_iter()).collect()
    }

    fn apply_growth<F>(&self, predicate: F) -> FieldSearchResult<B::Index>
    where
        F: Fn(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        self.iter()
            .flat_map(|f| f.neighbors().filter(|f| predicate(*f)))
            .map(|f| f.index())
            .collect()
    }

    fn extend_helper(&mut self, fields: FieldSearchResult<B::Index>) -> bool {
        let mut success = false;
        for i in fields.iter() {
            self.base_insert(i);
            success = true;
        }
        success
    }

    fn replace_helper(&mut self, fields: FieldSearchResult<B::Index>) {
        self.base_set.clear();
        for i in fields.iter() {
            self.base_insert(i);
        }
    }
}

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

#[cfg(test)]
mod test {
    use super::{
        directions::{BinaryDirection, GridDirection},
        matrix_board::*,
        structures::WrappedOffsetStructure,
        vec_board::*,
        *,
    };

    #[test]
    fn search_repeated_test() {
        type TestBoard = MatrixBoard<usize, WrappedOffsetStructure<Index2D, GridDirection>>;

        let board = TestBoard::with_default(2, 2, WrappedOffsetStructure::new());
        let mut search = board.iter_fields().nth(0).unwrap().search();
        assert!(search.grow_repeated(|_| true));
        assert_eq!(search.size(), 4);
        for &(x, y) in [(0, 0), (0, 1), (1, 0), (1, 1)].iter() {
            assert!(search.contains(board.get_field(Index2D { x, y }).unwrap()));
        }
    }

    #[test]
    fn bidirectional_test() {
        type TestBoard = VecBoard<Option<()>, WrappedOffsetStructure<Index1D, BinaryDirection>>;

        let mut board = TestBoard::with_default(5, WrappedOffsetStructure::new());
        let center = board.get_field_unchecked(2.into());
        iter_eq(
            center.iter_bidirectional(BinaryDirection::Forward, |_| true),
            &[2, 3, 4, 0, 1],
        );

        board[4] = Some(());
        let center = board.get_field_unchecked(2.into());
        iter_eq(
            center.iter_bidirectional(BinaryDirection::Forward, |f| f.is_empty()),
            &[2, 3, 1, 0],
        );

        // if the root does not match the predicate, iteration still continues
        board[2] = Some(());
        let center = board.get_field_unchecked(2.into());
        iter_eq(
            center.iter_bidirectional(BinaryDirection::Forward, |f| f.is_empty()),
            &[3, 1, 0],
        );

        board[3] = Some(());
        let center = board.get_field_unchecked(2.into());
        iter_eq(
            center.iter_bidirectional(BinaryDirection::Forward, |f| f.is_empty()),
            &[1, 0],
        );
    }

    fn iter_eq<T: Into<Index1D>>(left: impl Iterator<Item = T>, right: &[usize]) {
        let checked = left
            .zip(right.iter())
            .map(|(l, &r)| assert_eq!(l.into(), r.into()));
        assert_eq!(checked.count(), right.len());
    }
}
