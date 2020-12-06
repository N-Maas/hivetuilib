use std::{
    collections::HashMap,
    iter::{self, FromIterator},
};

use super::{directions::DirectionReversable, *};

// TODO: efficient set for boards with normal indizes

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct HashIndexMap<I: BoardIdxType + Hash, T> {
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

    fn insert(&mut self, i: Self::IndexType, el: T) -> Option<T> {
        self.map.insert(i, el)
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
    pub fn iter(&self) -> impl ExactSizeIterator<Item = M::IndexType> {
        self.map.iter_indices()
    }

    pub fn clear(&mut self) {
        self.map.clear()
    }
}

impl<M: IndexMap<Item = ()>> From<M> for SetWrapper<M> {
    fn from(map: M) -> Self {
        Self { map }
    }
}

pub trait Searchable<'a> {
    type Map: IndexMap<Item = ()>;
    type Board: Board<Index = <Self::Map as IndexMap>::IndexType>;

    fn search(self) -> SearchingSet<'a, Self::Map, Self::Board>;
}

impl<'a, B: BoardToMap<()>> Searchable<'a> for &'a B {
    type Map = <B as BoardToMap<()>>::Map;
    type Board = B;

    fn search(self) -> SearchingSet<'a, Self::Map, Self::Board> {
        SearchingSet::with_map(self.get_index_map(), self)
    }
}

impl<'a, B: BoardToMap<()>> Searchable<'a> for Field<'a, B> {
    type Map = <B as BoardToMap<()>>::Map;
    type Board = B;

    fn search(self) -> SearchingSet<'a, Self::Map, Self::Board> {
        let mut set = self.board().search();
        set.insert(self.index());
        set
    }
}

impl<'a, B: BoardToMap<()> + 'a> FromIterator<Field<'a, B>>
    for Option<SearchingSet<'a, B::Map, B>>
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

impl<'a, M, B> Field<'a, B>
where
    M: DirectionStructure<B>,
    B: BoardToMap<(), Structure = M>,
{
    /// Note that the first element is self.
    ///
    /// It is guaranteed that no field is visited twice.
    pub fn iter_line(&self, direction: M::Direction) -> impl Iterator<Item = Field<'a, B>> {
        let mut set = self.board().get_index_map().into();
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
        direction: M::Direction,
        predicate: P,
    ) -> Bidirectional<'a, M, B, P>
    where
        P: FnMut(Self) -> bool,
        M::Direction: DirectionReversable,
    {
        Bidirectional::new(*self, direction, predicate)
    }

    fn get_successor(&self, direction: M::Direction, set: &mut SetWrapper<B::Map>) -> Option<Self> {
        let field = self.next(direction)?;
        if set.insert(field.index()) {
            Some(field)
        } else {
            None
        }
    }
}

pub struct Bidirectional<'a, M, B, P>
where
    M: DirectionStructure<B>,
    B: Board<Structure = M> + BoardToMap<()>,
{
    root: Option<Field<'a, B>>,
    previous: Option<Field<'a, B>>,
    direction: M::Direction,
    set: SetWrapper<B::Map>,
    predicate: P,
}

impl<'a, M, B, P> Bidirectional<'a, M, B, P>
where
    M: DirectionStructure<B>,
    B: Board<Structure = M> + BoardToMap<()>,
{
    pub fn new(root: Field<'a, B>, direction: M::Direction, predicate: P) -> Self {
        Self {
            root: Some(root),
            previous: None,
            direction,
            set: root.board().get_index_map().into(),
            predicate,
        }
    }
}

impl<'a, M, B, P> Iterator for Bidirectional<'a, M, B, P>
where
    M: DirectionStructure<B>,
    M::Direction: DirectionReversable,
    B: BoardToMap<(), Structure = M>,
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

#[derive(Debug, PartialEq, Eq)]
pub struct SearchingSet<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    base_set: SetWrapper<M>,
    board: &'a B,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> SearchingSet<'a, M, B> {
    pub fn new(board: &'a B) -> Self
    where
        M: From<&'a B>,
    {
        Self {
            base_set: M::from(board).into(),
            board,
        }
    }

    pub fn with_map(map: M, board: &'a B) -> Self {
        Self {
            base_set: map.into(),
            board,
        }
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
