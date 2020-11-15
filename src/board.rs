pub mod directions;
pub mod matrix_board;
pub mod search;
pub mod structures;
pub mod vec_board;

use std::{
    cmp::Ordering, collections::HashSet, fmt::Debug, hash::Hash, iter::Copied, marker::PhantomData,
    slice::Iter, vec::IntoIter,
};

// ----- trait definitions -----

pub trait BoardIdxType: Copy + Eq + Debug {}

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

    fn get_mut(&mut self, index: Self::Index) -> Option<&mut Self::Content>;

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

    pub fn board(&self) -> &'a B {
        self.board
    }

    pub fn index(&self) -> B::Index {
        self.index
    }

    pub fn content(&self) -> &'a B::Content {
        self.content_checked().expect(&format!(
            "Index of field is invalid: {:?} - perhaps the field was removed from the board?",
            self.index
        ))
    }

    pub fn content_checked(&self) -> Option<&'a B::Content> {
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
        f.write_fmt(format_args!("Field {{ index: {:?} }}", self.index))
    }
}

impl<'a, T, B: Board<Content = Option<T>>> Field<'a, B> {
    pub fn is_empty(&self) -> bool {
        self.content().is_none()
    }
}

impl<'a, S, B: Board<Structure = S>> Field<'a, B>
where
    S: AdjacencyStructure<B>,
{
    pub fn is_adjacent_to(&self, index: B::Index) -> bool {
        self.board
            .structure()
            .is_adjacent(self.board, self.index, index)
    }

    pub fn is_adjacent(&self, other: &Self) -> bool {
        self.is_adjacent_to(other.index)
    }
}

impl<'a, S, B: Board<Structure = S>> Field<'a, B>
where
    S: NeighborhoodStructure<B>,
{
    pub fn neighbor_count(&self) -> usize {
        self.board
            .structure()
            .neighbor_count(self.board, self.index)
    }

    pub fn neighbors(&self) -> impl Iterator<Item = Field<'a, B>> {
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
    pub fn next(&self, direction: S::Direction) -> Option<Self> {
        let board = self.board;
        board
            .structure()
            .next(board, self.index, direction)
            .and_then(|i| Self::new(board, i))
    }
}

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
