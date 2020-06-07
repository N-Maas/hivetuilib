pub mod board_impl;
pub mod structures;

use std::{
    collections::HashSet, fmt::Debug, hash::Hash, iter::Copied, marker::PhantomData, ops::IndexMut,
    slice::Iter, vec::IntoIter,
};

// ----- trait definitions -----

pub trait BoardIdxType: Copy + Eq + Debug {}

pub trait Board<I: BoardIdxType>: BoardIndex<I> {
    type Structure;

    fn size(&self) -> usize;

    fn contains(&self, index: I) -> bool;

    fn structure(&self) -> &Self::Structure;

    // TODO better get_field_unchecked or similar?
    fn field_at<'a>(&'a self, index: I) -> Field<'a, I, Self> {
        self.get_field(index)
            .expect(&format!("invalid index: {:?}", index))
    }

    fn get_field<'a>(&'a self, index: I) -> Option<Field<'a, I, Self>> {
        Field::new(self, index)
    }

    fn get(&self, index: I) -> Option<&Self::Output> {
        if self.contains(index) {
            Some(self.index(index))
        } else {
            None
        }
    }

    fn get_mut(&mut self, index: I) -> Option<&mut Self::Output> {
        if self.contains(index) {
            Some(self.index_mut(index))
        } else {
            None
        }
    }

    fn iter_fields<'a>(&'a self) -> <&'a Self as BoardIntoFieldIter<I, Self::Output>>::IntoIter
    where
        Self::Output: 'a,
    {
        self.into_field_iter()
    }

    // TODO: required?
    fn iter<'a>(&'a self) -> <&'a Self as BoardIntoIter<I, Self::Output>>::IntoIter
    where
        Self::Output: 'a,
    {
        self.into_iter()
    }
}

// TODO impl Index possible?

macro_rules! implBoardIntoIter {
    ($trait:ident for $name:ident, $call:ident, $out:ty, $access:ident) => {
        pub trait $trait<I, T: ?Sized> {
            type Output;
            type IntoIter: Iterator<Item = Self::Output>;

            fn $call(self) -> Self::IntoIter;
        }

        impl<'a, I: BoardIdxType, T, B: Board<I, Output = T> + ?Sized> $trait<I, T> for &'a B
        where
            T: 'a + ?Sized,
        {
            type Output = $out;
            type IntoIter = $name<'a, I, T, B>;

            fn $call(self) -> Self::IntoIter {
                Self::IntoIter {
                    board: self,
                    iter: self.all_indices().into_iter(),
                    _f: PhantomData,
                }
            }
        }

        pub struct $name<'a, I: BoardIdxType, T: ?Sized, B: Board<I, Output = T> + ?Sized> {
            board: &'a B,
            iter: IntoIter<I>,
            _f: PhantomData<T>,
        }

        impl<'a, I: BoardIdxType, T, B: Board<I, Output = T> + ?Sized> Iterator
            for $name<'a, I, T, B>
        where
            T: 'a + ?Sized,
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

implBoardIntoIter!(BoardIntoFieldIter for FieldIter, into_field_iter, Field<'a, I, B>, get_field);

implBoardIntoIter!(BoardIntoIter for BoardIter, into_iter, &'a T, get);

// ----- extended board types -----

// TODO spelling?
pub trait ContiguousBoard<I: BoardIdxType + Ord>: Board<I> {
    fn max(&self) -> I;

    fn wrapped(&self, index: I) -> I;

    // TODO: get_wrapped etc. helper functions?
}

// ----- index type -----

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Index1D {
    pub val: usize,
}

impl BoardIdxType for Index1D {}

impl From<usize> for Index1D {
    fn from(val: usize) -> Self {
        Self { val }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index2D {
    pub x: usize,
    pub y: usize,
}

impl BoardIdxType for Index2D {}

// TOOD rather bad hack to enable iteration - enforce lifetime binding to self?
// TODO should we use IndexMut?
// #[unstable]
pub trait BoardIndex<I: BoardIdxType>: IndexMut<I> {
    fn all_indices(&self) -> Vec<I>;

    // fn enumerate_mut(&mut self) -> Vec<(I, &mut Self::Output)>;
}

// ----- field implementation -----

#[derive(Debug, Eq, Copy)]
pub struct Field<'a, I: BoardIdxType, B: Board<I> + ?Sized> {
    board: &'a B,
    index: I,
}

impl<'a, I: BoardIdxType, B: Board<I> + ?Sized> Field<'a, I, B> {
    pub fn new(board: &'a B, index: I) -> Option<Self> {
        if board.contains(index) {
            Some(Self { board, index })
        } else {
            None
        }
    }

    pub fn index(&self) -> I {
        self.index
    }

    pub fn content(&self) -> &B::Output {
        self.content_checked().expect(&format!(
            "Index of field is invalid: {:?} - perhaps the field was removed from the board?",
            self.index
        ))
    }

    pub fn content_checked(&self) -> Option<&B::Output> {
        self.board.get(self.index)
    }
}

// TODO good idea to compare pointer?
impl<'a, I: BoardIdxType, B: Board<I> + ?Sized> PartialEq for Field<'a, I, B> {
    fn eq(&self, other: &Self) -> bool {
        (self.board as *const B == other.board as *const B) && self.index == other.index
    }
}

impl<'a, I: BoardIdxType, B: Board<I> + ?Sized> Clone for Field<'a, I, B> {
    fn clone(&self) -> Self {
        Field { ..*self }
    }
}

impl<'a, I: BoardIdxType, S, B: Board<I, Structure = S> + ?Sized> Field<'a, I, B>
where
    S: AdjacencyStructure<I, B>,
{
    pub fn is_adjacent_to(&self, index: I) -> bool {
        self.board
            .structure()
            .is_adjacent(self.board, self.index, index)
    }

    pub fn is_adjacent(&self, other: &Self) -> bool {
        self.is_adjacent_to(other.index)
    }
}

impl<'a, I: BoardIdxType, S, B: Board<I, Structure = S> + ?Sized> Field<'a, I, B>
where
    S: NeighborhoodStructure<I, B>,
{
    pub fn neighbor_count(&self) -> usize {
        self.board
            .structure()
            .neighbor_count(self.board, self.index)
    }

    pub fn get_neighbors(&self) -> impl Iterator<Item = Field<'a, I, B>> {
        let board = self.board;
        board
            .structure()
            .get_neighbors(board, self.index)
            .into_iter()
            .filter_map(move |i| Self::new(board, i))
    }
}

impl<'a, I: BoardIdxType, S, B: Board<I, Structure = S> + ?Sized, D: Copy + Eq> Navigable<D>
    for Field<'a, I, B>
where
    S: DirectionStructure<I, B, D>,
{
    fn next(&self, direction: D) -> Option<Self> {
        let board = self.board;
        board
            .structure()
            .next(board, self.index, direction)
            .and_then(|i| Self::new(board, i))
    }
}

pub trait AdjacencyStructure<I: BoardIdxType, B: Board<I> + ?Sized> {
    fn is_adjacent(&self, board: &B, i: I, j: I) -> bool;
}

pub trait NeighborhoodStructure<I: BoardIdxType, B: Board<I> + ?Sized> {
    fn neighbor_count(&self, board: &B, index: I) -> usize;

    // TODO more efficient than vec?
    fn get_neighbors(&self, board: &B, index: I) -> Vec<I>;
}

pub trait DirectionStructure<I: BoardIdxType, B: Board<I> + ?Sized, D: Copy + Eq> {
    fn has_next(&self, board: &B, index: I, direction: D) -> bool {
        self.next(board, index, direction).is_some()
    }

    fn next(&self, board: &B, index: I, direction: D) -> Option<I>;
}

pub trait Navigable<D: Copy + Eq>: Sized {
    fn has_next(&self, direction: D) -> bool {
        self.next(direction).is_some()
    }

    fn next(&self, direction: D) -> Option<Self>;
}

// ----- directions -----

pub mod directions {
    use super::*;

    pub trait DirectionOffset<O>: Copy + Eq {
        fn get_offset(&self) -> O;
    }

    pub trait DirectionReversable: Copy + Eq {
        fn reversed(&self) -> Self;
    }

    pub trait DirectionEnumerable: Copy + Eq + Sized {
        type Iter: Iterator<Item = Self>;

        fn enumerate_all() -> Self::Iter;
    }

    // TODO: trait for direction -> index mapping (efficient structure)
    // TODO: derive macro for Enumerable/index mapping

    pub enum Offset {
        Neg(usize),
        Pos(usize),
    }

    fn apply_offset(n: usize, offset: Offset) -> Option<usize> {
        match offset {
            Offset::Pos(offset) => n.checked_add(offset),
            Offset::Neg(offset) => n.checked_sub(offset),
        }
    }

    pub trait OffsetableIndex: BoardIdxType {
        type Offset;

        fn apply_offset(&self, offset: Self::Offset) -> Option<Self>;
    }

    impl OffsetableIndex for Index1D {
        type Offset = Offset;

        fn apply_offset(&self, offset: Offset) -> Option<Self> {
            apply_offset(self.val, offset).map(|val| Self::from(val))
        }
    }

    impl OffsetableIndex for Index2D {
        type Offset = (Offset, Offset);

        fn apply_offset(&self, offset: (Offset, Offset)) -> Option<Self> {
            let x = apply_offset(self.x, offset.0)?;
            let y = apply_offset(self.y, offset.1)?;
            Some(Self { x, y })
        }
    }

    // ----- direction implementations -----

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum BinaryDirection {
        Forward,
        Backward,
    }

    impl DirectionOffset<Offset> for BinaryDirection {
        fn get_offset(&self) -> Offset {
            match self {
                BinaryDirection::Forward => Offset::Pos(1),
                BinaryDirection::Backward => Offset::Neg(1),
            }
        }
    }

    impl DirectionReversable for BinaryDirection {
        fn reversed(&self) -> Self {
            match self {
                BinaryDirection::Forward => BinaryDirection::Backward,
                BinaryDirection::Backward => BinaryDirection::Forward,
            }
        }
    }

    impl DirectionEnumerable for BinaryDirection {
        type Iter = Copied<Iter<'static, BinaryDirection>>;

        fn enumerate_all() -> Self::Iter {
            static DIRS: [BinaryDirection; 2] =
                [BinaryDirection::Forward, BinaryDirection::Backward];
            DIRS.iter().copied()
        }
    }
}
