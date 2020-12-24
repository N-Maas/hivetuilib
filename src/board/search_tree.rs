use super::{search::SetWrapper, *};

// TODO: method for removing field?!
// TODO: consider laziness
#[derive(Debug, Eq)]
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

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Clone for SearchingTree<'a, M, B>
where
    M: Clone,
{
    fn clone(&self) -> Self {
        Self {
            base_set: self.base_set.clone(),
            tree: self.tree.clone(),
            open_paths: self.open_paths.clone(),
            board: self.board,
        }
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> SearchingTree<'a, M, B> {
    pub fn new(board: &'a B) -> Self
    where
        M: From<&'a B>,
    {
        Self {
            base_set: M::from(board).into(),
            tree: Vec::new(),
            open_paths: Vec::new(),
            board,
        }
    }

    pub fn num_fields(&self) -> usize {
        self.base_set.size()
    }

    pub fn board(&self) -> &'a B {
        self.board
    }

    pub fn contains<T: Into<B::Index>>(&self, el: T) -> bool {
        self.base_set.contains(el.into())
    }

    pub fn insert_root<T: Into<B::Index>>(&mut self, el: T) {
        let idx = el.into();
        self.base_set.insert(idx);
        if !self.board.contains(idx) {
            panic!("Invalid index provided: {:?}", idx);
        }
        let parent = self.tree.len();
        self.tree.push((parent, idx));
        self.open_paths.push((parent, 1));
    }

    // retain paths, reopen_all_paths, iter_paths, ..
    // reopen_roots?
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
