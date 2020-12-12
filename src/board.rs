pub mod directions;
pub mod hypothetical;
pub mod search;
pub mod structures;

pub mod matrix_board;
pub mod open_board;
pub mod vec_board;

use std::{
    cmp::Ordering, collections::HashSet, fmt::Debug, hash::Hash, iter::Copied, marker::PhantomData,
    mem, slice::Iter, vec::IntoIter,
};

use directions::DirectionEnumerable;

use self::hypothetical::Hypothetical;

// ----- trait definitions -----

pub trait BoardIdxType: Copy + Eq + Debug {}

// TODO: do not use impl Trait returns
// TODO: replace occurences with Into<index> - general solution for mutable access coming from field?
pub trait Board: BoardIndexable {
    type Content;
    type Structure;

    fn size(&self) -> usize;

    fn contains(&self, index: Self::Index) -> bool {
        self.get(index).is_some()
    }

    fn structure(&self) -> &Self::Structure;

    // TODO better get_field_unchecked or similar?
    fn get_field_unchecked<'a>(&'a self, index: Self::Index) -> Field<'a, Self>
    where
        Self: Sized,
    {
        self.get_field(index)
            .expect(&format!("invalid index: {:?}", index))
    }

    fn get_field<'a>(&'a self, index: Self::Index) -> Option<Field<'a, Self>>
    where
        Self: Sized,
    {
        Field::new(self, index)
    }

    fn get(&self, index: Self::Index) -> Option<&Self::Content>;

    fn iter_fields<'a>(
        &'a self,
    ) -> <&'a Self as BoardIntoFieldIter<Self::Index, Self::Content>>::IntoIter
    where
        Self: Sized,
        Self::Content: 'a,
    {
        self.into_field_iter()
    }

    // TODO: required?
    // TODO: iter_mut impossible to define in trait currently
    fn iter<'a>(&'a self) -> <&'a Self as BoardIntoIter<Self::Index, Self::Content>>::IntoIter
    where
        Self: Sized,
        Self::Content: 'a,
    {
        self.into_iter()
    }
}

pub trait BoardMut: Board {
    // TODO: convenience methods? FieldMut API?

    fn get_mut(&mut self, index: Self::Index) -> Option<&mut Self::Content>;
}

// TODO impl Index possible?

macro_rules! implBoardIntoIter {
    ($trait:ident for $name:ident, $call:ident, $out:ty, $access:ident) => {
        pub trait $trait<I, T> {
            type Output;
            type IntoIter: Iterator<Item = Self::Output>;

            fn $call(self) -> Self::IntoIter;
        }

        impl<'a, B: Board> $trait<B::Index, B::Content> for &'a B
        where
            B::Content: 'a,
        {
            type Output = $out;
            type IntoIter = $name<'a, B>;

            fn $call(self) -> Self::IntoIter {
                Self::IntoIter {
                    board: self,
                    iter: self.all_indices().into_iter(),
                    _f: PhantomData,
                }
            }
        }

        pub struct $name<'a, B: Board> {
            board: &'a B,
            iter: IntoIter<B::Index>,
            _f: PhantomData<B::Content>,
        }

        impl<'a, B: Board> Iterator for $name<'a, B>
        where
            B::Content: 'a,
        {
            type Item = $out;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(|idx| self.board.$access(idx).unwrap())
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }
    };
}

implBoardIntoIter!(BoardIntoFieldIter for FieldIter, into_field_iter, Field<'a, B>, get_field);

implBoardIntoIter!(BoardIntoIter for BoardIter, into_iter, &'a B::Content, get);

// ----- extended board types -----

// TODO do these methods belong together?
pub trait ContiguousBoard: Board
where
    Self::Index: PartialOrd,
{
    type Offset;

    // should return a smallest common bound, i.e. i < b.bound() for a board b and every i with b.contains(i)
    // TODO: is this required at all? Add minimum?
    fn bound(&self) -> Self::Index;

    fn wrapped(&self, index: Self::Offset) -> Self::Index;

    // TODO: get_wrapped etc. helper functions?
}

// TOOD rather bad hack to enable iteration - enforce lifetime binding to self?
// #[unstable]
pub trait BoardIndexable {
    type Index: BoardIdxType;

    fn all_indices(&self) -> Vec<Self::Index>;

    // fn enumerate_mut(&mut self) -> Vec<(I, &mut Self::Content)>;
}

// ----- field implementation -----

#[derive(Eq)]
pub struct Field<'a, B: Board> {
    board: &'a B,
    index: B::Index,
}

impl<'a, B: Board> Field<'a, B> {
    pub fn new(board: &'a B, index: B::Index) -> Option<Self> {
        if board.contains(index) {
            Some(Self { board, index })
        } else {
            None
        }
    }

    pub fn board(self) -> &'a B {
        self.board
    }

    pub fn index(self) -> B::Index {
        self.index
    }

    pub fn content(self) -> &'a B::Content {
        self.content_checked().expect(&format!(
            "Index of field is invalid: {:?} - perhaps the field was removed from the board?",
            self.index
        ))
    }

    pub fn content_checked(self) -> Option<&'a B::Content> {
        self.board.get(self.index)
    }
}

// TODO good idea to compare pointer?
impl<'a, B: Board> PartialEq for Field<'a, B> {
    fn eq(&self, other: &Self) -> bool {
        (self.board as *const B == other.board as *const B) && self.index == other.index
    }
}

impl<'a, B: Board> Clone for Field<'a, B> {
    fn clone(&self) -> Self {
        Field { ..*self }
    }
}

impl<'a, B: Board> Copy for Field<'a, B> {}

impl<B: Board> Debug for Field<'_, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: make it pretty
        f.write_fmt(format_args!("Field {{ index: {:?} }}", self.index))
    }
}

impl<'a, B: Board> Field<'a, B>
where
    B::Content: Emptyable,
{
    pub fn is_empty(self) -> bool {
        self.content().call_field_is_empty()
    }
}

impl<'a, S, B: Board<Structure = S>> Field<'a, B>
where
    S: AdjacencyStructure<B>,
{
    pub fn is_adjacent<T: Into<B::Index>>(self, index: T) -> bool {
        self.board
            .structure()
            .is_adjacent(self.board, self.index, index.into())
    }
}

impl<'a, S, B: Board<Structure = S>> Field<'a, B>
where
    S: NeighborhoodStructure<B>,
{
    pub fn neighbor_count(self) -> usize {
        self.board
            .structure()
            .neighbor_count(self.board, self.index)
    }

    pub fn neighbors(self) -> impl Iterator<Item = Field<'a, B>> {
        let board = self.board;
        board
            .structure()
            .neighbors(board, self.index)
            .into_iter()
            .filter_map(move |i| Self::new(board, i))
    }
}

impl<'a, S, B: Board<Structure = S>> Field<'a, B>
where
    S: DirectionStructure<B>,
{
    pub fn next(self, direction: S::Direction) -> Option<Self> {
        let board = self.board;
        board
            .structure()
            .next(board, self.index, direction)
            .and_then(|i| Self::new(board, i))
    }

    pub fn has_next(self, direction: S::Direction) -> bool {
        let board = self.board;
        board.structure().has_next(board, self.index, direction)
    }

    pub fn neighbors_by_direction(self) -> impl Iterator<Item = (S::Direction, Field<'a, B>)>
    where
        S::Direction: DirectionEnumerable,
    {
        S::Direction::enumerate_all().filter_map(move |d| self.next(d).map(|f| (d, f)))
    }
}

impl<'a, T, B: BoardToMap<T, Content = T>> Field<'a, Hypothetical<'a, T, B>> {
    pub fn original_field<'b>(&self, board: &'b B) -> Field<'b, B> {
        Field::new(board, self.index).expect(&format!(
            "Index of field is invalid for original board: {:?}",
            self.index
        ))
    }
}

// ----- structure traits -----

pub trait AdjacencyStructure<B: Board> {
    fn is_adjacent(&self, board: &B, i: B::Index, j: B::Index) -> bool;
}

pub trait NeighborhoodStructure<B: Board> {
    fn neighbor_count(&self, board: &B, index: B::Index) -> usize {
        self.neighbors(board, index).len()
    }

    // TODO more efficient than vec?
    fn neighbors(&self, board: &B, index: B::Index) -> Vec<B::Index>;
}

pub trait DirectionStructure<B: Board> {
    type Direction: Copy + Eq;

    fn has_next(&self, board: &B, index: B::Index, direction: Self::Direction) -> bool {
        self.next(board, index, direction).is_some()
    }

    fn next(&self, board: &B, index: B::Index, direction: Self::Direction) -> Option<B::Index>;
}

// ----- index map -----

pub trait IndexMap {
    type IndexType: BoardIdxType;
    type Item;
    type Iter: ExactSizeIterator<Item = Self::IndexType>;

    // fn from_board<B: Board<Self::IndexType>>(board: &'a B) -> Self;
    fn size(&self) -> usize;

    fn contains(&self, i: Self::IndexType) -> bool {
        self.get(i).is_some()
    }

    fn get(&self, i: Self::IndexType) -> Option<&Self::Item>;

    fn get_mut(&mut self, i: Self::IndexType) -> Option<&mut Self::Item>;

    /// Returns the old value if the key was already present.
    fn insert(&mut self, i: Self::IndexType, el: Self::Item) -> Option<Self::Item>;

    fn iter_indices(&self) -> Self::Iter;

    fn clear(&mut self);

    // TODO: subset and further helper methods?
}

pub trait BoardToMap<T>: Board {
    type Map: IndexMap<Item = T, IndexType = Self::Index>;

    fn get_index_map(&self) -> Self::Map;
}

// ----- field -----

/// This trait is <b>not</b> intended to be used directly.
/// It is used to probide generic access function on a higher level (e.g. for Fields).
pub trait Emptyable: Default {
    fn call_field_is_empty(&self) -> bool;

    fn call_take_field(&mut self) -> Self {
        mem::take(self)
    }

    fn call_clear_field(&mut self) {
        self.call_take_field();
    }
}

impl<T> Emptyable for Option<T> {
    fn call_field_is_empty(&self) -> bool {
        self.is_none()
    }
}

impl<T> Emptyable for Vec<T> {
    fn call_field_is_empty(&self) -> bool {
        self.is_empty()
    }

    fn call_clear_field(&mut self) {
        self.clear()
    }
}
