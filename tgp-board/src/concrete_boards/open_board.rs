use crate::{
    search::{HashIndexMap, SearchingSet, SearchingTree},
    structures::directions::{DirectionOffset, Offset, OffsetableIndex},
    Board, BoardIdxType, BoardIndexable, BoardMut, BoardToMap, Field,
};

use std::{
    cmp::Ordering,
    collections::VecDeque,
    iter, mem,
    ops::{Add, Index, IndexMut},
};

// TODO: remove field
// TODO: use i32?!
/// A two-dimensional board which can grow as needed. Supports inserting and removing single fields.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OpenBoard<T, S = ()> {
    columns: VecDeque<VecDeque<Option<T>>>,
    num_rows: usize,
    size: usize,
    // added to all indices
    offset: (isize, isize),
    structure: S,
}

impl<T, S> OpenBoard<T, S> {
    fn calculate_index(&self, index: OpenIndex) -> Option<(usize, usize)> {
        let x = index.x + self.offset.0;
        let y = index.y + self.offset.1;
        if x >= 0 && y >= 0 {
            Some((x as usize, y as usize))
        } else {
            None
        }
    }

    pub fn new(structure: S) -> Self {
        Self {
            columns: VecDeque::new(),
            num_rows: 0,
            size: 0,
            offset: (0, 0),
            structure,
        }
    }

    pub fn with_dimensions(num_rows: usize, num_cols: usize, structure: S) -> Self {
        Self {
            columns: iter::repeat_with(VecDeque::new).take(num_cols).collect(),
            num_rows,
            size: 0,
            offset: (0, 0),
            structure,
        }
    }

    pub fn num_cols(&self) -> usize {
        self.columns.len()
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn lower_x(&self) -> isize {
        -self.offset.0
    }

    pub fn lower_y(&self) -> isize {
        -self.offset.1
    }

    pub fn upper_x(&self) -> isize {
        self.lower_x() + self.num_cols() as isize
    }

    pub fn upper_y(&self) -> isize {
        self.lower_y() + self.num_rows as isize
    }

    /// returns true if the field was not contained previously
    pub fn extend_and_insert(&mut self, index: OpenIndex, val: T) -> bool {
        self.extend_to_index(index);
        self.insert(index, val)
    }

    /// returns true if the index is valid and the field was not contained previously
    pub fn insert(&mut self, index: OpenIndex, val: T) -> bool {
        self.calculate_index(index).map_or(false, |(x, y)| {
            if y < self.num_rows {
                if let Some(column) = self.columns.get_mut(x) {
                    while y >= column.len() {
                        column.push_back(None);
                    }
                    self.size += 1;
                    return mem::replace(&mut column[y], Some(val)).is_none();
                }
            }
            false
        })
    }

    /// returns true if the field was successfully deleted
    pub fn delete(&mut self, index: OpenIndex) -> bool {
        self.calculate_index(index).map_or(false, |(x, y)| {
            if let Some(column) = self.columns.get_mut(x) {
                if y < column.len() {
                    self.size -= 1;
                    return column[y].take().is_some();
                }
            }
            false
        })
    }

    pub fn search(&self) -> SearchingSet<HashIndexMap<OpenIndex>, Self> {
        SearchingSet::new(self)
    }

    pub fn search_tree(&self) -> SearchingTree<HashIndexMap<OpenIndex>, Self> {
        SearchingTree::new(self)
    }

    fn extend_to_index(&mut self, index: OpenIndex) {
        while index.x < -self.offset.0 {
            self.insert_column(false);
        }
        while index.x >= self.columns.len() as isize - self.offset.0 {
            self.insert_column(true);
        }
        while index.y < -self.offset.1 {
            self.insert_row(false);
        }
        while index.y >= self.num_rows as isize - self.offset.1 {
            self.insert_row(true);
        }
    }

    fn insert_row(&mut self, top: bool) {
        self.num_rows += 1;
        if !top {
            self.offset.1 += 1;
            for column in &mut self.columns {
                column.push_front(None);
            }
        }
    }

    fn insert_column(&mut self, right: bool) {
        if !right {
            self.offset.0 += 1;
            self.columns.push_front(VecDeque::new());
        } else {
            self.columns.push_back(VecDeque::new());
        }
    }

    // TODO: shrink operation
    // TODO: insert row/column?
}

// TODO: more constructors?
// TODO add_row or comparable?

impl<T, I: Into<OpenIndex>, S> Index<I> for OpenBoard<T, S> {
    type Output = T;

    fn index(&self, index: I) -> &T {
        self.get(index.into()).expect("Invalid index.")
    }
}

impl<T, I: Into<OpenIndex>, S> IndexMut<I> for OpenBoard<T, S> {
    fn index_mut(&mut self, index: I) -> &mut T {
        self.get_mut(index.into()).expect("Invalid index.")
    }
}

impl<T, S> BoardIndexable for OpenBoard<T, S> {
    type Index = OpenIndex;
    // TODO: this is wrong and not performant
    fn all_indices(&self) -> Vec<OpenIndex> {
        let (dx, dy) = self.offset;
        self.columns
            .iter()
            .enumerate()
            .flat_map(|(x, col)| {
                col.iter().enumerate().filter_map(move |(y, field)| {
                    field
                        .as_ref()
                        .map(|_| OpenIndex::from((x as isize - dx, y as isize - dy)))
                })
            })
            .collect()
    }
}

impl<T, S> Board for OpenBoard<T, S> {
    type Content = T;
    type Structure = S;

    fn size(&self) -> usize {
        self.size
    }

    fn structure(&self) -> &S {
        &self.structure
    }

    fn get(&self, index: OpenIndex) -> Option<&T> {
        let (x, y) = self.calculate_index(index)?;
        let column = self.columns.get(x)?;
        column.get(y)?.into()
    }
}

impl<T, S> BoardMut for OpenBoard<T, S> {
    fn get_mut(&mut self, index: OpenIndex) -> Option<&mut T> {
        let (x, y) = self.calculate_index(index)?;
        let column = self.columns.get_mut(x)?;
        column.get_mut(y)?.into()
    }
}

// TODO: Contiguous board?

impl<T, S, E> BoardToMap<E> for OpenBoard<T, S> {
    type Map = HashIndexMap<OpenIndex, E>;

    fn get_index_map(&self) -> Self::Map {
        Self::Map::new()
    }
}

// ----- the index belonging to an OpenBoard -----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenIndex {
    pub x: isize,
    pub y: isize,
}

impl BoardIdxType for OpenIndex {}

impl From<(isize, isize)> for OpenIndex {
    fn from((x, y): (isize, isize)) -> Self {
        Self { x, y }
    }
}

impl<B: Board<Index = OpenIndex>> From<Field<'_, B>> for OpenIndex {
    fn from(f: Field<'_, B>) -> Self {
        f.index()
    }
}

impl PartialOrd for OpenIndex {
    fn partial_cmp(&self, other: &OpenIndex) -> Option<Ordering> {
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

impl OffsetableIndex for OpenIndex {
    type Offset = (Offset, Offset);

    fn apply_offset(&self, (Offset(dx), Offset(dy)): (Offset, Offset)) -> (Offset, Offset) {
        (Offset(self.x + dx), Offset(self.y + dy))
    }

    fn from_offset((Offset(x), Offset(y)): (Offset, Offset)) -> Option<Self> {
        Some(Self { x, y })
    }
}

impl<D> Add<D> for OpenIndex
where
    D: DirectionOffset<<Self as OffsetableIndex>::Offset>,
{
    type Output = Self;

    fn add(self, rhs: D) -> Self {
        Self::from_offset(self.apply_offset(rhs.get_offset())).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_test() {
        let mut board = OpenBoard::<bool, ()>::new(());
        board.extend_and_insert((0, 0).into(), false);
        assert_eq!(board.size(), 1);
        assert_eq!(board.num_cols(), 1);
        assert_eq!(board.num_rows(), 1);
        board.extend_and_insert((1, 1).into(), true);
        board.extend_and_insert((-1, -1).into(), true);
        assert_eq!(board.size(), 3);
        assert_eq!(board.num_cols(), 3);
        assert_eq!(board.num_rows(), 3);
        assert_eq!(board[(0, 0)], false);
        assert!(board.contains((1, 1).into()));
        assert!(!board.contains((0, 1).into()));
        board.extend_and_insert((-1, 1).into(), false);
        assert_eq!(board[(-1, 1)], false);
        assert!(board.delete((-1, -1).into()));
        assert!(!board.delete((1, 0).into()));
        assert!(board.delete((-1, 1).into()));
    }

    #[test]
    fn iter_test() {
        let mut board = OpenBoard::<bool, ()>::with_dimensions(3, 3, ());
        board.insert((1, 0).into(), true);
        board.insert((0, 2).into(), true);
        let indices = board.all_indices();
        assert_eq!(indices.len(), 2);
        assert!(indices.contains(&(1, 0).into()));
        assert!(indices.contains(&(0, 2).into()));
    }
}
