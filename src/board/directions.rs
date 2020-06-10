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
            (Offset::Pos(a), Offset::Neg(b)) => {
                if *a == 0 && *b == 0 {
                    Some(Ordering::Equal)
                } else {
                    Some(Ordering::Greater)
                }
            }
            (Offset::Neg(a), Offset::Pos(b)) => {
                if *a == 0 && *b == 0 {
                    Some(Ordering::Equal)
                } else {
                    Some(Ordering::Less)
                }
            }
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
