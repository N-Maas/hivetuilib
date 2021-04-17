use std::marker::PhantomData;

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
        fn neighbor_count(&self, board: &B, index: B::Index) -> usize {
            D::enumerate_all()
                .filter_map(|d| self.next(board, index, d))
                .filter(|i| board.contains(*i))
                .count()
        }

        fn neighbors(&self, board: &B, index: B::Index) -> Vec<B::Index> {
            D::enumerate_all()
                .filter_map(|d| self.next(board, index, d))
                .filter(|i| board.contains(*i))
                .collect()
        }
    };
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
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

impl<B: Board, D: DirectionOffset<<B::Index as OffsetableIndex>::Offset>> DirectionStructure<B>
    for OffsetStructure<B::Index, D>
where
    B::Index: OffsetableIndex,
{
    type Direction = D;

    fn next(&self, board: &B, index: B::Index, direction: D) -> Option<B::Index> {
        B::Index::from_offset(index.apply_offset(direction.get_offset()))
            .filter(|i| board.contains(*i))
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

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
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

impl<B: ContiguousBoard, D: DirectionOffset<<B::Index as OffsetableIndex>::Offset>>
    DirectionStructure<B> for WrappedOffsetStructure<B::Index, D>
where
    B::Index: OffsetableIndex<Offset = B::Offset> + PartialOrd,
{
    type Direction = D;

    fn next(&self, board: &B, index: B::Index, direction: D) -> Option<B::Index> {
        Some(board.wrapped(index.apply_offset(direction.get_offset())))
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
