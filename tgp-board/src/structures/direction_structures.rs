use std::{
    fmt::{self, Debug},
    marker::PhantomData,
};

use crate::trait_definitions::{Board, ContiguousBoard};

use super::{
    directions::{DirectionEnumerable, DirectionOffset, OffsetableIndex},
    AdjacencyStructure, DirectionStructure, NeighborhoodStructure,
};

// ----- macros for simpler implementation of direction structures -----
macro_rules! implAdjacencyStructure {
    () => {
        fn is_adjacent(&self, board: &B, i: B::Index, j: B::Index) -> bool {
            D::enumerate_all()
                .filter_map(|d| self.next(board, i, d))
                .any(|index| index == j)
        }
    };
}

macro_rules! implNeighborhoodStructure {
    () => {
        #[inline(always)]
        fn neighbor_count(&self, board: &B, index: B::Index) -> usize {
            D::enumerate_all()
                .filter_map(|d| self.next(board, index, d))
                .filter(|i| board.contains(*i))
                .count()
        }

        #[inline(always)]
        fn neighbors<'a>(
            &'a self,
            board: &'a B,
            index: B::Index,
        ) -> impl Iterator<Item = B::Index> + 'a {
            D::enumerate_all()
                .filter_map(move |d| self.next(board, index, d))
                .filter(|i| board.contains(*i))
        }
    };
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct OffsetStructure<I: OffsetableIndex, D: DirectionOffset<I::Offset>> {
    _i: PhantomData<I>,
    _d: PhantomData<D>,
}

impl<I: OffsetableIndex, D: DirectionOffset<I::Offset>> OffsetStructure<I, D> {
    pub fn new() -> Self {
        Self {
            _i: PhantomData,
            _d: PhantomData,
        }
    }
}

impl<I: OffsetableIndex, D: DirectionOffset<I::Offset>> Debug for OffsetStructure<I, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OffsetStructure")
    }
}

impl<B: Board, D: DirectionOffset<<B::Index as OffsetableIndex>::Offset>> DirectionStructure<B>
    for OffsetStructure<B::Index, D>
where
    B::Index: OffsetableIndex,
{
    type Direction = D;

    #[inline(always)]
    fn next(&self, board: &B, index: B::Index, direction: D) -> Option<B::Index> {
        B::Index::from_offset(index.apply_offset(direction.offset())).filter(|i| board.contains(*i))
    }
}

// TODO: good ideas? (might be inperformant)
impl<B: Board, D: DirectionOffset<<B::Index as OffsetableIndex>::Offset>> AdjacencyStructure<B>
    for OffsetStructure<B::Index, D>
where
    B::Index: OffsetableIndex,
    D: DirectionEnumerable,
{
    implAdjacencyStructure!();
}

impl<B: Board, D: DirectionOffset<<B::Index as OffsetableIndex>::Offset>> NeighborhoodStructure<B>
    for OffsetStructure<B::Index, D>
where
    B::Index: OffsetableIndex,
    D: DirectionEnumerable,
{
    implNeighborhoodStructure!();
}

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub struct WrappedOffsetStructure<I: OffsetableIndex + PartialOrd, D: DirectionOffset<I::Offset>> {
    _i: PhantomData<I>,
    _d: PhantomData<D>,
}

impl<I: OffsetableIndex + PartialOrd, D: DirectionOffset<I::Offset>> WrappedOffsetStructure<I, D> {
    pub fn new() -> Self {
        Self {
            _i: PhantomData,
            _d: PhantomData,
        }
    }
}

impl<I: OffsetableIndex + PartialOrd, D: DirectionOffset<I::Offset>> Debug
    for WrappedOffsetStructure<I, D>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WrappedOffsetStructure")
    }
}

impl<B: ContiguousBoard, D: DirectionOffset<<B::Index as OffsetableIndex>::Offset>>
    DirectionStructure<B> for WrappedOffsetStructure<B::Index, D>
where
    B::Index: OffsetableIndex<Offset = B::Offset> + PartialOrd,
{
    type Direction = D;

    #[inline(always)]
    fn next(&self, board: &B, index: B::Index, direction: D) -> Option<B::Index> {
        Some(board.wrapped(index.apply_offset(direction.offset())))
    }
}

// TODO: good ideas? (might be inperformant)
impl<B: ContiguousBoard, D: DirectionOffset<<B::Index as OffsetableIndex>::Offset>>
    AdjacencyStructure<B> for WrappedOffsetStructure<B::Index, D>
where
    B::Index: OffsetableIndex<Offset = B::Offset> + PartialOrd,
    D: DirectionEnumerable,
{
    implAdjacencyStructure!();
}

impl<B: ContiguousBoard, D: DirectionOffset<<B::Index as OffsetableIndex>::Offset>>
    NeighborhoodStructure<B> for WrappedOffsetStructure<B::Index, D>
where
    B::Index: OffsetableIndex<Offset = B::Offset> + PartialOrd,
    D: DirectionEnumerable,
{
    implNeighborhoodStructure!();
}
