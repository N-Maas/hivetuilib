use super::{search::SetWrapper, *};

// TODO: method for removing field?!
// TODO: consider laziness
#[derive(Debug, Eq, Clone)]
pub struct SearchingTree<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    base_set: SetWrapper<M>,
    // (parent, field)
    tree: Vec<(usize, B::Index)>,
    // (index in tree, length)
    open_paths: Vec<(usize, usize)>,
    board: &'a B,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> PartialEq
    for SearchingTree<'a, M, B>
where
    M: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.base_set.eq(&other.base_set)
            && self.tree.eq(&other.tree)
            && self.open_paths.eq(&other.open_paths)
    }
}

// ----- implementation of the Path API for a SearchingTree ----

#[derive(Debug, Eq, Clone, Copy)]
pub struct Path<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    index: usize,
    length: usize,
    tree: &'a SearchingTree<'a, M, B>,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> PartialEq for Path<'a, M, B>
{
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.length == other.length
    }
}
