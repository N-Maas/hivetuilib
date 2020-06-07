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
    DirectionStructure<I, B, D> for OffsetStructure<I, D>
{
    // TODO check validity of index?
    fn next(&self, _board: &B, index: I, direction: D) -> Option<I> {
        index.apply_offset(direction.get_offset())
    }
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
    DirectionStructure<I, B, D> for WrappedOffsetStructure<I, D>
{
    // TODO check validity of index?
    fn next(&self, board: &B, index: I, direction: D) -> Option<I> {
        index
            .apply_offset(direction.get_offset())
            .map(|i| board.wrapped(i))
    }
}
