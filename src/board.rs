use std::{
    fmt::Debug, iter, iter::FromIterator, marker::PhantomData, ops::Index, ops::IndexMut,
    vec::IntoIter,
};

// ----- trait definitions -----

pub trait BoardIdxType: Copy + Eq + Debug {}

pub trait Board<I>: BoardIndex<I>
where
    I: BoardIdxType,
{
    type Structure;

    fn size(&self) -> usize;

    fn contains(&self, index: I) -> bool;

    fn structure(&self) -> &Self::Structure;

    fn field_at<'a>(&'a self, index: I) -> Field<'a, I, Self> {
        self.get_field(index)
            .expect(&format!("invalid index: {:?}", index))
    }

    fn get_field<'a>(&'a self, index: I) -> Option<Field<'a, I, Self>> {
        if self.contains(index) {
            Some(Field::new(self, index))
        } else {
            None
        }
    }

    fn get(&self, idx: I) -> Option<&Self::Output> {
        if self.contains(idx) {
            Some(self.index(idx))
        } else {
            None
        }
    }

    fn get_mut(&mut self, idx: I) -> Option<&mut Self::Output> {
        if self.contains(idx) {
            Some(self.index_mut(idx))
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

// ----- field implementation -----

#[derive(Debug)]
pub struct Field<'a, I: BoardIdxType, B: Board<I> + ?Sized> {
    board: &'a B,
    index: I,
}

impl<'a, I: BoardIdxType, B: Board<I> + ?Sized> Field<'a, I, B> {
    pub fn new(board: &'a B, index: I) -> Self {
        Self { board, index }
    }

    pub fn index(&self) -> I {
        self.index
    }

    pub fn content(&self) -> &B::Output {
        &self
            .board
            .get(self.index)
            .expect(&format!("Index of field is invalid: {:?}", self.index))
    }
}

impl<'a, I: BoardIdxType, B: Board<I> + ?Sized> Clone for Field<'a, I, B> {
    fn clone(&self) -> Self {
        Field { ..*self }
    }
}

impl<'a, I: BoardIdxType, B: Board<I> + ?Sized> Copy for Field<'a, I, B> {}

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
    S: NeighborhoodyStructure<I, B>,
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
            .map(move |i| Self::new(board, i))
    }
}

// TOOD rather bad hack to enable iteration
// #[unstable]
pub trait BoardIndex<I: BoardIdxType>: IndexMut<I> {
    fn all_indices(&self) -> Vec<I>;

    // fn enumerate_mut(&mut self) -> Vec<(I, &mut Self::Output)>;
}

pub trait AdjacencyStructure<I: BoardIdxType, B: Board<I> + ?Sized> {
    fn is_adjacent(&self, board: &B, i: I, j: I) -> bool;
}

pub trait NeighborhoodyStructure<I: BoardIdxType, B: Board<I> + ?Sized> {
    fn neighbor_count(&self, board: &B, field: I) -> usize;

    // TODO more efficient than vec?
    fn get_neighbors(&self, board: &B, field: I) -> Vec<I>;
}

// ----- board implementations -----

#[derive(Debug, Clone)]
pub struct VecBoard<T, S=()> {
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
        (0..self.content.len()).map(|val| Index1D::from(val)).collect()
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

// ----- structure implementations -----

#[derive(Debug, Clone)]
pub struct AdjacencySet<I: BoardIdxType + Hash> {
    edges: HashSet<(I, I)>,
}

impl<I: BoardIdxType + Hash> AdjacencySet<I> {
    fn new() -> Self {
        Self {
            edges: HashSet::new(),
        }
    }

    fn add_directed(&mut self, i: I, j: I) {
        self.edges.insert((i, j));
    }

    fn add_undirected(&mut self, i: I, j: I) {
        self.edges.insert((i, j));
        self.edges.insert((j, i));
    }

    fn iter_edges(&self) -> impl Iterator<Item=&(I, I)> {
        self.edges.iter()
    }
}

impl<I: BoardIdxType + Hash, B: Board<I> + ?Sized> AdjacencyStructure<I, B> for AdjacencySet<I> {
    fn is_adjacent(&self, _board: &B, i: I, j: I) -> bool {
        self.edges.contains(&(i, j))
    }
}

