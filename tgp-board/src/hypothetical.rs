use std::ops::{Index, IndexMut};

use super::{
    Board, BoardIndexable, BoardMut, BoardToMap, ContiguousBoard, Emptyable, Field, IndexMap,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Hypothetical<'a, B: Board, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
{
    board: &'a B,
    map: M,
}

impl<'a, T, B: BoardToMap<T, Content = T>> Hypothetical<'a, B, B::Map> {
    pub fn from_board(board: &'a B) -> Self {
        Self::with_index_map(board, board.get_index_map())
    }

    pub fn from_field(field: Field<'a, B>) -> Self {
        Self::from_board(field.board())
    }
}

impl<'a, B: Board, M> Hypothetical<'a, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
{
    pub fn with_index_map(board: &'a B, map: M) -> Self {
        Self { board, map }
    }

    pub fn original_board(&self) -> &'a B {
        self.board
    }

    pub fn set_field(&mut self, index: impl Into<B::Index>, el: B::Content) {
        let index = index.into();
        self.assert_contained(index);
        self.map.insert(index, el);
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    // TODO: is panicking a good idea?
    fn assert_contained(&self, index: B::Index) {
        if !self.board.contains(index) {
            panic!("invalid index: {:?}", index)
        }
    }
}

impl<B: Board, M> Hypothetical<'_, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
    B::Content: Emptyable,
{
    pub fn clear_field(&mut self, index: impl Into<B::Index>) {
        let index = index.into();
        self.assert_contained(index);
        self.map.insert(index, Default::default());
    }

    pub fn apply_move(&mut self, from: impl Into<B::Index>, to: impl Into<B::Index>)
    where
        B::Content: Clone,
    {
        let from = from.into();
        let to = to.into();
        self.assert_contained(from);
        self.assert_contained(to);
        let value = self
            .map
            .insert(from, Default::default())
            // unwrap: correct because checked previously
            .unwrap_or_else(|| self.board.get(from).unwrap().clone());
        self.map.insert(to, value);
    }
}

impl<'a, B: Board, M> Clone for Hypothetical<'a, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
    M: Clone,
{
    fn clone(&self) -> Self {
        Self {
            board: self.board,
            map: self.map.clone(),
        }
    }
}

impl<B: Board, M> BoardIndexable for Hypothetical<'_, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
{
    type Index = B::Index;

    fn all_indices(&self) -> Vec<Self::Index> {
        self.board.all_indices()
    }
}

impl<B: Board, M> Board for Hypothetical<'_, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
{
    type Content = B::Content;
    type Structure = B::Structure;

    fn size(&self) -> usize {
        self.board.size()
    }

    fn contains(&self, index: Self::Index) -> bool {
        self.board.contains(index)
    }

    fn structure(&self) -> &Self::Structure {
        self.board.structure()
    }

    fn get(&self, index: Self::Index) -> Option<&Self::Content> {
        self.map.get(index).or_else(|| self.board.get(index))
    }
}

impl<B: Board, M> BoardMut for Hypothetical<'_, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
    B::Content: Clone,
{
    fn get_mut(&mut self, index: Self::Index) -> Option<&mut Self::Content> {
        self.board.get(index).and_then(move |content| {
            self.map.insert(index, content.clone());
            self.map.get_mut(index)
        })
    }
}

impl<T, I, B: Board<Content = T>, M> Index<I> for Hypothetical<'_, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
    B: Index<I>,
{
    type Output = B::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.board.index(index)
    }
}

impl<T, I, B: Board<Content = T>, M> IndexMut<I> for Hypothetical<'_, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
    B: IndexMut<I, Output = T>,
    I: Into<B::Index>,
    B::Content: Clone,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.get_mut(index.into()).expect("Invalid index.")
    }
}

impl<B: Board, M> ContiguousBoard for Hypothetical<'_, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
    B: ContiguousBoard,
    <B as BoardIndexable>::Index: PartialOrd,
{
    type Offset = B::Offset;

    fn bound(&self) -> Self::Index {
        self.board.bound()
    }

    fn wrapped(&self, index: Self::Offset) -> Self::Index {
        self.board.wrapped(index)
    }
}

impl<B: Board, M, E> BoardToMap<E> for Hypothetical<'_, B, M>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
    B: BoardToMap<E>,
{
    type Map = <B as BoardToMap<E>>::Map;

    fn get_index_map(&self) -> Self::Map {
        BoardToMap::<E>::get_index_map(self.board)
    }
}
