use crate::{
    hypothetical::Hypothetical,
    search::{SearchingSet, SearchingTree, SetWrapper},
    structures::{
        directions::{DirectionEnumerable, DirectionReversable},
        AdjacencyStructure, DirectionStructure, NeighborhoodStructure,
    },
    trait_definitions::{Board, BoardToMap},
    IndexMap,
};
use std::{
    fmt::{self, Debug},
    iter, mem,
};

#[derive(Eq)]
pub struct Field<'a, B: Board> {
    board: &'a B,
    index: B::Index,
}

impl<'a, B: Board> Field<'a, B> {
    pub fn new(board: &'a B, index: B::Index) -> Option<Self> {
        if board.contains(index) {
            Some(Self { board, index })
        } else {
            None
        }
    }

    pub fn board(self) -> &'a B {
        self.board
    }

    pub fn index(self) -> B::Index {
        self.index
    }

    pub fn content(self) -> &'a B::Content {
        self.content_checked().unwrap_or_else(|| {
            panic!(
                "Index of field is invalid: {:?} - perhaps the field was removed from the board?",
                self.index
            )
        })
    }

    pub fn content_checked(self) -> Option<&'a B::Content> {
        self.board.get(self.index)
    }
}

// TODO good idea to compare pointer?
impl<'a, B: Board> PartialEq for Field<'a, B> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.board, other.board) && self.index == other.index
    }
}

impl<'a, B: Board> Clone for Field<'a, B> {
    fn clone(&self) -> Self {
        Field { ..*self }
    }
}

impl<'a, B: Board> Copy for Field<'a, B> {}

impl<B: Board> Debug for Field<'_, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Field {{ index: {:?} }}", self.index)
    }
}

impl<'a, B: Board> Field<'a, B>
where
    B::Content: Emptyable,
{
    pub fn is_empty(self) -> bool {
        self.content().call_field_is_empty()
    }
}

impl<'a, S, B: Board<Structure = S>> Field<'a, B>
where
    S: AdjacencyStructure<B>,
{
    pub fn is_adjacent<T: Into<B::Index>>(self, index: T) -> bool {
        self.board
            .structure()
            .is_adjacent(self.board, self.index, index.into())
    }
}

impl<'a, S, B: Board<Structure = S>> Field<'a, B>
where
    S: NeighborhoodStructure<B> + 'a,
{
    pub fn neighbor_count(self) -> usize {
        self.board
            .structure()
            .neighbor_count(self.board, self.index)
    }

    pub fn neighbors(self) -> impl Iterator<Item = Field<'a, B>> {
        let board = self.board;
        board
            .structure()
            .neighbors(board, self.index)
            .filter_map(move |i| Self::new(board, i))
    }
}

impl<'a, S, B: Board<Structure = S>> Field<'a, B>
where
    S: DirectionStructure<B>,
{
    pub fn next(self, direction: S::Direction) -> Option<Self> {
        let board = self.board;
        board
            .structure()
            .next(board, self.index, direction)
            .and_then(|i| Self::new(board, i))
    }

    pub fn has_next(self, direction: S::Direction) -> bool {
        let board = self.board;
        board.structure().has_next(board, self.index, direction)
    }

    pub fn neighbors_by_direction(self) -> impl Iterator<Item = (S::Direction, Field<'a, B>)>
    where
        S::Direction: DirectionEnumerable,
    {
        S::Direction::enumerate_all().filter_map(move |d| self.next(d).map(|f| (d, f)))
    }
}

impl<'a, B: Board, M: IndexMap<Item = B::Content>> Field<'a, Hypothetical<'a, B, M>>
where
    M: IndexMap<IndexType = B::Index, Item = B::Content>,
{
    pub fn original_field<'b>(&self, board: &'b B) -> Field<'b, B> {
        Field::new(board, self.index).unwrap_or_else(|| {
            panic!(
                "Index of field is invalid for original board: {:?}",
                self.index
            )
        })
    }
}

// ----- search methods -----

impl<'a, B> Field<'a, B>
where
    B: BoardToMap<()>,
{
    pub fn search(self) -> SearchingSet<'a, B::Map, B> {
        let mut set = SearchingSet::new(self.board);
        set.insert(self.index());
        set
    }
}

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

impl<'a, M, B> Field<'a, B>
where
    M: DirectionStructure<B>,
    B: BoardToMap<(), Structure = M>,
{
    /// Note that the first element is self.
    ///
    /// It is guaranteed that no field is visited twice.
    pub fn iter_line(&self, direction: M::Direction) -> impl Iterator<Item = Field<'a, B>> {
        let mut set = self.board().get_index_map().into();
        iter::successors(Some(*self), move |f| f.get_successor(direction, &mut set))
    }

    /// The iterator will first follow the line of the given direction
    /// while the predicate return true. Afterwards, it continues with the reverse direction.
    ///
    /// Note that the first element is self. In the case that the predicate rejects self, iteration still continues.
    ///
    /// It is guaranteed that no field is visited twice.
    pub fn iter_bidirectional<P>(
        &self,
        direction: M::Direction,
        predicate: P,
    ) -> Bidirectional<'a, M, B, P>
    where
        P: FnMut(Self) -> bool,
        M::Direction: DirectionReversable,
    {
        Bidirectional::new(*self, direction, predicate)
    }

    fn get_successor(&self, direction: M::Direction, set: &mut SetWrapper<B::Map>) -> Option<Self> {
        let field = self.next(direction)?;
        if set.insert(field.index()) {
            Some(field)
        } else {
            None
        }
    }
}

pub struct Bidirectional<'a, M, B, P>
where
    M: DirectionStructure<B>,
    B: Board<Structure = M> + BoardToMap<()>,
{
    root: Option<Field<'a, B>>,
    previous: Option<Field<'a, B>>,
    direction: M::Direction,
    set: SetWrapper<B::Map>,
    predicate: P,
}

impl<'a, M, B, P> Bidirectional<'a, M, B, P>
where
    M: DirectionStructure<B>,
    B: Board<Structure = M> + BoardToMap<()>,
{
    pub fn new(root: Field<'a, B>, direction: M::Direction, predicate: P) -> Self {
        Self {
            root: Some(root),
            previous: None,
            direction,
            set: root.board().get_index_map().into(),
            predicate,
        }
    }
}

impl<'a, M, B, P> Iterator for Bidirectional<'a, M, B, P>
where
    M: DirectionStructure<B>,
    M::Direction: DirectionReversable,
    B: BoardToMap<(), Structure = M>,
    P: FnMut(Field<'a, B>) -> bool,
{
    type Item = Field<'a, B>;

    fn next(&mut self) -> Option<Field<'a, B>> {
        match self.previous {
            // handle some edge cases for the root element
            None => {
                // unwrap is safe due to initialization
                let root = self.root.unwrap();
                debug_assert!(self.set.insert(root.index()));
                self.previous = Some(root);
                if (self.predicate)(root) {
                    Some(root)
                } else {
                    self.next()
                }
            }
            Some(field) => {
                let next = field
                    .get_successor(self.direction, &mut self.set)
                    .filter(|f| (self.predicate)(*f));
                match next {
                    Some(_) => {
                        self.previous = next;
                        next
                    }
                    None => {
                        // When the first line is finished: reset to root, switch direction and continue.
                        let root = self.root.take()?;
                        self.previous = Some(root);
                        self.direction = self.direction.reversed();
                        self.next()
                    }
                }
            }
        }
    }
}

// ----- contetns of fields -----

/// This trait is <b>not</b> intended to be used directly.
/// It is used to provide generic access functionality on a higher level (e.g. for Fields).
pub trait Emptyable: Default {
    fn call_field_is_empty(&self) -> bool;

    fn call_take_field(&mut self) -> Self {
        mem::take(self)
    }

    fn call_clear_field(&mut self) {
        self.call_take_field();
    }
}

impl<T> Emptyable for Option<T> {
    fn call_field_is_empty(&self) -> bool {
        self.is_none()
    }
}

impl<T> Emptyable for Vec<T> {
    fn call_field_is_empty(&self) -> bool {
        self.is_empty()
    }

    fn call_clear_field(&mut self) {
        self.clear()
    }
}
