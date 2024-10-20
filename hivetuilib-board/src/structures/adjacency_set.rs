use crate::trait_definitions::{Board, BoardIdxType};
use std::{collections::HashSet, hash::Hash};

use super::AdjacencyStructure;

#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct AdjacencySet<I: BoardIdxType + Hash> {
    edges: HashSet<(I, I)>,
}

impl<I: BoardIdxType + Hash> AdjacencySet<I> {
    pub fn new() -> Self {
        Self {
            edges: HashSet::new(),
        }
    }

    pub fn add_directed(&mut self, i: I, j: I) {
        self.edges.insert((i, j));
    }

    pub fn add_undirected(&mut self, i: I, j: I) {
        self.edges.insert((i, j));
        self.edges.insert((j, i));
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = &(I, I)> {
        self.edges.iter()
    }
}

impl<B: Board> AdjacencyStructure<B> for AdjacencySet<B::Index>
where
    B::Index: Hash,
{
    fn is_adjacent(&self, _board: &B, i: B::Index, j: B::Index) -> bool {
        self.edges.contains(&(i, j))
    }
}
