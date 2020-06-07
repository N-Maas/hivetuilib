use super::*;

use std::{
    iter,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone)]
pub struct VecBoard<T, S = ()> {
    content: Vec<T>,
    structure: S,
}

impl<T: Clone, S> VecBoard<T, S> {
    pub fn from_default(count: usize, def: T, structure: S) -> Self {
        VecBoard {
            content: vec![def; count],
            structure,
        }
    }
}

impl<T: Default, S> VecBoard<T, S> {
    pub fn with_default(count: usize, structure: S) -> Self {
        VecBoard {
            content: iter::repeat_with(|| Default::default())
                .take(count)
                .collect(),
            structure,
        }
    }
}

// TODO: builder

impl<T, S> Index<Index1D> for VecBoard<T, S> {
    type Output = T;

    fn index(&self, index: Index1D) -> &Self::Output {
        self.content.index(index.val)
    }
}

impl<T, S> IndexMut<Index1D> for VecBoard<T, S> {
    fn index_mut(&mut self, index: Index1D) -> &mut Self::Output {
        self.content.index_mut(index.val)
    }
}

impl<T, S> BoardIndex<Index1D> for VecBoard<T, S> {
    fn all_indices(&self) -> Vec<Index1D> {
        (0..self.content.len())
            .map(|val| Index1D::from(val))
            .collect()
    }
}

impl<T, S> Board<Index1D> for VecBoard<T, S> {
    type Structure = S;

    fn size(&self) -> usize {
        self.content.len()
    }

    fn contains(&self, index: Index1D) -> bool {
        index.val < self.size()
    }

    fn structure(&self) -> &Self::Structure {
        &self.structure
    }
}

impl<T, S> ContiguousBoard<Index1D> for VecBoard<T, S> {
    fn max(&self) -> Index1D {
        Index1D::from(self.content.len())
    }

    fn wrapped(&self, index: Index1D) -> Index1D {
        Index1D::from(index.val % self.content.len())
    }
}

mod test {
    use super::*;

    #[test]
    fn vec_board_test() {
        let board = VecBoard::<usize, ()>::with_default(1, ());
        assert_eq!(board.size(), 1);
        assert_eq!(board.get(Index1D::from(0)), Some(&0));
    }
}
