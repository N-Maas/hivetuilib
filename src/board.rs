use std::{
    fmt::Debug,
    iter,
    iter::FromIterator,
    marker::PhantomData,
    ops::{Index, IndexMut},
    vec::IntoIter,
};

// ----- trait definitions -----

// TODO generic index type instead of usize?
pub trait Board<T>: BoardIndex + UnstableAccess<T> {
    type Structure;

    fn size(&self) -> usize;

    fn contains_field(&self, index: Self::Index) -> bool {
        self.get(index).is_some()
    }

    fn structure(&self) -> &Self::Structure;

    fn field_at<'a>(&'a self, index: Self::Index) -> Field<'a, T, Self> {
        self.get_field(index)
            .expect(&format!("invalid index: {:?}", index))
    }

    fn get_field<'a>(&'a self, index: Self::Index) -> Option<Field<'a, T, Self>> {
        if self.contains_field(index) {
            Some(Field::new(self, index))
        } else {
            None
        }
    }

    fn get(&self, index: Self::Index) -> Option<&T>;

    fn get_mut(&mut self, index: Self::Index) -> Option<&mut T>;

    fn iter_fields<'a>(&'a self) -> <&'a Self as BoardIntoFieldIter<T>>::IntoIter
    where
        T: 'a,
    {
        self.into_field_iter()
    }

    fn iter<'a>(&'a self) -> <&'a Self as BoardIntoIter<T>>::IntoIter
    where
        T: 'a,
    {
        self.into_iter()
    }

    fn iter_mut<'a>(&'a mut self) -> IntoIter<&'a mut T>
    where
        T: 'a,
    {
        self.mut_ref_vec().into_iter()
    }
}

// TODO
// impl Index

macro_rules! implBoardIntoIter {
    ($trait:ident for $name:ident, $call:ident, $out:ty, $access:ident) => {
        pub trait $trait<T> {
            type Output;
            type IntoIter: Iterator<Item = Self::Output>;

            fn $call(self) -> Self::IntoIter;
        }

        impl<'a, T, B: Board<T> + ?Sized> $trait<T> for &'a B
        where
            T: 'a,
        {
            type Output = $out;
            type IntoIter = $name<'a, T, B>;

            fn $call(self) -> Self::IntoIter {
                Self::IntoIter {
                    board: self,
                    iter: self.all_indices().into_iter(),
                    _f: PhantomData,
                }
            }
        }

        pub struct $name<'a, T, B: Board<T> + ?Sized> {
            board: &'a B,
            iter: IntoIter<B::Index>,
            _f: PhantomData<T>,
        }

        impl<'a, T, B: Board<T> + ?Sized> Iterator for $name<'a, T, B>
        where
            T: 'a,
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

implBoardIntoIter!(BoardIntoFieldIter for FieldIter, into_field_iter, Field<'a, T, B>, get_field);

implBoardIntoIter!(BoardIntoIter for BoardIter, into_iter, &'a T, get);

// ----- index type -----

#[derive(Debug, Clone, Copy)]
pub struct Index1D {
    pub val: usize,
}

impl From<usize> for Index1D {
    fn from(val: usize) -> Self {
        Self { val }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Index2D {
    pub x: usize,
    pub y: usize,
}

// ----- field implementation -----

#[derive(Debug)]
pub struct Field<'a, T, B: Board<T> + ?Sized> {
    board: &'a B,
    index: B::Index,
    _t: PhantomData<T>,
}

impl<'a, T, B: Board<T> + ?Sized> Field<'a, T, B> {
    pub fn new(board: &'a B, index: B::Index) -> Self {
        Self {
            board,
            index,
            _t: PhantomData,
        }
    }

    pub fn index(&self) -> B::Index {
        self.index
    }

    pub fn content(&self) -> &T {
        &self
            .board
            .get(self.index)
            .expect(&format!("Index of field is invalid: {:?}", self.index))
    }
}

impl<'a, T, B: Board<T> + ?Sized> Clone for Field<'a, T, B> {
    fn clone(&self) -> Self {
        Field { ..*self }
    }
}

impl<'a, T, B: Board<T> + ?Sized> Copy for Field<'a, T, B> {}

impl<'a, T, S, B: Board<T, Structure = S> + ?Sized> Field<'a, T, B>
where
    S: AdjacencyStructure<B>,
{
    pub fn is_adjacent_to(&self, index: B::Index) -> bool {
        self.board
            .structure()
            .is_adjacent(self.board, self.index, index)
    }

    pub fn is_adjacent(&self, other: &Self) -> bool {
        self.is_adjacent_to(other.index)
    }

    pub fn neighbor_count(&self) -> usize {
        self.board
            .structure()
            .neighbor_count(self.board, self.index)
    }

    pub fn get_neighbors(&self) -> impl Iterator<Item = Field<'a, T, B>> {
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
pub trait BoardIndex {
    type Index: Copy + Debug;

    fn all_indices(&self) -> Vec<Self::Index>;
}

// #[unstable]
pub trait UnstableAccess<T> {
    fn mut_ref_vec<'a>(&'a mut self) -> Vec<&'a mut T>;
}

pub trait AdjacencyStructure<B: BoardIndex + ?Sized> {
    fn is_adjacent(&self, board: &B, i: B::Index, j: B::Index) -> bool;

    fn neighbor_count(&self, board: &B, field: B::Index) -> usize;

    fn get_neighbors(&self, board: &B, field: B::Index) -> Vec<B::Index>;
}

// ----- board implementations -----

#[derive(Debug, Clone)]
pub struct LinearBoard<T> {
    content: Vec<T>,
}

impl<T: Clone> LinearBoard<T> {
    pub fn from_default(count: usize, def: T) -> Self {
        LinearBoard {
            content: vec![def; count],
        }
    }
}

impl<T: Default> LinearBoard<T> {
    pub fn with_default(count: usize) -> Self {
        LinearBoard {
            content: iter::repeat_with(|| Default::default())
                .take(count)
                .collect(),
        }
    }
}

impl<T> FromIterator<T> for LinearBoard<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        LinearBoard {
            content: iter.into_iter().collect(),
        }
    }
}

impl<T> BoardIndex for LinearBoard<T> {
    type Index = Index1D;

    fn all_indices(&self) -> Vec<Self::Index> {
        (0..self.content.len()).map(|val| Index1D { val }).collect()
    }
}

impl<T> UnstableAccess<T> for LinearBoard<T> {
    fn mut_ref_vec<'a>(&'a mut self) -> Vec<&'a mut T> {
        self.content.iter_mut().collect()
    }
}

impl<T> Board<T> for LinearBoard<T> {
    type Structure = ();

    fn size(&self) -> usize {
        self.content.len()
    }

    fn structure(&self) -> &Self::Structure {
        &()
    }

    fn get(&self, index: Index1D) -> Option<&T> {
        self.content.get(index.val)
    }

    fn get_mut(&mut self, index: Index1D) -> Option<&mut T> {
        self.content.get_mut(index.val)
    }
}

// ########## Macro?
impl<I, T> Index<I> for LinearBoard<T>
where
    Index1D: From<I>,
{
    type Output = T;

    fn index(&self, idx: I) -> &T {
        self.content.index(Index1D::from(idx).val)
    }
}

impl<I, T> IndexMut<I> for LinearBoard<T>
where
    Index1D: From<I>,
{
    fn index_mut(&mut self, idx: I) -> &mut T {
        self.content.index_mut(Index1D::from(idx).val)
    }
}
// ##########

// pub fn test_index<T: Debug, B: Board<T>>(b: B) {
//     print!("{:?}", b[0]);
// }
