use std::ops::Add;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Offset(pub isize);

pub trait OffsetableIndex: BoardIdxType {
    type Offset;

    fn apply_offset(&self, offset: Self::Offset) -> Self::Offset;

    fn from_offset(index: Self::Offset) -> Option<Self>;
}

impl OffsetableIndex for Index1D {
    type Offset = Offset;

    fn apply_offset(&self, Offset(delta): Offset) -> Offset {
        Offset(self.val as isize + delta)
    }

    fn from_offset(Offset(index): Offset) -> Option<Self> {
        if index >= 0 {
            Some(Self::from(index as usize))
        } else {
            None
        }
    }
}

impl<D> Add<D> for Index1D
where
    D: DirectionOffset<<Self as OffsetableIndex>::Offset>,
{
    type Output = <Self as OffsetableIndex>::Offset;

    fn add(self, rhs: D) -> Self::Output {
        self.apply_offset(rhs.get_offset())
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
    ($name:ident[$num:literal] {
        $($dir:ident($x:literal, $y:literal) - $rev:ident),+ $(,)?
     }) => {
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

// TODO remove "Direction" from name?
// represents the directions in a grid, without diagonals
impl2DDirection!(
    GridDirection[4] {
        Up(0, 1) - Down,
        Right(1, 0) - Left,
        Down(0, -1) - Up,
        Left(-1, 0) - Right,
    }
);

// represents the directions in a grid, including diagonals
impl2DDirection!(
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

// represents the directions in a grid, including diagonals
impl2DDirection!(
    HexaDirection[6] {
        Up(0, 1) - Down,
        UpRight(1, 1) - DownLeft,
        DownRight(1, 0) - UpLeft,
        Down(0, -1) - Up,
        DownLeft(-1, -1) - UpRight,
        UpLeft(-1, 0) - DownRight,
    }
);

mod test {
    use super::*;
}
