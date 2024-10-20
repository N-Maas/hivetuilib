use std::fmt::{self, Debug};
use std::iter::FromIterator;

use crate::{structures::NeighborhoodStructure, Board, BoardToMap, Field, IndexMap};

use super::{FieldSearchIter, SetWrapper};

// Equally applicable to SearchingTree:
// TODO: method for removing field?!
// TODO: consider laziness
// TODO: hooks
// TODO: default value for M possible?
#[derive(Eq)]
pub struct SearchingSet<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    base_set: SetWrapper<M>,
    board: &'a B,
    buffer: Vec<B::Index>,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Debug for SearchingSet<'a, M, B>
where
    M: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SearchingSet")
            .field("base_set", &self.base_set)
            .finish()
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> PartialEq
    for SearchingSet<'a, M, B>
where
    M: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.base_set.eq(&other.base_set)
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Clone for SearchingSet<'a, M, B>
where
    M: Clone,
{
    fn clone(&self) -> Self {
        Self {
            base_set: self.base_set.clone(),
            board: self.board,
            buffer: Vec::new(),
        }
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> SearchingSet<'a, M, B> {
    pub fn new(board: &'a B) -> Self
    where
        B: BoardToMap<(), Map = M>,
    {
        Self {
            base_set: board.get_index_map().into(),
            board,
            buffer: Vec::new(),
        }
    }

    pub fn from_map(map: M, board: &'a B) -> Self {
        Self {
            base_set: map.into(),
            board,
            buffer: Vec::new(),
        }
    }

    pub fn board(&self) -> &'a B {
        self.board
    }

    // the number of contained fields
    pub fn size(&self) -> usize {
        self.base_set.size()
    }

    pub fn contains<T: Into<B::Index>>(&self, el: T) -> bool {
        self.base_set.contains(el.into())
    }

    /// panics if the provided index is invalid
    pub fn insert<T: Into<B::Index>>(&mut self, el: T) -> bool {
        let idx = el.into();
        if !self.board.contains(idx) {
            panic!("Invalid index provided: {:?}", idx);
        }
        self.base_set.insert(idx)
    }

    pub fn iter(&self) -> Iter<'a, M, B> {
        Iter {
            board: self.board,
            iter: self.base_set.iter(),
        }
    }

    pub fn clear(&mut self) {
        self.base_set.clear()
    }

    // ----- the search API -----
    /// Returns true, if at least one field was added.
    pub fn extend<F, Iter>(&mut self, map_fields: F) -> bool
    where
        F: FnMut(Field<'a, B>) -> Iter,
        Iter: FieldSearchIter<'a, B::Index>,
    {
        self.new_via_map(map_fields);
        self.extend_helper()
    }

    /// Returns true, if at least one field was added.
    ///
    /// Note: The closure must not use interior mutability.
    pub fn extend_repeated<F, Iter>(&mut self, map_fields: F) -> bool
    where
        F: Fn(Field<'a, B>) -> Iter,
        Iter: FieldSearchIter<'a, B::Index>,
    {
        self.extend_repeated_impl(move |f, buffer| buffer.extend(map_fields(f).into()))
    }

    fn extend_repeated_impl<F>(&mut self, map_fields: F) -> bool
    where
        F: Fn(Field<'a, B>, &mut Vec<B::Index>),
    {
        let mut success = false;
        let mut queued = Vec::new();
        for f in self.iter() {
            map_fields(f, &mut self.buffer);
        }
        loop {
            queued.clear();
            // insert into set and queue in parallel
            queued.extend(self.buffer.iter().filter(|&&i| {
                if !self.board.contains(i) {
                    panic!("Field with invalid index: {:?}", i);
                }
                self.base_set.insert(i)
            }));
            self.buffer.clear();
            if queued.is_empty() {
                return success;
            } else {
                success = true;
            }

            // only read the values from the last iteration to avoid quadratic complexity
            for f in queued
                .iter()
                // we already checked that the index is valid
                .map(|i| self.board.get_field(*i).unwrap())
            {
                map_fields(f, &mut self.buffer);
            }
        }
    }

    // /// Returns true, if at least one field was added.
    // pub fn extend_with<F>(&mut self, collect: F) -> bool
    // where
    //     F: FnOnce(&Self) -> FieldSearchResult<B::Index>,
    // {
    //     self.extend_helper(collect(self))
    // }

    // /// Returns true, if at least one field was added.
    // pub fn extend_with_repeated<F>(&mut self, mut collect: F) -> bool
    // where
    //     F: FnMut(&Self) -> FieldSearchResult<B::Index>,
    // {
    //     let mut success = false;
    //     while self.extend_with(&mut collect) {
    //         success = true;
    //     }
    //     success
    // }

    /// Returns true, if at least one field was added.
    pub fn grow<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        self.new_via_pred(predicate);
        self.extend_helper()
    }

    /// Returns true, if at least one field was added.
    pub fn grow_repeated<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        self.extend_repeated_impl(|f, buffer| {
            buffer.extend(f.neighbors().filter(|&f| predicate(f)).map(Field::index))
        })
    }

    pub fn replace<F, Iter>(&mut self, map_fields: F)
    where
        F: FnMut(Field<'a, B>) -> Iter,
        Iter: FieldSearchIter<'a, B::Index>,
    {
        self.new_via_map(map_fields);
        self.base_set.clear();
        self.extend_helper();
    }

    // pub fn replace_with<F>(&mut self, collect: F)
    // where
    //     F: FnOnce(&Self) -> FieldSearchResult<B::Index>,
    // {
    //     self.replace_helper(collect(self))
    // }

    pub fn step<F>(&mut self, predicate: F)
    where
        F: Fn(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        self.new_via_pred(predicate);
        self.base_set.clear();
        self.extend_helper();
    }

    fn new_via_map<F, Iter>(&mut self, mut map_fields: F)
    where
        F: FnMut(Field<'a, B>) -> Iter,
        Iter: FieldSearchIter<'a, B::Index>,
    {
        for index in self.iter().flat_map(|f| map_fields(f).into()) {
            self.buffer.push(index)
        }
    }

    fn new_via_pred<F>(&mut self, predicate: F)
    where
        F: Fn(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        for f in self
            .iter()
            .flat_map(|f| f.neighbors().filter(|&f| predicate(f)))
        {
            self.buffer.push(f.index())
        }
    }

    fn extend_helper(&mut self) -> bool {
        let mut success = false;
        for &i in self.buffer.iter() {
            if !self.board.contains(i) {
                panic!("Field with invalid index: {:?}", i);
            }
            if self.base_set.insert(i) {
                success = true;
            }
        }
        self.buffer.clear();
        success
    }

    // fn replace_helper(&mut self, fields: FieldSearchResult<B::Index>) {
    //     self.base_set.clear();
    //     for i in fields.iter() {
    //         self.base_insert(i);
    //     }
    // }
}

#[derive(Debug, Clone)]
pub struct Iter<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    board: &'a B,
    iter: M::Iter,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Iterator for Iter<'a, M, B> {
    type Item = Field<'a, B>;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.iter.next()?;
        // unwrap: index is required to be valid
        Some(self.board.get_field(idx).unwrap())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> ExactSizeIterator
    for Iter<'a, M, B>
{
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> IntoIterator
    for SearchingSet<'a, M, B>
{
    type Item = Field<'a, B>;
    type IntoIter = Iter<'a, M, B>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, B: BoardToMap<()> + 'a> FromIterator<Field<'a, B>>
    for Option<SearchingSet<'a, B::Map, B>>
{
    fn from_iter<T: IntoIterator<Item = Field<'a, B>>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        let first = iter.next()?;
        let mut set = first.search();
        for field in iter {
            set.insert(field.index());
        }
        Some(set)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        concrete_boards::{matrix_board::*, vec_board::*},
        structures::{
            directions::{BinaryDirection, GridDirection},
            WrappedOffsetStructure,
        },
        Board,
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
