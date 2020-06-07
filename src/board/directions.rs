use super::*;

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

// TODO: trait for direction -> index mapping (efficient structure)
// TODO: derive macro for Enumerable/index mapping

pub enum Offset {
    Neg(usize),
    Pos(usize),
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
