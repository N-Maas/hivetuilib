use super::directions::*;
use super::*;

#[derive(Debug, Clone)]
pub struct AdjacencySet<I: BoardIdxType + Hash> {
    edges: HashSet<(I, I)>,
}

impl<I: BoardIdxType + Hash> AdjacencySet<I> {
    pub fn new() -> Self {
        Self {
            edges: HashSet::new(),
        }
    }

    pub fn add_directed(&mut self, i: I, j: I) {
        self.edges.insert((i, j));
    }

    pub fn add_undirected(&mut self, i: I, j: I) {
        self.edges.insert((i, j));
        self.edges.insert((j, i));
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = &(I, I)> {
        self.edges.iter()
    }
}

impl<I: BoardIdxType + Hash, B: Board<I> + ?Sized> AdjacencyStructure<I, B> for AdjacencySet<I> {
    fn is_adjacent(&self, _board: &B, i: I, j: I) -> bool {
        self.edges.contains(&(i, j))
    }
}

// ----- macros for simpler implementation of direction structures -----
macro_rules! implAdjacencyStructure {
    () => {
        fn is_adjacent(&self, board: &B, i: I, j: I) -> bool {
            D::enumerate_all()
                .filter_map(|d| self.next(board, i, d))
                .any(|index| index == j)
        }
    };
}

macro_rules! implNeighborhoodStructure {
    () => {
        fn neighbor_count(&self, board: &B, index: I) -> usize {
            D::enumerate_all()
                .filter_map(|d| self.next(board, index, d))
                .filter(|i| board.contains(*i))
                .count()
        }

        fn get_neighbors(&self, board: &B, index: I) -> Vec<I> {
            D::enumerate_all()
                .filter_map(|d| self.next(board, index, d))
                .filter(|i| board.contains(*i))
                .collect()
        }
    };
}

// ----- direction structures -----

#[derive(Debug, Clone, Copy)]
pub struct OffsetStructure<I: OffsetableIndex, D: DirectionOffset<I::Offset>> {
    _i: PhantomData<I>,
    _d: PhantomData<D>,
}

impl<I: OffsetableIndex, D: DirectionOffset<I::Offset>> OffsetStructure<I, D> {
    pub fn new() -> Self {
        Self {
            _i: PhantomData,
            _d: PhantomData,
        }
    }
}

impl<I: OffsetableIndex, B: Board<I> + ?Sized, D: DirectionOffset<I::Offset>>
    DirectionStructure<I, B> for OffsetStructure<I, D>
{
    type Direction = D;

    fn next(&self, board: &B, index: I, direction: D) -> Option<I> {
        index
            .apply_offset(direction.get_offset())
            .filter(|i| board.contains(*i))
    }
}

// TODO: good ideas? (might be inperformant)
impl<I: OffsetableIndex, B: Board<I> + ?Sized, D: DirectionOffset<I::Offset>>
    AdjacencyStructure<I, B> for OffsetStructure<I, D>
where
    D: DirectionEnumerable,
{
    implAdjacencyStructure!();
}

impl<I: OffsetableIndex, B: Board<I> + ?Sized, D: DirectionOffset<I::Offset>>
    NeighborhoodStructure<I, B> for OffsetStructure<I, D>
where
    D: DirectionEnumerable,
{
    implNeighborhoodStructure!();
}

#[derive(Debug, Clone, Copy)]
pub struct WrappedOffsetStructure<I: OffsetableIndex + Ord, D: DirectionOffset<I::Offset>> {
    _i: PhantomData<I>,
    _d: PhantomData<D>,
}

impl<I: OffsetableIndex + Ord, D: DirectionOffset<I::Offset>> WrappedOffsetStructure<I, D> {
    pub fn new() -> Self {
        Self {
            _i: PhantomData,
            _d: PhantomData,
        }
    }
}

impl<I: OffsetableIndex + Ord, B: ContiguousBoard<I> + ?Sized, D: DirectionOffset<I::Offset>>
    DirectionStructure<I, B> for WrappedOffsetStructure<I, D>
{
    type Direction = D;

    fn next(&self, board: &B, index: I, direction: D) -> Option<I> {
        index
            .apply_offset(direction.get_offset())
            .map(|i| board.wrapped(i))
            .filter(|i| board.contains(*i))
    }
}

// TODO: good ideas? (might be inperformant)
impl<I: OffsetableIndex + Ord, B: ContiguousBoard<I> + ?Sized, D: DirectionOffset<I::Offset>>
    AdjacencyStructure<I, B> for WrappedOffsetStructure<I, D>
where
    D: DirectionEnumerable,
{
    implAdjacencyStructure!();
}

impl<I: OffsetableIndex + Ord, B: ContiguousBoard<I> + ?Sized, D: DirectionOffset<I::Offset>>
    NeighborhoodStructure<I, B> for WrappedOffsetStructure<I, D>
where
    D: DirectionEnumerable,
{
    implNeighborhoodStructure!();
}
