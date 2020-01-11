use std::{
    iter,
    iter::FromIterator,
    fmt::Debug,
    marker::PhantomData,
    ops::{Index, IndexMut},
    vec::IntoIter,
};

// ----- trait definitions -----

// TODO generic index type instead of usize?
pub trait Board<T>:
    IndexMut<usize, Output = T>
    + MutAccess<Content = T>
    // bad hack (?)
    + Sized
{
    type Structure;

    fn size(&self) -> usize;

    fn structure(&self) -> &Self::Structure;

    fn field_at<'a>(&'a self, index: usize) -> Field<'a, T, Self> {
        if index < self.size() {
            Field::new(self, index)
        } else {
            panic!(format!("index out of bounds: the size is {} but the index is {}", self.size(), index))
        }
    }

    fn get_field<'a>(&'a self, index: usize) -> Option<Field<'a, T, Self>> {
        if index < self.size() {
            Some(Field::new(self, index))
        } else {
            None
        }
    }

    fn get(&self, index: usize) -> Option<&T> {
        if index < self.size() {
            Some(&self[index])
        } else {
            None
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < self.size() {
            Some(&mut self[index])
        } else {
            None
        }
    }

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

    fn iter_mut<'a>(&'a mut self) -> IntoIter<&'a mut Self::Content>
    where
        T: 'a,
    {
        self.mut_ref_vec().into_iter()
    }
}

macro_rules! implBoardIntoIter {
    ($trait:ident for $name:ident, $call:ident, $out:ty, $access:ident) => {
        pub trait $trait<T> {
            type Output;
            type IntoIter: Iterator<Item = Self::Output>;

            fn $call(self) -> Self::IntoIter;
        }

        impl<'a, T, B: Board<T>> $trait<T> for &'a B
        where
            T: 'a,
        {
            type Output = $out;
            type IntoIter = $name<'a, T, B>;

            fn $call(self) -> Self::IntoIter {
                Self::IntoIter {
                    board: self,
                    current: 0,
                    _f: PhantomData,
                }
            }
        }

        pub struct $name<'a, T, B: Board<T>> {
            board: &'a B,
            current: usize,
            _f: PhantomData<T>,
        }

        impl<'a, T, B: Board<T>> Iterator for $name<'a, T, B>
        where
            T: 'a,
        {
            type Item = $out;

            fn next(&mut self) -> Option<Self::Item> {
                let idx = self.current;
                self.current += 1;

                if idx < self.board.size() {
                    Some(self.board.$access(idx))
                } else {
                    None
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let rem = self.board.size() - self.current;
                (rem, Some(rem))
            }
        }
    };
}

implBoardIntoIter!(BoardIntoFieldIter for FieldIter, into_field_iter, Field<'a, T, B>, field_at);

implBoardIntoIter!(BoardIntoIter for BoardIter, into_iter, &'a T, index);

#[derive(Debug)]
pub struct Field<'a, T, B: Index<usize, Output = T>> {
    board: &'a B,
    index: usize,
    _t: PhantomData<T>,
}

impl<'a, T, B: Index<usize, Output = T>> Field<'a, T, B> {
    pub fn new(board: &'a B, index: usize) -> Self {
        Self {
            board,
            index,
            _t: PhantomData,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn content(&self) -> &T {
        &self.board[self.index]
    }
}

impl<'a, T, B: Index<usize, Output = T>> Clone for Field<'a, T, B> {
    fn clone(&self) -> Self {
        Field { ..*self }
    }
}

impl<'a, T, B: Index<usize, Output = T>> Copy for Field<'a, T, B> {}

// TOOD rather bad hack to enable mutable iteration
pub trait MutAccess {
    type Content;

    // #[unstable]
    fn mut_ref_vec<'a>(&'a mut self) -> Vec<&'a mut Self::Content>;
}

// ----- implementations -----

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

impl<T> Index<usize> for LinearBoard<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.content[index]
    }
}

impl<T> IndexMut<usize> for LinearBoard<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.content[index]
    }
}

impl<T> MutAccess for LinearBoard<T> {
    type Content = T;

    fn mut_ref_vec<'a>(&'a mut self) -> Vec<&'a mut Self::Content> {
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
}
