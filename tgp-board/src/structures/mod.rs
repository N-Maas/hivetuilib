mod adjacency_set;
mod direction_structures;
pub mod directions;

pub use adjacency_set::*;
pub use direction_structures::*;

use crate::trait_definitions::Board;

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
