use std::{iter::Copied, slice::Iter};

use crate::trait_definitions::BoardIdxType;

// TODO: move type parameter to associated type?
pub trait DirectionOffset<O>: Copy + Eq {
    fn offset(&self) -> O;

    fn from_offset(offset: O) -> Option<Self>;
}

pub trait DirectionReversable: Copy + Eq {
    fn reversed(&self) -> Self;
}

pub trait DirectionEnumerable: Copy + Eq + Sized {
    type Iter: ExactSizeIterator<Item = Self>;

    fn enumerate_all() -> Self::Iter;

    fn next_direction(&self) -> Self {
        let (idx, _) = Self::enumerate_all()
            .enumerate()
            .find(|(_, d)| d == self)
            .expect("Enumeration of directions not complete!");
        let mut iter = Self::enumerate_all();
        // can not fail because of modulo and Iter being an ExactSizeIterator
        iter.nth((idx + 1) % iter.len()).unwrap()
    }

    fn prev_direction(&self) -> Self {
        let (idx, _) = Self::enumerate_all()
            .enumerate()
            .find(|(_, d)| d == self)
            .expect("Enumeration of directions not complete!");
        let mut iter = Self::enumerate_all();
        // add iter.len() to avoid undeflow
        // can not fail because of modulo and Iter being an ExactSizeIterator
        iter.nth((iter.len() + idx - 1) % iter.len()).unwrap()
    }
}

// TODO: Enumerable without reverse?
// TODO: trait for direction -> index mapping (efficient structure)
// TODO: derive macro for Enumerable/index mapping

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Offset(pub isize);

pub trait OffsetableIndex: BoardIdxType {
    type Offset;

    fn apply_offset(&self, offset: Self::Offset) -> Self::Offset;

    fn from_offset(index: Self::Offset) -> Option<Self>;
}

// ----- direction implementations -----

// TODO remove "Direction" from name?
// a direction with two possibilities: Forward and Backward
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BinaryDirection {
    Forward,
    Backward,
}

impl DirectionOffset<Offset> for BinaryDirection {
    fn offset(&self) -> Offset {
        match self {
            BinaryDirection::Forward => Offset(1),
            BinaryDirection::Backward => Offset(-1),
        }
    }

    fn from_offset(offset: Offset) -> Option<Self> {
        match offset {
            Offset(1) => Some(BinaryDirection::Forward),
            Offset(-1) => Some(BinaryDirection::Backward),
            _ => None,
        }
    }
}

impl DirectionReversable for BinaryDirection {
    fn reversed(&self) -> Self {
        match self {
            BinaryDirection::Forward => BinaryDirection::Backward,
            BinaryDirection::Backward => BinaryDirection::Forward,
        }
    }
}

impl DirectionEnumerable for BinaryDirection {
    type Iter = Copied<Iter<'static, BinaryDirection>>;

    fn enumerate_all() -> Self::Iter {
        static DIRS: [BinaryDirection; 2] = [BinaryDirection::Forward, BinaryDirection::Backward];
        DIRS.iter().copied()
    }
}

// implement directions used for a two dimensions
macro_rules! impl2DDirection {
    (   $(#[$meta:meta])*
        $name:ident[$num:literal] {
            $($dir:ident($x:literal, $y:literal) - $rev:ident),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name {
            $($dir,)+
        }

        impl DirectionOffset<(Offset, Offset)> for $name {
            fn offset(&self) -> (Offset, Offset) {
                match self {
                    $($name::$dir => (Offset($x), Offset($y)),)+
                }
            }

            fn from_offset(offset: (Offset, Offset)) -> Option<Self> {
                match offset {
                    $((Offset($x), Offset($y)) => Some($name::$dir),)+
                    _ => None,
                }
            }
        }

        impl DirectionReversable for $name {
            fn reversed(&self) -> Self {
                match self {
                    $($name::$dir => $name::$rev,)+
                }
            }
        }

        impl DirectionEnumerable for $name {
            type Iter = Copied<Iter<'static, $name>>;

            fn enumerate_all() -> Self::Iter {
                static DIRS: [$name; $num] = [
                        $($name::$dir,)+
                    ];
                DIRS.iter().copied()
            }
        }
    };
}

impl2DDirection!(
    /// Represents the directions in a grid, without diagonals.
    GridDirection[4] {
        Up(0, 1) - Down,
        Right(1, 0) - Left,
        Down(0, -1) - Up,
        Left(-1, 0) - Right,
    }
);

impl2DDirection!(
    /// Represents the directions in a grid, including diagonals.
    GridDiagDirection[8] {
        Up(0, 1) - Down,
        UpRight(1, 1) - DownLeft,
        Right(1, 0) - Left,
        DownRight(1, -1) - UpLeft,
        Down(0, -1) - Up,
        DownLeft(-1, -1) - UpRight,
        Left(-1, 0) - Right,
        UpLeft(-1, 1) - DownRight,
    }
);

impl2DDirection!(
    /// Represents directions for a hexagonal board.
    /// <pre>
    ///  ___
    /// /0/2\___
    /// \___/1/2\___
    /// /0/1\___/2/2\
    /// \___/1/1\___/
    /// /0/0\___/2/1\
    /// \___/1/0\___/
    ///     \___/2/0\
    ///         \___/
    /// </pre>
    HexaDirection[6] {
        Up(0, 1) - Down,
        UpRight(1, 1) - DownLeft,
        DownRight(1, 0) - UpLeft,
        Down(0, -1) - Up,
        DownLeft(-1, -1) - UpRight,
        UpLeft(-1, 0) - DownRight,
    }
);
