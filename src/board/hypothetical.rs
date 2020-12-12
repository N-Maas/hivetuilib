use super::{Board, BoardIndexable, BoardToMap, ContiguousBoard, Emptyable, Field, IndexMap};

#[derive(Debug, PartialEq, Eq)]
pub struct Hypothetical<'a, T, B: BoardToMap<T, Content = T>> {
    board: &'a B,
    map: B::Map,
}

impl<'a, T, B: BoardToMap<T, Content = T>> Hypothetical<'a, T, B> {
    // TODO: is panicking a good idea?
    fn assert_contained(&self, index: B::Index) {
        if !self.board.contains(index) {
            panic!("invalid index: {:?}", index)
        }
    }

    pub fn original_board(&self) -> &'a B {
        self.board
    }

    pub fn replace(&mut self, index: impl Into<B::Index>, el: T) {
        let index = index.into();
        self.assert_contained(index);
        self.map.insert(index, el);
    }

    pub fn from_board(board: &'a B) -> Self {
        Hypothetical {
            board,
            map: board.get_index_map(),
        }
    }

    pub fn from_field(field: Field<'a, B>) -> Self {
        Hypothetical {
            board: field.board(),
            map: field.board().get_index_map(),
        }
    }
}

impl<T, B: BoardToMap<T, Content = T>> Hypothetical<'_, T, B>
where
    T: Emptyable,
{
    pub fn clear_field(&mut self, index: impl Into<B::Index>) {
        let index = index.into();
        self.assert_contained(index);
        self.map.insert(index, Default::default());
    }

    pub fn apply_move(&mut self, from: impl Into<B::Index>, to: impl Into<B::Index>)
    where
        T: Clone,
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

impl<'a, T, B: BoardToMap<T, Content = T>> Clone for Hypothetical<'a, T, B>
where
    B::Map: Clone,
{
    fn clone(&self) -> Self {
        Self {
            board: self.board,
            map: self.map.clone(),
        }
    }
}

impl<T, B: BoardToMap<T, Content = T>> BoardIndexable for Hypothetical<'_, T, B> {
    type Index = B::Index;

    fn all_indices(&self) -> Vec<Self::Index> {
        self.board.all_indices()
    }
}

impl<T, B: BoardToMap<T, Content = T>> Board for Hypothetical<'_, T, B> {
    type Content = T;
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

impl<T, B: BoardToMap<T, Content = T> + ContiguousBoard> ContiguousBoard for Hypothetical<'_, T, B>
where
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

impl<T, B, E> BoardToMap<E> for Hypothetical<'_, T, B>
where
    B: BoardToMap<T, Content = T> + BoardToMap<E>,
{
    type Map = <B as BoardToMap<E>>::Map;

    fn get_index_map(&self) -> Self::Map {
        BoardToMap::<E>::get_index_map(self.board)
    }
}
