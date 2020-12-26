use super::{
    search::{FieldSearchResult, SetWrapper},
    *,
};

impl<'a, B> Field<'a, B>
where
    B: BoardToMap<()>,
{
    pub fn search_tree(self) -> SearchingTree<'a, B::Map, B> {
        let mut tree = SearchingTree::new(self.board);
        tree.insert_root(self.index());
        tree
    }
}

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
        B: BoardToMap<(), Map = M>,
    {
        Self {
            base_set: board.get_index_map().into(),
            tree: Vec::new(),
            open_paths: Vec::new(),
            board,
        }
    }

    pub fn num_fields(&self) -> usize {
        self.base_set.size()
    }

    pub fn num_active_paths(&self) -> usize {
        self.open_paths.len()
    }

    pub fn board(&self) -> &'a B {
        self.board
    }

    pub fn contains<T: Into<B::Index>>(&self, el: T) -> bool {
        self.base_set.contains(el.into())
    }

    pub fn iter_paths(&self) -> PathIter<M, B> {
        PathIter {
            inner: self.open_paths.iter(),
            searching_tree: self,
        }
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

    // TODO: a bit of code duplication here
    pub fn extend_with<F>(&mut self, mut map_fields: F) -> bool
    where
        F: FnMut(&Self, Path<M, B>) -> FieldSearchResult<B::Index>,
    {
        let old_paths = mem::replace(&mut self.open_paths, Vec::new());
        let mut success = false;
        for (tree_index, length) in old_paths {
            let path = Path::new(tree_index, length, self);
            let old_idx = path.endpoint().index();
            let new_indices = map_fields(self, path);

            for i in new_indices {
                success = true;
                if i == old_idx {
                    self.insert_new_endpoint(i, self.tree[tree_index].0, length);
                } else {
                    self.insert_new_endpoint(i, tree_index, length + 1);
                }
            }
        }
        success
    }

    pub fn extend<F>(&mut self, map_fields: F, mode: SearchMode) -> bool
    where
        F: FnMut(Field<B>) -> FieldSearchResult<B::Index>,
    {
        match mode {
            SearchMode::NewFieldsOnly => {
                self.extend_by_overlap(map_fields, |i, t, _| t.contains(i))
            }
            SearchMode::NoCycles => self.extend_by_overlap(map_fields, |i, _, p| p.contains(i)),
            SearchMode::AnyFields => self.extend_by_overlap(map_fields, |_, _, _| false),
        }
    }

    pub fn grow<F>(&mut self, mut predicate: F, mode: SearchMode) -> bool
    where
        F: FnMut(Field<B>) -> bool,
        B::Structure: NeighborhoodStructure<B>,
    {
        self.extend(
            |f| {
                f.neighbors()
                    .filter(|&n| predicate(n))
                    .map(|n| n.index())
                    .collect()
            },
            mode,
        )
    }

    fn extend_by_overlap<F, G>(&mut self, mut map_fields: F, is_overlap: G) -> bool
    where
        F: FnMut(Field<B>) -> FieldSearchResult<B::Index>,
        G: Fn(B::Index, &Self, Path<M, B>) -> bool,
    {
        let old_paths = mem::replace(&mut self.open_paths, Vec::new());
        let mut success = false;
        for (tree_index, length) in old_paths {
            let field = Path::new(tree_index, 1, self).endpoint();
            let old_idx = field.index();
            let new_indices = map_fields(field);

            for i in new_indices {
                if !is_overlap(i, self, Path::new(tree_index, length, self)) {
                    success = true;
                    if i == old_idx {
                        self.insert_new_endpoint(i, self.tree[tree_index].0, length);
                    } else {
                        self.insert_new_endpoint(i, tree_index, length + 1);
                    }
                }
            }
        }
        success
    }

    fn insert_new_endpoint(&mut self, i: B::Index, parent: usize, length: usize) {
        if !self.board.contains(i) {
            panic!("Field with invalid index: {:?}", i);
        }
        let tree_index = self.tree.len();
        self.base_set.insert(i);
        self.tree.push((parent, i));
        self.open_paths.push((tree_index, length));
    }

    // retain paths, reopen_all_paths, iter_fields, ..
    // reopen_roots?
    // perform_dfs
}

pub enum SearchMode {
    NewFieldsOnly,
    NoCycles,
    AnyFields,
}

pub struct PathIter<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    inner: Iter<'a, (usize, usize)>,
    searching_tree: &'a SearchingTree<'a, M, B>,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Iterator for PathIter<'a, M, B> {
    type Item = Path<'a, M, B>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|&(tree_index, length)| Path::new(tree_index, length, self.searching_tree))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> ExactSizeIterator
    for PathIter<'a, M, B>
{
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

    pub fn iter_subpaths(&self) -> SubpathIter<'a, M, B> {
        SubpathIter {
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
pub struct SubpathIter<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    current: Option<Path<'a, M, B>>,
}

impl<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> Iterator
    for SubpathIter<'a, M, B>
{
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
    for SubpathIter<'a, M, B>
{
}

#[derive(Debug, Clone)]
pub struct PointIter<'a, M: IndexMap<Item = ()>, B: Board<Index = M::IndexType>> {
    inner: SubpathIter<'a, M, B>,
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
        directions::{BinaryDirection, GridDiagDirection},
        matrix_board::*,
        structures::{OffsetStructure, WrappedOffsetStructure},
        vec_board::*,
        BoardToMap,
    };

    use super::{Path, SearchMode, SearchingTree};

    #[test]
    fn path_test() {
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

    #[test]
    fn basic_test() {
        type TestBoard = MatrixBoard<usize, OffsetStructure<Index2D, GridDiagDirection>>;

        let board = TestBoard::with_default(2, 2, OffsetStructure::new());
        let mut tree = SearchingTree::<<TestBoard as BoardToMap<()>>::Map, TestBoard>::new(&board);
        tree.insert_root(Index2D::from((0, 0)));
        tree.extend_with(|_, path| path.endpoint().neighbors().collect());
        let paths = tree.iter_paths().collect::<Vec<_>>();
        assert_eq!(paths.len(), 3);
        let expected = vec![
            Index2D::from((1, 0)),
            Index2D::from((1, 1)),
            Index2D::from((0, 1)),
        ];
        assert!((paths[0] != paths[1]) && (paths[1]) != (paths[2]) && (paths[0] != paths[2]));
        for p in paths {
            assert_eq!(p.len(), 2);
            assert!(expected.contains(&p.endpoint().index()));
        }
    }

    #[test]
    fn no_cylce_mode_test() {
        type TestBoard = VecBoard<usize, WrappedOffsetStructure<Index1D, BinaryDirection>>;

        let board = TestBoard::with_default(3, WrappedOffsetStructure::new());
        let mut tree = SearchingTree::<<TestBoard as BoardToMap<()>>::Map, TestBoard>::new(&board);
        tree.insert_root(Index1D::from(1));
        tree.extend(|f| f.neighbors().collect(), SearchMode::NoCycles);
        tree.extend(|f| f.neighbors().collect(), SearchMode::NoCycles);
        let paths = tree.iter_paths().collect::<Vec<_>>();
        assert_eq!(paths.len(), 2);
        let expected = vec![Index1D::from(0), Index1D::from(2)];
        assert!(paths[0] != paths[1]);
        for p in paths {
            assert_eq!(p.len(), 3);
            assert!(expected.contains(&p.endpoint().index()));
        }
    }
}
