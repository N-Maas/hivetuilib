use std::{cmp::Ordering, convert::TryFrom, fmt::Debug, mem, ops::ControlFlow, usize};

use tgp::{GameData, RevEffect};

use crate::{engine_stepper::EngineStepper, IndexType, RatingType, INTERNAL_ERROR};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TreeIndex(usize, usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TreeEntry {
    pub rating: RatingType,
    pub indizes: Box<[IndexType]>,
    pub num_children: IndexType,
}

impl PartialOrd for TreeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TreeEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rating.cmp(&other.rating)
    }
}

impl TreeEntry {
    pub fn new((rating, indizes): (RatingType, Box<[IndexType]>)) -> Self {
        Self {
            rating,
            indizes,
            num_children: 0,
        }
    }

    pub fn num_children(&self) -> usize {
        usize::try_from(self.num_children).unwrap()
    }
}

pub(crate) struct RetainedMoves<'a> {
    inner: &'a mut Vec<usize>,
    offset: usize,
}

impl<'a> RetainedMoves<'a> {
    fn new(inner: &'a mut Vec<usize>, offset: usize) -> Self {
        Self { inner, offset }
    }

    pub(crate) fn add(&mut self, val: usize) {
        assert!(self.inner.last().map_or(true, |&x| val + self.offset > x));
        self.inner.push(val + self.offset);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SearchTreeState {
    tree: Vec<Vec<TreeEntry>>,
    next_levels: Option<(Vec<TreeEntry>, Vec<TreeEntry>)>,
}

impl SearchTreeState {
    pub fn new() -> Self {
        Self {
            tree: vec![vec![
                // sentinel
                TreeEntry {
                    rating: 0,
                    indizes: Box::from(Vec::new()),
                    num_children: 0,
                },
            ]],
            next_levels: None,
        }
    }

    pub fn depth(&self) -> usize {
        self.tree.len() - 1
    }

    // TODO: not a good API
    pub fn root_moves(&self) -> impl Iterator<Item = (RatingType, &[IndexType])> + '_ {
        self.tree
            .get(1)
            .expect(INTERNAL_ERROR)
            .iter()
            .map(|entry| (entry.rating, entry.indizes.as_ref()))
    }

    pub fn new_levels(&mut self) {
        assert!(self.next_levels.is_none(), "{}", INTERNAL_ERROR);
        self.next_levels = Some((Vec::new(), Vec::new()));
    }

    /// Returns true if fully successful and false if children or grandchildren are empty
    /// (note that children are still pushed if grandchildren are empty)
    pub fn extend(&mut self) -> bool {
        let (children, g_children) = self.next_levels.take().expect(INTERNAL_ERROR);
        if !children.is_empty() && !g_children.is_empty() {
            self.tree.push(children);
            self.tree.push(g_children);
            true
        } else if !children.is_empty() {
            self.tree.push(children);
            false
        } else {
            false
        }
    }

    pub fn push_child<I>(
        &mut self,
        parent: TreeIndex,
        rating: RatingType,
        indizes: Box<[IndexType]>,
        grandchildren: I,
    ) where
        I: IntoIterator<Item = (RatingType, Box<[IndexType]>)>,
    {
        assert!(parent.0 == self.depth(), "{}", INTERNAL_ERROR);

        let (children, g_children) = self.next_levels.as_mut().expect(INTERNAL_ERROR);
        let mut count = 0;
        g_children.extend(grandchildren.into_iter().map(TreeEntry::new).inspect(|_| {
            count += 1;
        }));
        children.push(TreeEntry {
            rating,
            indizes,
            num_children: count,
        });
        self.entry_mut(parent).num_children += 1;
    }

    pub fn update_ratings(&mut self) {
        for i in (1..self.depth()).rev() {
            let (l, r) = self.tree.split_at_mut(i + 1);
            let moves = l.last_mut().expect(INTERNAL_ERROR);
            let children = r.first().expect(INTERNAL_ERROR);

            let mut start = 0;
            for entry in moves {
                let children = &children[start..start + entry.num_children()];
                let is_own_turn = (i % 2) == 0;
                let new_value = if is_own_turn {
                    children.iter().max()
                } else {
                    children.iter().min()
                };
                if let Some(e) = new_value {
                    entry.rating = e.rating;
                }
                start += entry.num_children();
            }
        }
    }

    pub fn prune<F>(&mut self, mut retain_fn: F)
    where
        F: FnMut(usize, &[TreeEntry], RetainedMoves),
    {
        // sentinel
        let mut old_retained = vec![0];
        let mut current_retained = Vec::new();
        for i in 1..self.tree.len() {
            let (left, right) = self.tree.split_at_mut(i);
            let old_moves = left.last_mut().unwrap();
            let moves = right.first().unwrap();
            let mut offset = 0;
            let mut retained = old_retained.iter().copied().peekable();
            // compute retained children
            for (j, entry) in old_moves.iter_mut().enumerate() {
                // if the entry is retained, continue to prune its children
                if retained.peek() == Some(&j) {
                    let old_len = current_retained.len();
                    if entry.num_children() > 1 {
                        retain_fn(
                            i - 1,
                            &moves[offset..offset + entry.num_children()],
                            RetainedMoves::new(&mut current_retained, offset),
                        );
                    } else if entry.num_children() == 1 {
                        current_retained.push(offset);
                    }
                    retained.next();
                    entry.num_children = (current_retained.len() - old_len) as u32;
                }
                offset += entry.num_children();
            }

            // remove pruned elements
            Self::retain_by_index(&mut self.tree[i - 1], &old_retained);
            old_retained = mem::take(&mut current_retained);
        }
        // last level
        Self::retain_by_index(self.tree.last_mut().unwrap(), &old_retained);
    }

    fn retain_by_index<T: Clone>(vec: &mut Vec<T>, indices: &[usize]) {
        let mut retained = indices.iter().copied().peekable();
        *vec = vec
            .iter()
            .enumerate()
            .filter(|(j, _)| {
                if retained.peek() == Some(&j) {
                    retained.next();
                    true
                } else {
                    false
                }
            })
            .map(|(_, x)| x.clone())
            .collect();
    }

    /// f must return the engine in the same state as before
    pub fn for_each_leaf<T: GameData + Debug, F, E>(
        &mut self,
        stepper: &mut EngineStepper<T>,
        mut function: F,
    ) -> Result<(), E>
    where
        T::EffectType: RevEffect<T>,
        F: FnMut(&mut Self, &mut EngineStepper<T>, TreeIndex) -> ControlFlow<E>,
    {
        let mut children_start = vec![0; self.tree.len()];
        self.for_each_leaf_impl(stepper, &mut function, TreeIndex(0, 0), &mut children_start)
    }

    fn for_each_leaf_impl<T: GameData + Debug, F, E>(
        &mut self,
        stepper: &mut EngineStepper<T>,
        function: &mut F,
        t_index: TreeIndex,
        children_start: &mut Vec<usize>,
    ) -> Result<(), E>
    where
        T::EffectType: RevEffect<T>,
        F: FnMut(&mut Self, &mut EngineStepper<T>, TreeIndex) -> ControlFlow<E>,
    {
        let depth = t_index.0 + 1;
        if depth == self.tree.len() {
            return match function(self, stepper, t_index) {
                ControlFlow::Continue(()) => Ok(()),
                ControlFlow::Break(err) => Err(err),
            };
        }

        let offset = children_start[t_index.0];
        for child in 0..self.entry(t_index).num_children() {
            let entry = &self.get_level(depth)[child + offset];
            let num_children = entry.num_children();
            stepper.forward_step(&entry.indizes);
            let err = self.for_each_leaf_impl(
                stepper,
                function,
                TreeIndex(depth, child + offset),
                children_start,
            );
            stepper.backward_step();
            children_start[depth] += num_children;
            err?;
        }
        Ok(())
    }

    fn get_level(&self, index: usize) -> &[TreeEntry] {
        &self.tree.get(index).expect(INTERNAL_ERROR)
    }

    fn entry(&self, index: TreeIndex) -> &TreeEntry {
        &self.tree[index.0][index.1]
    }

    fn entry_mut(&mut self, index: TreeIndex) -> &mut TreeEntry {
        &mut self.tree[index.0][index.1]
    }
}

#[cfg(test)]
mod test {
    use std::ops::ControlFlow;

    use tgp::engine::Engine;

    use crate::{
        engine_stepper::EngineStepper,
        search_tree_state::{TreeEntry, TreeIndex},
        test::ZeroOneGame,
        IndexType, RatingType,
    };

    use super::SearchTreeState;

    fn indizes(input: &[IndexType]) -> Box<[IndexType]> {
        Box::from(input)
    }

    #[test]
    fn build_search_tree_test() {
        let mut sts = SearchTreeState::new();
        assert_eq!(sts.depth(), 0);

        sts.new_levels();
        sts.push_child(TreeIndex(0, 0), 0, indizes(&[1]), vec![(0, indizes(&[1]))]);
        sts.push_child(
            TreeIndex(0, 0),
            33,
            indizes(&[3]),
            vec![(333, indizes(&[3]))],
        );
        sts.push_child(
            TreeIndex(0, 0),
            -11,
            indizes(&[2]),
            vec![(-111, indizes(&[2])), (0, indizes(&[0]))],
        );
        sts.push_child(
            TreeIndex(0, 0),
            77,
            indizes(&[0]),
            vec![(123, indizes(&[])), (-123, indizes(&[]))],
        );
        sts.extend();

        assert_eq!(
            sts.tree[0],
            vec![TreeEntry {
                rating: 0,
                indizes: indizes(&[]),
                num_children: 4
            },]
        );
        assert_eq!(
            sts.tree[1],
            vec![
                TreeEntry {
                    rating: 0,
                    indizes: indizes(&[1]),
                    num_children: 1
                },
                TreeEntry {
                    rating: 33,
                    indizes: indizes(&[3]),
                    num_children: 1
                },
                TreeEntry {
                    rating: -11,
                    indizes: indizes(&[2]),
                    num_children: 2
                },
                TreeEntry {
                    rating: 77,
                    indizes: indizes(&[0]),
                    num_children: 2
                }
            ]
        );
        assert_eq!(
            sts.tree[2],
            vec![
                TreeEntry::new((0, indizes(&[1]))),
                TreeEntry::new((333, indizes(&[3]))),
                TreeEntry::new((-111, indizes(&[2]))),
                TreeEntry::new((0, indizes(&[0]))),
                TreeEntry::new((123, indizes(&[]))),
                TreeEntry::new((-123, indizes(&[]))),
            ]
        );

        sts.prune(|_, elements, mut retainer| {
            for (i, entry) in elements.iter().enumerate() {
                if entry.rating > 0 {
                    retainer.add(i);
                }
            }
        });
        assert_eq!(
            sts.tree[1],
            vec![
                TreeEntry {
                    rating: 33,
                    indizes: indizes(&[3]),
                    num_children: 1
                },
                TreeEntry {
                    rating: 77,
                    indizes: indizes(&[0]),
                    num_children: 1
                }
            ]
        );
        assert_eq!(
            sts.tree[2],
            vec![
                TreeEntry::new((333, indizes(&[3]))),
                TreeEntry::new((123, indizes(&[])))
            ]
        );
    }

    #[test]
    fn iteration_test() {
        let mut sts = SearchTreeState::new();
        // TODO: initialization is a bit broken
        sts.tree[0].first_mut().unwrap().num_children = 2;
        sts.tree.push(vec![
            TreeEntry::new((-1, indizes(&[0]))),
            TreeEntry::new((1, indizes(&[1]))),
        ]);

        let data = ZeroOneGame::new(false, 4);
        let mut engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(&mut engine);

        sts.new_levels();
        sts.for_each_leaf(&mut stepper, |tree_state, stepper, t_index| {
            assert_eq!(stepper.decision_context().len(), 1);
            let rating =
                stepper.data().num_ones as RatingType - stepper.data().num_zeros as RatingType;
            tree_state.push_child(
                t_index,
                rating,
                indizes(&[0, 1]),
                vec![(rating + 1, indizes(&[1]))],
            );
            tree_state.push_child(
                t_index,
                rating,
                indizes(&[1, 0]),
                vec![(rating + 1, indizes(&[1]))],
            );
            tree_state.push_child(
                t_index,
                rating + 2,
                indizes(&[1, 1]),
                vec![(rating + 1, indizes(&[0])), (rating + 3, indizes(&[1]))],
            );
            ControlFlow::<()>::Continue(())
        })
        .unwrap();
        sts.extend();
        assert_eq!(sts.tree.last().unwrap().len(), 8);

        sts.for_each_leaf(&mut stepper, |tree_state, stepper, t_index| {
            assert_eq!(stepper.decision_context().len(), 3);
            let expected_rating =
                stepper.data().num_ones as RatingType - stepper.data().num_zeros as RatingType;
            let tree_rating = tree_state.entry(t_index).rating;
            assert_eq!(expected_rating, tree_rating);
            ControlFlow::<()>::Continue(())
        })
        .unwrap();

        sts.update_ratings();
        assert_eq!(
            sts.tree[1],
            vec![
                TreeEntry {
                    rating: 0,
                    indizes: indizes(&[0]),
                    num_children: 3
                },
                TreeEntry {
                    rating: 2,
                    indizes: indizes(&[1]),
                    num_children: 3
                }
            ]
        );
    }
}
