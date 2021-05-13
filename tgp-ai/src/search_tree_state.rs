use std::{cmp::Ordering, convert::TryFrom, mem, usize};

use tgp::{GameData, RevEffect};

use crate::{
    engine_stepper::EngineStepper, rater::DecisionType, IndexType, RatingType, INTERNAL_ERROR,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TreeIndex(usize, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord)]
pub(crate) struct TreeEntry {
    pub rating: RatingType,
    pub index: IndexType,
    pub num_children: IndexType,
}

impl PartialOrd for TreeEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.rating.partial_cmp(&other.rating)
    }
}

impl TreeEntry {
    pub fn new((rating, index): (RatingType, IndexType)) -> Self {
        Self {
            rating,
            index,
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
        debug_assert!(self.inner.last().map_or(true, |&x| val > x));
        self.inner.push(val + self.offset);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SearchTreeState {
    tree: Vec<Vec<TreeEntry>>,
    next_levels: Option<(Vec<TreeEntry>, Vec<TreeEntry>)>,
}

impl SearchTreeState {
    pub fn new<I>(initial_moves: I) -> Self
    where
        I: IntoIterator<Item = (RatingType, IndexType)>,
    {
        let initial_moves = initial_moves
            .into_iter()
            .map(TreeEntry::new)
            .collect::<Vec<_>>();
        let num_children = IndexType::try_from(initial_moves.len()).unwrap();
        Self {
            tree: vec![
                vec![
                    // sentinel
                    TreeEntry {
                        rating: 0,
                        index: 0,
                        num_children,
                    },
                ],
                initial_moves,
            ],
            next_levels: None,
        }
    }

    pub fn depth(&self) -> usize {
        self.tree.len() - 1
    }

    pub fn new_levels(&mut self) {
        assert!(self.next_levels.is_none(), "{}", INTERNAL_ERROR);
        self.next_levels = Some((Vec::new(), Vec::new()));
    }

    pub fn extend(&mut self) {
        let (children, g_children) = self.next_levels.take().expect(INTERNAL_ERROR);
        self.tree.push(children);
        self.tree.push(g_children);
    }

    pub fn push_child<I>(
        &mut self,
        parent: TreeIndex,
        rating: RatingType,
        index: IndexType,
        grandchildren: I,
    ) where
        I: IntoIterator<Item = (RatingType, IndexType)>,
    {
        assert!(parent.0 == self.depth(), "{}", INTERNAL_ERROR);

        let (children, g_children) = self.next_levels.as_mut().expect(INTERNAL_ERROR);
        let mut count = 0;
        g_children.extend(grandchildren.into_iter().map(TreeEntry::new).inspect(|_| {
            count += 1;
        }));
        children.push(TreeEntry {
            rating,
            index,
            num_children: count,
        });
        self.entry_mut(parent).num_children += 1;
    }

    pub fn update_ratings(&mut self) {
        for i in (1..self.depth()).rev() {
            let is_own_turn = (i % 2) == 1;
            let (l, r) = self.tree.split_at_mut(i + 1);
            let moves = l.last_mut().expect(INTERNAL_ERROR);
            let children = r.first().expect(INTERNAL_ERROR);

            let mut start = 0;
            for entry in moves {
                let children = &children[start..start + entry.num_children()];
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
            let moves = &self.tree[i];
            let mut offset = 0;
            let mut retained = old_retained.iter().copied().peekable();
            // compute retained children
            for (j, entry) in self.tree[i - 1].iter().enumerate() {
                // if the entry is retained, continue to prune its children
                if retained.peek() == Some(&j) {
                    retain_fn(
                        i,
                        &moves[offset..offset + entry.num_children()],
                        RetainedMoves::new(&mut current_retained, offset),
                    );
                    retained.next();
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

    fn retain_by_index<T: Copy>(vec: &mut Vec<T>, indices: &[usize]) {
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
            .map(|(_, &x)| x)
            .collect();
    }

    /// f must return the engine in the same state as before
    pub fn for_each_leaf<T: GameData, F, M>(
        &mut self,
        stepper: &mut EngineStepper<T, M>,
        mut function: F,
    ) where
        T::EffectType: RevEffect<T>,
        F: FnMut(&mut Self, &mut EngineStepper<T, M>, TreeIndex),
        M: Fn(&T::Context) -> DecisionType,
    {
        assert!(self.depth() > 0);
        let mut children_start = vec![0; self.tree.len()];
        self.for_each_leaf_impl(stepper, &mut function, TreeIndex(0, 0), &mut children_start);
    }

    fn for_each_leaf_impl<T: GameData, F, M>(
        &mut self,
        stepper: &mut EngineStepper<T, M>,
        function: &mut F,
        t_index: TreeIndex,
        children_start: &mut Vec<usize>,
    ) where
        T::EffectType: RevEffect<T>,
        F: FnMut(&mut Self, &mut EngineStepper<T, M>, TreeIndex),
        M: Fn(&T::Context) -> DecisionType,
    {
        let depth = t_index.0 + 1;
        if depth == self.tree.len() {
            function(self, stepper, t_index);
        } else {
            let offset = children_start[t_index.0];
            for child in 0..self.entry(t_index).num_children() {
                let index = child + offset;
                let entry = self.get_level(depth)[index];
                stepper.forward_step(entry.index);
                self.for_each_leaf_impl(stepper, function, TreeIndex(depth, index), children_start);
                stepper.backward_step();
                children_start[depth] += entry.num_children();
            }
        }
    }

    fn get_level(&self, index: usize) -> &[TreeEntry] {
        &self.tree.get(index).expect(INTERNAL_ERROR)
    }

    fn entry(&self, index: TreeIndex) -> TreeEntry {
        self.tree[index.0][index.1]
    }

    fn entry_mut(&mut self, index: TreeIndex) -> &mut TreeEntry {
        &mut self.tree[index.0][index.1]
    }
}

#[cfg(test)]
mod test {
    use tgp::engine::Engine;

    use crate::{
        engine_stepper::EngineStepper,
        search_tree_state::{TreeEntry, TreeIndex},
        test::{type_mapping, ZeroOneGame},
        RatingType,
    };

    use super::SearchTreeState;

    #[test]
    fn build_search_tree_test() {
        let mut sts = SearchTreeState::new(vec![(0, 1), (3, 3), (-1, 2)]);
        assert_eq!(sts.depth(), 1);
        assert_eq!(sts.tree.last().unwrap().len(), 3);

        sts.new_levels();
        sts.push_child(TreeIndex(1, 0), 0, 1, vec![(0, 1)]);
        sts.push_child(TreeIndex(1, 1), 33, 3, vec![(333, 3)]);
        sts.push_child(TreeIndex(1, 2), -11, 2, vec![(-111, 2), (0, 0)]);
        sts.extend();

        assert_eq!(
            sts.tree[1],
            vec![
                TreeEntry {
                    rating: 0,
                    index: 1,
                    num_children: 1
                },
                TreeEntry {
                    rating: 3,
                    index: 3,
                    num_children: 1
                },
                TreeEntry {
                    rating: -1,
                    index: 2,
                    num_children: 1
                }
            ]
        );
        assert_eq!(
            sts.tree[2],
            vec![
                TreeEntry {
                    rating: 0,
                    index: 1,
                    num_children: 1
                },
                TreeEntry {
                    rating: 33,
                    index: 3,
                    num_children: 1
                },
                TreeEntry {
                    rating: -11,
                    index: 2,
                    num_children: 2
                }
            ]
        );
        assert_eq!(
            sts.tree[3],
            vec![
                TreeEntry::new((0, 1)),
                TreeEntry::new((333, 3)),
                TreeEntry::new((-111, 2)),
                TreeEntry::new((0, 0)),
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
            vec![TreeEntry {
                rating: 3,
                index: 3,
                num_children: 1
            }]
        );
        assert_eq!(
            sts.tree[2],
            vec![TreeEntry {
                rating: 33,
                index: 3,
                num_children: 1
            }]
        );
        assert_eq!(sts.tree[3], vec![TreeEntry::new((333, 3)),]);
    }

    #[test]
    fn iteration_test() {
        let mut sts = SearchTreeState::new(vec![(-1, 0), (1, 1)]);
        let data = ZeroOneGame::new(false, 4);
        let mut stepper = EngineStepper::new(Engine::new_logging(2, data), type_mapping);

        sts.new_levels();
        sts.for_each_leaf(&mut stepper, |tree_state, stepper, t_index| {
            let rating =
                stepper.data().num_ones as RatingType - stepper.data().num_zeros as RatingType;
            tree_state.push_child(t_index, rating, 1, vec![(rating + 1, 1)]);
            tree_state.push_child(t_index, rating, 2, vec![(rating + 1, 1)]);
            tree_state.push_child(
                t_index,
                rating + 2,
                3,
                vec![(rating + 1, 0), (rating + 3, 1)],
            );
        });
        sts.extend();
        assert_eq!(sts.tree.last().unwrap().len(), 8);

        sts.for_each_leaf(&mut stepper, |tree_state, stepper, t_index| {
            let expected_rating =
                stepper.data().num_ones as RatingType - stepper.data().num_zeros as RatingType;
            let tree_rating = tree_state.entry(t_index).rating;
            assert_eq!(expected_rating, tree_rating);
        });

        sts.update_ratings();
        assert_eq!(
            sts.tree[1],
            vec![
                TreeEntry {
                    rating: 0,
                    index: 0,
                    num_children: 3
                },
                TreeEntry {
                    rating: 2,
                    index: 1,
                    num_children: 3
                }
            ]
        );
    }
}
