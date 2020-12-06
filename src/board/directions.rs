use super::*;

// TODO: move type parameter to associated type?
pub trait DirectionOffset<O>: Copy + Eq {
    fn get_offset(&self) -> O;
}

pub trait DirectionReversable: Copy + Eq {
    fn reversed(&self) -> Self;
}

pub trait DirectionEnumerable: Copy + Eq + Sized {
    type Iter: Iterator<Item = Self>;

    fn enumerate_all() -> Self::Iter;
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
    fn get_offset(&self) -> Offset {
        match self {
            BinaryDirection::Forward => Offset(1),
            BinaryDirection::Backward => Offset(-1),
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
            fn get_offset(&self) -> (Offset, Offset) {
                match self {
                    $($name::$dir => (Offset($x), Offset($y)),)+
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
