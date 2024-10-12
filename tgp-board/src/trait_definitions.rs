use std::fmt::Debug;

use crate::field::Field;

pub trait BoardIdxType: Copy + Eq + Debug {}

// TODO: do not use impl Trait returns
// TODO: replace occurences with Into<index> - general solution for mutable access coming from field?
pub trait Board: BoardIndexable {
    type Content;
    // TODO: add trait bound?
    type Structure;

    fn size(&self) -> usize;

    fn contains(&self, index: Self::Index) -> bool {
        self.get(index).is_some()
    }

    fn structure(&self) -> &Self::Structure;

    // TODO better get_field_unchecked or similar?
    fn get_field_unchecked(&self, index: Self::Index) -> Field<Self>
    where
        Self: Sized,
    {
        self.get_field(index)
            .unwrap_or_else(|| panic!("Invalid index: {:?}", index))
    }

    fn get_field(&self, index: Self::Index) -> Option<Field<Self>>
    where
        Self: Sized,
    {
        Field::new(self, index)
    }

    fn get(&self, index: Self::Index) -> Option<&Self::Content>;

    fn iter_fields<'a>(&'a self) -> impl Iterator<Item = Field<'a, Self>>
    where
        Self: Sized,
        Self::Content: 'a,
    {
        self.into_field_iter()
    }

    // TODO: required?
    // TODO: iter_mut impossible to define in trait currently
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Self::Content>
    where
        Self: Sized,
        Self::Content: 'a,
    {
        self.into_iter()
    }
}

pub trait BoardMut: Board {
    // TODO: convenience methods? FieldMut API?

    fn get_mut(&mut self, index: Self::Index) -> Option<&mut Self::Content>;
}

// TODO impl Index possible?

macro_rules! implBoardIntoIter {
    ($trait:ident for $name:ident, $call:ident, $out:ty, $access:ident) => {
        pub trait $trait<I, T> {
            type Output;

            fn $call(self) -> impl Iterator<Item = Self::Output>;
        }

        impl<'a, B: Board> $trait<B::Index, B::Content> for &'a B
        where
            B::Content: 'a,
        {
            type Output = $out;

            fn $call(self) -> impl Iterator<Item = Self::Output> {
                self.all_indices().map(|idx| self.$access(idx).unwrap())
            }
        }
    };
}

implBoardIntoIter!(BoardIntoFieldIter for FieldIter, into_field_iter, Field<'a, B>, get_field);

implBoardIntoIter!(BoardIntoIter for BoardIter, into_iter, &'a B::Content, get);

// ----- extended board types -----

// TODO do these methods belong together?
pub trait ContiguousBoard: Board
where
    Self::Index: PartialOrd,
{
    type Offset;

    // should return a smallest common bound, i.e. i < b.bound() for a board b and every i with b.contains(i)
    // TODO: is this required at all? Add minimum?
    fn bound(&self) -> Self::Index;

    fn wrapped(&self, index: Self::Offset) -> Self::Index;

    // TODO: get_wrapped etc. helper functions?
}

// TOOD rather bad hack to enable iteration - enforce lifetime binding to self?
// #[unstable]
pub trait BoardIndexable {
    type Index: BoardIdxType;

    fn all_indices(&self) -> impl Iterator<Item = Self::Index>;

    // fn enumerate_mut(&mut self) -> Vec<(I, &mut Self::Content)>;
}

// ----- index map -----

/// Note that the iteration order should always be deterministic!
pub trait IndexMap {
    type IndexType: BoardIdxType;
    type Item;
    type Iter: ExactSizeIterator<Item = Self::IndexType>;

    // fn from_board<B: Board<Self::IndexType>>(board: &'a B) -> Self;
    fn size(&self) -> usize;

    fn contains(&self, i: Self::IndexType) -> bool {
        self.get(i).is_some()
    }

    fn get(&self, i: Self::IndexType) -> Option<&Self::Item>;

    fn get_mut(&mut self, i: Self::IndexType) -> Option<&mut Self::Item>;

    /// Returns the old value if the key was already present.
    fn insert(&mut self, i: Self::IndexType, el: Self::Item) -> Option<Self::Item>;

    fn retain(&mut self, filter: impl FnMut(Self::IndexType, &mut Self::Item) -> bool);

    fn iter_indices(&self) -> Self::Iter;

    fn clear(&mut self);

    // TODO: subset and further helper methods?
}

pub trait BoardToMap<T>: Board {
    type Map: IndexMap<Item = T, IndexType = Self::Index>;

    fn get_index_map(&self) -> Self::Map;
}
