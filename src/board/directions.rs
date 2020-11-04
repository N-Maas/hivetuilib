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

#[derive(Debug, Clone, Copy, Eq)]
pub enum Offset {
    Neg(usize),
    Pos(usize),
}

impl PartialEq for Offset {
    fn eq(&self, other: &Offset) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl PartialOrd for Offset {
    fn partial_cmp(&self, other: &Offset) -> Option<Ordering> {
        match (self, other) {
            (Offset::Neg(a), Offset::Neg(b)) => Some(b.cmp(a)),
            (Offset::Pos(a), Offset::Pos(b)) => Some(a.cmp(b)),
            (Offset::Pos(0), Offset::Neg(0)) => Some(Ordering::Equal),
            (Offset::Neg(0), Offset::Pos(0)) => Some(Ordering::Equal),
            (Offset::Pos(_), Offset::Neg(_)) => Some(Ordering::Greater),
            (Offset::Neg(_), Offset::Pos(_)) => Some(Ordering::Less),
        }
    }
}

impl Ord for Offset {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn apply_offset(n: usize, offset: Offset) -> Option<usize> {
    match offset {
        Offset::Pos(offset) => n.checked_add(offset),
        Offset::Neg(offset) => n.checked_sub(offset),
    }
}

pub trait OffsetableIndex: BoardIdxType {
    type Offset;

    fn apply_offset(&self, offset: Self::Offset) -> Option<Self>;
}

impl OffsetableIndex for Index1D {
    type Offset = Offset;

    fn apply_offset(&self, offset: Offset) -> Option<Self> {
        apply_offset(self.val, offset).map(|val| Self::from(val))
    }
}

impl OffsetableIndex for Index2D {
    type Offset = (Offset, Offset);

    fn apply_offset(&self, offset: (Offset, Offset)) -> Option<Self> {
        let x = apply_offset(self.x, offset.0)?;
        let y = apply_offset(self.y, offset.1)?;
        Some(Self { x, y })
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
            BinaryDirection::Forward => Offset::Pos(1),
            BinaryDirection::Backward => Offset::Neg(1),
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
        $($dir:ident($x_type:ident($x:literal), $y_type:ident($y:literal)) - $rev:ident),+ $(,)?
     }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name {
            $($dir,)+
        }

        impl DirectionOffset<(Offset, Offset)> for $name {
            fn get_offset(&self) -> (Offset, Offset) {
                match self {
                    $($name::$dir => (Offset::$x_type($x), Offset::$y_type($y)),)+
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
// a representing the directions in a grid, without diagonals
impl2DDirection!(
    GridDirection[4] {
        Up(Pos(0), Pos(1)) - Down,
        Right(Pos(1), Pos(0)) - Left,
        Down(Pos(0), Neg(1)) - Up,
        Left(Neg(1), Pos(0)) - Right,
    }
);

// a representing the directions in a grid, including diagonals
impl2DDirection!(
    GridDiagDirection[8] {
        Up(Pos(0), Pos(1)) - Down,
        UpRight(Pos(1), Pos(1)) - DownLeft,
        Right(Pos(1), Pos(0)) - Left,
        DownRight(Pos(1), Neg(1)) - UpLeft,
        Down(Pos(0), Neg(1)) - Up,
        DownLeft(Neg(1), Neg(1)) - UpRight,
        Left(Neg(1), Pos(0)) - Right,
        UpLeft(Neg(1), Pos(1)) - DownRight,
    }
);

mod test {
    use super::*;

    #[test]
    fn offset_ord_test() {
        assert_eq!(Offset::Pos(0), Offset::Neg(0));
        assert_eq!(Offset::Neg(0), Offset::Pos(0));
        assert_eq!(Offset::Pos(1), Offset::Pos(1));
        assert_eq!(Offset::Neg(1), Offset::Neg(1));
        assert_ne!(Offset::Neg(0), Offset::Neg(1));
        assert_ne!(Offset::Pos(0), Offset::Pos(1));
        assert_ne!(Offset::Neg(1), Offset::Pos(1));
        assert_ne!(Offset::Pos(1), Offset::Neg(1));

        assert_eq!(Offset::Pos(0).cmp(&Offset::Neg(0)), Ordering::Equal);
        assert_eq!(Offset::Pos(0).cmp(&Offset::Pos(1)), Ordering::Less);
        assert_eq!(Offset::Neg(0).cmp(&Offset::Neg(1)), Ordering::Greater);
        assert_eq!(Offset::Neg(1).cmp(&Offset::Pos(1)), Ordering::Less);
        assert_eq!(Offset::Pos(1).cmp(&Offset::Neg(1)), Ordering::Greater);
    }
}
