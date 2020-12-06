use super::{Board, BoardIndexable, BoardToMap, ContiguousBoard, IndexMap};

#[derive(Debug, PartialEq, Eq)]
pub struct Hypothetical<'a, T, B: BoardToMap<T, Content = T>> {
    board: &'a B,
    map: B::Map,
}

impl<T, B: BoardToMap<T, Content = T>> Hypothetical<'_, T, B> {
    // TODO: is panicking a good idea?
    fn assert_contained(&self, index: B::Index) {
        if !self.board.contains(index) {
            panic!("invalid index: {:?}", index)
        }
    }

    pub fn replace(&mut self, index: B::Index, el: T) {
        self.assert_contained(index);
        self.map.insert(index, el);
    }
}

impl<T, B: BoardToMap<Option<T>, Content = Option<T>>> Hypothetical<'_, Option<T>, B> {
    pub fn clear_field(&mut self, index: B::Index) {
        self.assert_contained(index);
        self.map.insert(index, None);
    }

    pub fn apply_move(&mut self, from: B::Index, to: B::Index)
    where
        T: Clone,
    {
        self.assert_contained(from);
        self.assert_contained(to);
        let value = self
            .map
            .insert(from, None)
            // safe because checked previously
            .unwrap_or_else(|| self.board.get(from).unwrap().clone());
        self.map.insert(to, value);
    }
}

impl<'a, T, B: BoardToMap<T, Content = T>> From<&'a B> for Hypothetical<'a, T, B> {
    fn from(board: &'a B) -> Self {
        Self {
            board,
            map: board.get_index_map(),
        }
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
