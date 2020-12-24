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

#[derive(Debug, Eq)]
pub struct Path<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    tree_index: usize,
    length: usize,
    searching_tree: &'a SearchingTree<'a, M, B>,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Path<'a, M, B> {
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn searching_tree(&self) -> &'a SearchingTree<'a, M, B> {
        self.searching_tree
    }

    /// Warning: this method requires O(n) time where n is the length of the path.
    pub fn contains<T: Into<B::Index>>(&self, el: T) -> bool {
        let idx = el.into();
        self.iter_points().any(|f| f.index() == idx)
    }

    pub fn endpoint(&self) -> Field<'a, B> {
        let (_, index) = self.tree_values();
        // index is required to be valid
        self.searching_tree.board.get_field(index).unwrap()
    }

    pub fn iter_points(&self) -> PointIter<'a, M, B> {
        PointIter {
            inner: self.iter_subpaths(),
        }
    }

    // TODO: implement Index for Range<usize>?
    pub fn subpath(&self, start: usize, end: usize) -> Option<Self> {
        if end > self.length {
            return None;
        }
        let mut path = Path {
            length: end,
            ..*self
        };
        let mut remaining = start;
        while remaining > 0 {
            path = path.next_subpath()?;
            remaining -= 1;
        }
        Some(path)
    }

    pub fn next_subpath(&self) -> Option<Self> {
        if self.length > 1 {
            let (parent, index) = self.tree_values();
            if parent == self.tree_index {
                panic!("Path is of length >1, but has only one point: {:?}", index);
            }
            Some(Self::new(parent, self.length - 1, self.searching_tree))
        } else {
            None
        }
    }

    pub fn iter_subpaths(&self) -> PathIter<'a, M, B> {
        PathIter {
            current: Some(*self),
        }
    }

    fn new(tree_index: usize, length: usize, searching_tree: &'a SearchingTree<'a, M, B>) -> Self {
        let result = Self {
            tree_index: tree_index,
            length,
            searching_tree,
        };
        let (_, index) = result.tree_values();
        if !searching_tree.board.contains(index) {
            panic!("Invalid index at path construction: {:?}", index);
        }
        result
    }

    fn tree_values(&self) -> (usize, B::Index) {
        self.searching_tree.tree[self.tree_index]
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> PartialEq for Path<'a, M, B> {
    fn eq(&self, other: &Self) -> bool {
        self.tree_index == other.tree_index && self.length == other.length
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Clone for Path<'a, M, B> {
    fn clone(&self) -> Self {
        Self {
            tree_index: self.tree_index,
            length: self.length,
            searching_tree: self.searching_tree,
        }
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Copy for Path<'a, M, B> {}

#[derive(Debug, Clone)]
pub struct PathIter<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    current: Option<Path<'a, M, B>>,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Iterator for PathIter<'a, M, B> {
    type Item = Path<'a, M, B>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.current;
        self.current = self.current.and_then(|p| p.next_subpath());
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.current.map_or(0, |p| p.len());
        (len, Some(len))
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> ExactSizeIterator
    for PathIter<'a, M, B>
{
}

#[derive(Debug, Clone)]
pub struct PointIter<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    inner: PathIter<'a, M, B>,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Iterator for PointIter<'a, M, B> {
    type Item = Field<'a, B>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|p| p.endpoint())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[cfg(test)]
mod test {
    use crate::board::{
        vec_board::{Index1D, VecBoard},
        BoardToMap,
    };

    use super::{Path, SearchingTree};

    #[test]
    fn basic_test() {
        type TestBoard = VecBoard<usize>;

        let board = TestBoard::with_default(1, ());
        let mut tree = SearchingTree::<<TestBoard as BoardToMap<()>>::Map, TestBoard>::new(&board);
        let path_1 = Path {
            tree_index: 0,
            length: 0,
            searching_tree: &tree,
        };
        let path_2 = path_1;
        assert!(path_1 == path_2);
        tree.insert_root(Index1D::from(0));
    }
}
