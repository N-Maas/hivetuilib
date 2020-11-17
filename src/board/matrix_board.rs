use super::{
    directions::DirectionOffset, directions::Offset, directions::OffsetableIndex,
    search::BoardToMap, search::HashIndexMap, *,
};

use std::{
    iter,
    ops::{Add, Index, IndexMut},
};

// A two-dimensional immutable board. The fields are saved in a vec internally, calculating the index as necessary.
#[derive(Debug, Clone)]
pub struct MatrixBoard<T, S = ()> {
    content: Box<[T]>,
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
            content: iter::repeat(val).take(num_cols * num_rows).collect(),
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

impl<T, S, E> BoardToMap<E> for MatrixBoard<T, S> {
    type Map = HashIndexMap<Index2D, E>;

    fn get_index_map(&self) -> Self::Map {
        Self::Map::new()
    }
}

// ----- the index belonging to a MatrixBoard -----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Index2D {
    pub x: usize,
    pub y: usize,
}

impl BoardIdxType for Index2D {}

impl From<(usize, usize)> for Index2D {
    fn from((x, y): (usize, usize)) -> Self {
        Self { x, y }
    }
}

impl<B: Board<Index = Index2D>> From<Field<'_, B>> for Index2D {
    fn from(f: Field<'_, B>) -> Self {
        f.index()
    }
}

impl PartialOrd for Index2D {
    fn partial_cmp(&self, other: &Index2D) -> Option<Ordering> {
        if self.x == other.x && self.y == other.y {
            Some(Ordering::Equal)
        } else if self.x <= other.y && self.y <= other.y {
            Some(Ordering::Less)
        } else if self.x >= other.y && self.y >= other.y {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}

impl OffsetableIndex for Index2D {
    type Offset = (Offset, Offset);

    fn apply_offset(&self, (Offset(dx), Offset(dy)): (Offset, Offset)) -> (Offset, Offset) {
        (Offset(self.x as isize + dx), Offset(self.y as isize + dy))
    }

    fn from_offset((Offset(x), Offset(y)): (Offset, Offset)) -> Option<Self> {
        if x >= 0 && y >= 0 {
            Some(Self {
                x: x as usize,
                y: y as usize,
            })
        } else {
            None
        }
    }
}

impl<D> Add<D> for Index2D
where
    D: DirectionOffset<<Self as OffsetableIndex>::Offset>,
{
    type Output = <Self as OffsetableIndex>::Offset;

    fn add(self, rhs: D) -> Self::Output {
        self.apply_offset(rhs.get_offset())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_test() {
        let board = MatrixBoard::<usize, ()>::with_default(2, 2, ());
        assert_eq!(board.size(), 4);
        assert_eq!(board[(1, 1)], 0);
    }
}
