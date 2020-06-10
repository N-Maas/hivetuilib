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
    pub fn from_value(count: usize, val: T, structure: S) -> Self {
        Self {
            content: vec![val; count],
            structure,
        }
    }
}

impl<T: Default, S> VecBoard<T, S> {
    pub fn with_default(count: usize, structure: S) -> Self {
        Self {
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
    fn bound(&self) -> Index1D {
        Index1D::from(self.content.len())
    }

    fn wrapped(&self, index: Index1D) -> Index1D {
        Index1D::from(index.val % self.content.len())
    }
}

// A two-dimensional immutable board. The fields are saved in a vec internally, calculating the index as necessary.
#[derive(Debug, Clone)]
pub struct MatrixBoard<T, S = ()> {
    content: Vec<T>,
    num_cols: usize,
    num_rows: usize,
    structure: S,
}

impl<T, S> MatrixBoard<T, S> {
    fn calculate_index(&self, index: Index2D) -> Option<usize> {
        if index.x < self.num_cols && index.y < self.num_rows {
            Some(index.y * self.num_rows + index.x)
        } else {
            None
        }
    }

    pub fn num_cols(&self) -> usize {
        self.num_cols
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }
}

impl<T: Clone, S> MatrixBoard<T, S> {
    pub fn from_value(num_cols: usize, num_rows: usize, val: T, structure: S) -> Self {
        Self {
            content: vec![val; num_cols * num_rows],
            num_cols,
            num_rows,
            structure,
        }
    }
}

impl<T: Default, S> MatrixBoard<T, S> {
    pub fn with_default(num_cols: usize, num_rows: usize, structure: S) -> Self {
        Self {
            content: iter::repeat_with(|| Default::default())
                .take(num_cols * num_rows)
                .collect(),
            num_cols,
            num_rows,
            structure,
        }
    }
}

impl<T, S> Index<Index2D> for MatrixBoard<T, S> {
    type Output = T;

    fn index(&self, index: Index2D) -> &Self::Output {
        let idx = self.calculate_index(index).unwrap();
        self.content.index(idx)
    }
}

impl<T, S> IndexMut<Index2D> for MatrixBoard<T, S> {
    fn index_mut(&mut self, index: Index2D) -> &mut Self::Output {
        let idx = self.calculate_index(index).unwrap();
        self.content.index_mut(idx)
    }
}

impl<T, S> BoardIndex<Index2D> for MatrixBoard<T, S> {
    fn all_indices(&self) -> Vec<Index2D> {
        (0..self.num_cols)
            .flat_map(|x| (0..self.num_rows).map(move |y| Index2D { x, y }))
            .collect()
    }
}

impl<T, S> Board<Index2D> for MatrixBoard<T, S> {
    type Structure = S;

    fn size(&self) -> usize {
        self.num_cols * self.num_rows
    }

    fn contains(&self, index: Index2D) -> bool {
        self.calculate_index(index).is_some()
    }

    fn structure(&self) -> &Self::Structure {
        &self.structure
    }
}

impl<T, S> ContiguousBoard<Index2D> for MatrixBoard<T, S> {
    fn bound(&self) -> Index2D {
        Index2D {
            x: self.num_cols,
            y: self.num_rows,
        }
    }

    fn wrapped(&self, index: Index2D) -> Index2D {
        Index2D {
            x: index.x % self.num_cols,
            y: index.y % self.num_rows,
        }
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
