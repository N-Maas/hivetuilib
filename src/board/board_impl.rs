use super::{directions::Offset, *};

use std::{
    iter,
    ops::{Index, IndexMut},
};

// TODO: use Box<[T]> instead
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

impl<T, I: Into<Index1D>, S> Index<I> for VecBoard<T, S> {
    type Output = T;

    fn index(&self, index: I) -> &T {
        self.content.index(index.into().val)
    }
}

impl<T, I: Into<Index1D>, S> IndexMut<I> for VecBoard<T, S> {
    fn index_mut(&mut self, index: I) -> &mut T {
        self.content.index_mut(index.into().val)
    }
}

impl<T, S> BoardIndexable for VecBoard<T, S> {
    type Index = Index1D;

    fn all_indices(&self) -> Vec<Index1D> {
        (0..self.content.len())
            .map(|val| Index1D::from(val))
            .collect()
    }
}

impl<T, S> Board for VecBoard<T, S> {
    type Content = T;
    type Structure = S;

    fn size(&self) -> usize {
        self.content.len()
    }

    fn contains(&self, index: Index1D) -> bool {
        index.val < self.size()
    }

    fn structure(&self) -> &S {
        &self.structure
    }

    fn get(&self, index: Index1D) -> Option<&T> {
        self.content.get(index.val)
    }

    fn get_mut(&mut self, index: Index1D) -> Option<&mut T> {
        self.content.get_mut(index.val)
    }
}

impl<T, S> ContiguousBoard for VecBoard<T, S> {
    type Offset = Offset;

    fn bound(&self) -> Index1D {
        Index1D::from(self.content.len())
    }

    fn wrapped(&self, Offset(index): Offset) -> Index1D {
        let rem = index.rem_euclid(self.content.len() as isize);
        assert!(rem >= 0);
        Index1D::from(rem as usize)
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

impl<T, I: Into<Index2D>, S> Index<I> for MatrixBoard<T, S> {
    type Output = T;

    fn index(&self, index: I) -> &T {
        self.get(index.into()).expect("Index out of bounds.")
    }
}

impl<T, I: Into<Index2D>, S> IndexMut<I> for MatrixBoard<T, S> {
    fn index_mut(&mut self, index: I) -> &mut T {
        self.get_mut(index.into()).expect("Index out of bounds.")
    }
}

impl<T, S> BoardIndexable for MatrixBoard<T, S> {
    type Index = Index2D;
    fn all_indices(&self) -> Vec<Index2D> {
        (0..self.num_cols)
            .flat_map(|x| (0..self.num_rows).map(move |y| Index2D { x, y }))
            .collect()
    }
}

impl<T, S> Board for MatrixBoard<T, S> {
    type Content = T;
    type Structure = S;

    fn size(&self) -> usize {
        self.num_cols * self.num_rows
    }

    fn contains(&self, index: Index2D) -> bool {
        self.calculate_index(index).is_some()
    }

    fn structure(&self) -> &S {
        &self.structure
    }

    fn get(&self, index: Index2D) -> Option<&T> {
        let idx = self.calculate_index(index)?;
        self.content.get(idx)
    }

    fn get_mut(&mut self, index: Index2D) -> Option<&mut T> {
        let idx = self.calculate_index(index)?;
        self.content.get_mut(idx)
    }
}

impl<T, S> ContiguousBoard for MatrixBoard<T, S> {
    type Offset = (Offset, Offset);

    fn bound(&self) -> Index2D {
        Index2D {
            x: self.num_cols,
            y: self.num_rows,
        }
    }

    fn wrapped(&self, (Offset(x), Offset(y)): (Offset, Offset)) -> Index2D {
        let rem_x = x.rem_euclid(self.num_cols as isize);
        let rem_y = y.rem_euclid(self.num_rows as isize);
        assert!(rem_x >= 0 && rem_y >= 0);
        Index2D {
            x: rem_x as usize,
            y: rem_y as usize,
        }
    }
}

mod test {
    use super::*;

    #[test]
    fn vec_board_test() {
        let board = VecBoard::<usize, ()>::with_default(1, ());
        assert_eq!(board.size(), 1);
        assert_eq!(board[0], 0);
    }

    #[test]
    fn matrix_board_test() {
        let board = MatrixBoard::<usize, ()>::with_default(2, 2, ());
        assert_eq!(board.size(), 4);
        assert_eq!(board[(1, 1)], 0);
    }
}
