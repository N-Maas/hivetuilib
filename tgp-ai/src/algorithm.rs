use std::{cmp::Ordering, marker::PhantomData};

use tgp::{
    engine::{logging::EventLog, CloneError, Engine, EventListener, GameEngine},
    GameData, RevEffect,
};

use crate::{
    engine_stepper::EngineStepper,
    rater::{translate, DecisionType, Rater},
    search_tree_state::SearchTreeState,
    IndexType, Params, RatingType, INTERNAL_ERROR,
};

pub trait RateAndMap<T: GameData> {
    fn apply_type_mapping(&self, context: &T::Context) -> DecisionType;

    // TODO: player probably unnecessary
    fn rate_moves(
        &self,
        rater: &mut Rater<T>,
        data: &T,
        old_context: &[(T::Context, usize)],
        player: usize,
    );

    // TODO: player probably unnecessary
    fn rate_game_state(
        &self,
        data: &T,
        old_context: &[(T::Context, usize)],
        player: usize,
    ) -> RatingType;
}

// TODO: lift last part (i.e. only decide on subdecision)?
/// To apply the min-max algorithm, the engine must be in pending decision state
/// and the decision must be a top-level decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidEngineState {
    PendingEffect,
    FollowUp,
    Finished,
}

impl From<CloneError> for InvalidEngineState {
    fn from(e: CloneError) -> Self {
        match e {
            CloneError::PendingEffect => InvalidEngineState::PendingEffect,
            CloneError::FollowUp => InvalidEngineState::FollowUp,
        }
    }
}

pub struct MinMaxAlgorithm<T: GameData, R: RateAndMap<T>>
where
    T::EffectType: RevEffect<T>,
{
    params: Params,
    rate_and_map: R,
    _t: PhantomData<T>,
}

type RatingList = Vec<(RatingType, IndexType)>;

impl<T: GameData, R: RateAndMap<T>> MinMaxAlgorithm<T, R>
where
    T::EffectType: RevEffect<T>,
{
    pub fn new(params: Params, rate_and_map: R) -> Self {
        Self {
            params,
            rate_and_map,
            _t: PhantomData,
        }
    }

    pub fn apply<L>(&self, engine: &mut Engine<T, L>)
    where
        T: Clone,
        L: EventListener<T>,
    {
        let (_, index_list) = self.run(engine).expect("Invalid engine state!");
        for i in index_list {
            match engine.pull() {
                tgp::engine::GameState::PendingDecision(dec) => dec.select_option(i),
                _ => panic!("{}", INTERNAL_ERROR),
            }
        }
    }

    pub fn run<L>(
        &self,
        engine: &Engine<T, L>,
    ) -> Result<(RatingType, Vec<usize>), InvalidEngineState>
    where
        T: Clone,
        L: EventListener<T>,
    {
        self.params.integrity_check();

        if engine.is_finished() {
            return Err(InvalidEngineState::Finished);
        }
        let mut engine = engine.try_clone_with_listener(EventLog::new())?;
        let mut stepper = EngineStepper::new(&mut engine, |context| {
            self.rate_and_map.apply_type_mapping(context)
        });
        let player = stepper.player();

        // calculate move
        let mut tree = SearchTreeState::new();
        let num_runs = self
            .params
            .depth
            .saturating_sub(self.params.first_cut_delay_depth)
            + 1;
        for _ in 0..num_runs {
            self.extend_search_tree(&mut stepper, &mut tree, player);
            // TODO: prune
        }
        dbg!(&tree);
        let (rating, index) = dbg!(tree
            .root_moves()
            .max_by(|(r1, _), (r2, _)| r1.cmp(r2))
            .expect(INTERNAL_ERROR));

        // return result
        dbg!(tree.depth());
        Ok((
            rating,
            translate(
                &mut engine,
                |context| self.rate_and_map.apply_type_mapping(context),
                index,
            ),
        ))
    }

    fn extend_search_tree<M>(
        &self,
        stepper: &mut EngineStepper<T, M>,
        tree: &mut SearchTreeState,
        player: usize,
    ) where
        M: Fn(&T::Context) -> DecisionType,
    {
        assert!(tree.depth() < 2 * self.params.depth);
        let is_root = tree.depth() == 0;
        tree.new_levels();
        tree.for_each_leaf(stepper, |tree, stepper, t_index| {
            assert_eq!(
                stepper.player(),
                player,
                "Min-max algorithm requires alternating turns!"
            );
            let new_moves = if is_root {
                let depth =
                    self.params.first_cut_delay_depth + self.params.first_move_added_delay_depth;
                let branch_limit = f32::ceil(
                    self.params.limit_multiplier_first_move
                        * self.params.branch_limit_first_cut as f32,
                ) as usize;
                let move_limit = f32::ceil(
                    self.params.limit_multiplier_first_move * self.params.move_limit as f32,
                ) as usize;
                self.collect_recursive(
                    2 * depth,
                    stepper,
                    player,
                    depth,
                    self.params.branch_differences.surely_worse,
                    branch_limit,
                    self.params.move_differences.surely_worse,
                    move_limit,
                )
            } else {
                self.collect_recursive(
                    2 * self.params.first_cut_delay_depth,
                    stepper,
                    player,
                    self.params.first_cut_delay_depth,
                    self.params.branch_differences.probably_worse,
                    self.params.branch_limit_first_cut,
                    self.params.move_differences.probably_worse,
                    self.params.move_limit,
                )
            };
            for (rating, index, children) in new_moves {
                tree.push_child(t_index, rating, index, children);
            }
        });
        tree.extend();
        dbg!(&tree);
        tree.update_ratings();
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_recursive<M>(
        &self,
        depth: usize,
        stepper: &mut EngineStepper<T, M>,
        player: usize,
        delay_depth: usize,
        branch_diff: RatingType,
        branch_limit: usize,
        move_diff: RatingType,
        move_limit: usize,
    ) -> Vec<(RatingType, IndexType, RatingList)>
    where
        M: Fn(&T::Context) -> DecisionType,
    {
        if depth == 0 || stepper.is_finished() {
            return Vec::new();
        }

        let is_own_turn = stepper.player() == player;
        let compare = move |l: i32, r: i32| {
            if is_own_turn {
                r.cmp(&l)
            } else {
                l.cmp(&r)
            }
        };

        // collect moves and calculate min-max ratings
        let moves = self.create_move_ratings(
            stepper,
            move_diff,
            move_limit,
            Rater::cut_and_sort_with_equivalency,
        );
        let mut moves = moves
            .into_iter()
            .map(|(_, index, eq)| {
                stepper.forward_step(index);
                let (rating, children) = self.collect_and_cut(depth - 1, stepper, player);
                stepper.backward_step();
                (rating, index, eq, children)
            })
            .collect::<Vec<_>>();

        // cut the moves to the defined limit
        if depth >= 2 * delay_depth {
            moves.sort_unstable_by(|&(r1, _, _, _), &(r2, _, _, _)| compare(r1, r2));
            let min = moves.first().unwrap().0;
            moves.retain(|(rating, _, _, _)| RatingType::abs(*rating - min) <= branch_diff);
            if moves.len() > branch_limit {
                // TODO: Clustering
                moves.truncate(branch_limit);
            }
        }

        // resolve equivalency classes
        moves
            .into_iter()
            .map(|(mut rating, mut index, equivalent_moves, mut children)| {
                for m_idx in equivalent_moves {
                    stepper.forward_step(m_idx);
                    let (m_rating, m_children) = self.collect_and_cut(depth - 1, stepper, player);
                    if compare(m_rating, rating) == Ordering::Less {
                        rating = m_rating;
                        index = m_idx;
                        children = m_children;
                    }
                    stepper.backward_step();
                }
                (rating, index, children)
            })
            .collect()
    }

    fn collect_and_cut<M>(
        &self,
        depth: usize,
        stepper: &mut EngineStepper<T, M>,
        player: usize,
    ) -> (RatingType, RatingList)
    where
        M: Fn(&T::Context) -> DecisionType,
    {
        if depth == 0 || stepper.is_finished() {
            return (
                self.rate_and_map.rate_game_state(
                    stepper.data(),
                    stepper.decision_context(),
                    player,
                ),
                Vec::new(),
            );
        }

        let is_own_turn = stepper.player() == player;
        let compare = move |l: i32, r: i32| {
            if is_own_turn {
                r.cmp(&l)
            } else {
                l.cmp(&r)
            }
        };

        // collect moves and calculate min-max ratings
        let mut moves = self.create_move_ratings(
            stepper,
            self.params.move_differences.probably_worse,
            self.params.move_limit,
            Rater::cut_and_sort_with_equivalency,
        );
        for (rating, index, _) in moves.iter_mut() {
            stepper.forward_step(*index);
            *rating = self.min_max_rating(depth - 1, stepper, player);
            stepper.backward_step();
        }

        // cut the moves to the defined limit
        if depth >= (2 * self.params.first_cut_delay_depth - 1) {
            moves.sort_unstable_by(|&(r1, _, _), &(r2, _, _)| compare(r1, r2));
            let min = moves.first().unwrap().0;
            moves.retain(|(rating, _, _)| {
                RatingType::abs(*rating - min) <= self.params.branch_differences.probably_worse
            });
            if moves.len() > self.params.branch_limit_first_cut {
                // TODO: Clustering
                moves.truncate(self.params.branch_limit_first_cut);
            }
        }

        // resolve equivalency classes
        let result = moves
            .into_iter()
            .map(|(mut rating, mut index, equivalent_moves)| {
                for m_idx in equivalent_moves {
                    stepper.forward_step(m_idx);
                    let m_rating = self.min_max_rating(depth - 1, stepper, player);
                    if compare(m_rating, rating) == Ordering::Less {
                        rating = m_rating;
                        index = m_idx;
                    }
                    stepper.backward_step();
                }
                (rating, index)
            })
            .collect::<Vec<_>>();

        // calculate result
        let min = result
            .iter()
            .min_by(|&&(r1, _), &&(r2, _)| compare(r1, r2))
            .unwrap()
            .0;
        (min, result)
    }

    fn min_max_rating<M>(
        &self,
        depth: usize,
        stepper: &mut EngineStepper<T, M>,
        player: usize,
    ) -> RatingType
    where
        M: Fn(&T::Context) -> DecisionType,
    {
        if depth == 0 || stepper.is_finished() {
            return self.rate_and_map.rate_game_state(
                stepper.data(),
                stepper.decision_context(),
                player,
            );
        }

        let is_own_turn = stepper.player() == player;
        let moves = self.create_move_ratings(
            stepper,
            self.params.move_differences.probably_worse,
            self.params.move_limit,
            Rater::cut_and_sort,
        );
        let ratings = moves.into_iter().map(|(_, index)| {
            stepper.forward_step(index);
            let result = self.min_max_rating(depth - 1, stepper, player);
            stepper.backward_step();
            result
        });
        if is_own_turn {
            ratings.max().expect(INTERNAL_ERROR)
        } else {
            ratings.min().expect(INTERNAL_ERROR)
        }
    }

    #[inline]
    fn create_move_ratings<M, E: Ord>(
        &self,
        stepper: &mut EngineStepper<T, M>,
        move_difference: RatingType,
        move_limit: usize,
        rater_fn: fn(Rater<T>, RatingType) -> Vec<E>,
    ) -> Vec<E>
    where
        M: Fn(&T::Context) -> DecisionType,
    {
        let current_player = stepper.player();
        let mut rater = Rater::new(stepper.engine(), |context| {
            self.rate_and_map.apply_type_mapping(context)
        });
        self.rate_and_map.rate_moves(
            &mut rater,
            stepper.data(),
            stepper.decision_context(),
            current_player,
        );
        let min = rater.current_max() - move_difference;
        let mut result = rater_fn(rater, min);
        debug_assert!(result.first().unwrap() >= result.last().unwrap());
        if result.len() > self.params.move_limit {
            // TODO: Clustering
            result.truncate(move_limit);
        }
        result
    }
}

#[cfg(test)]
mod test {
    use tgp::engine::{Engine, GameEngine, GameState};

    use crate::{
        engine_stepper::EngineStepper,
        test::{type_mapping, RateAndMapZeroOne, ZeroOneGame},
        MinMaxAlgorithm, Params,
    };

    #[test]
    fn min_max_test() {
        let params = Params::new(4, 4, 2, 2);
        let alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        let data = ZeroOneGame::new(false, 6);
        let mut engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(&mut engine, type_mapping);

        assert_eq!(alg.min_max_rating(0, &mut stepper, 0), 0);
        assert_eq!(alg.min_max_rating(0, &mut stepper, 1), 0);
        assert_eq!(alg.min_max_rating(1, &mut stepper, 0), 1);
        assert_eq!(alg.min_max_rating(1, &mut stepper, 1), -1);
        assert_eq!(alg.min_max_rating(2, &mut stepper, 0), -1);
        assert_eq!(alg.min_max_rating(2, &mut stepper, 1), 1);
        assert_eq!(alg.min_max_rating(3, &mut stepper, 0), 0);
        assert_eq!(alg.min_max_rating(3, &mut stepper, 1), 0);
        assert_eq!(alg.min_max_rating(4, &mut stepper, 0), -4);
        assert_eq!(alg.min_max_rating(4, &mut stepper, 1), 4);
        assert_eq!(alg.min_max_rating(6, &mut stepper, 0), -4);
        assert_eq!(alg.min_max_rating(6, &mut stepper, 1), 4);
    }

    #[test]
    fn collect_and_cut_test() {
        let params = Params::new(4, 4, 2, 2);
        let mut alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        let data = ZeroOneGame::new(false, 6);
        let mut engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(&mut engine, type_mapping);

        assert_eq!(alg.collect_and_cut(0, &mut stepper, 0), (0, Vec::new()));
        assert_eq!(alg.collect_and_cut(0, &mut stepper, 1), (0, Vec::new()));
        assert_eq!(
            alg.collect_and_cut(1, &mut stepper, 0),
            (1, vec![(1, 0), (-1, 1)])
        );
        assert_eq!(
            alg.collect_and_cut(1, &mut stepper, 1),
            (-1, vec![(-1, 0), (1, 1)])
        );
        assert_eq!(
            alg.collect_and_cut(2, &mut stepper, 0),
            (-1, vec![(-1, 0), (-9, 1)])
        );
        assert_eq!(
            alg.collect_and_cut(2, &mut stepper, 1),
            (1, vec![(1, 0), (9, 1)])
        );
        alg.params.first_cut_delay_depth = 1;
        assert_eq!(alg.collect_and_cut(2, &mut stepper, 0), (-1, vec![(-1, 0)]));
        assert_eq!(alg.collect_and_cut(2, &mut stepper, 1), (1, vec![(1, 0)]));
    }

    #[test]
    fn collect_recursive_test() {
        let params = Params::new(4, 4, 2, 2);
        let alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        let data = ZeroOneGame::new(false, 6);
        let mut engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(&mut engine, type_mapping);

        assert_eq!(
            alg.collect_recursive(0, &mut stepper, 0, 2, 2, 2, 2, 4),
            Vec::new()
        );
        assert_eq!(
            alg.collect_recursive(0, &mut stepper, 1, 2, 2, 2, 2, 4),
            Vec::new()
        );
        assert_eq!(
            alg.collect_recursive(1, &mut stepper, 0, 2, 2, 2, 2, 4),
            vec![(1, 0, Vec::new()), (-1, 1, Vec::new())]
        );
        assert_eq!(
            alg.collect_recursive(1, &mut stepper, 1, 2, 2, 2, 2, 4),
            vec![(-1, 0, Vec::new()), (1, 1, Vec::new())]
        );
        assert_eq!(
            alg.collect_recursive(2, &mut stepper, 0, 2, 2, 2, 2, 4),
            vec![
                (-1, 0, vec![(-1, 3), (1, 1), (1, 2)]),
                (-9, 1, vec![(-9, 3), (-1, 1), (-1, 2)])
            ]
        );
        assert_eq!(
            alg.collect_recursive(2, &mut stepper, 1, 2, 2, 2, 2, 4),
            vec![
                (1, 0, vec![(1, 3), (-1, 1), (-1, 2)]),
                (9, 1, vec![(9, 3), (1, 1), (1, 2)])
            ]
        );
        assert_eq!(
            alg.collect_recursive(2, &mut stepper, 0, 1, 2, 2, 2, 4),
            vec![(-1, 0, vec![(-1, 3), (1, 1), (1, 2)])]
        );
        assert_eq!(
            alg.collect_recursive(2, &mut stepper, 1, 1, 2, 2, 2, 4),
            vec![(1, 0, vec![(1, 3), (-1, 1), (-1, 2)])]
        );
    }

    #[test]
    fn run_test() {
        let params = Params::new(1, 4, 2, 2);
        let mut alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        alg.params.first_cut_delay_depth = 1;
        let data = ZeroOneGame::new(true, 8);
        let mut engine = Engine::new_logging(2, data);
        assert_eq!(alg.run(&engine), Ok((1, vec![0, 0])));

        alg.params.depth = 2;
        alg.params.first_cut_delay_depth = 2;
        assert_eq!(alg.run(&engine), Ok((4, vec![0, 0])));

        match engine.pull() {
            GameState::PendingDecision(dec) => dec.select_option(0),
            _ => unreachable!(),
        }
        match engine.pull() {
            GameState::PendingDecision(dec) => dec.select_option(1),
            _ => unreachable!(),
        }
        match engine.pull() {
            GameState::PendingEffect(eff) => eff.all_effects(),
            _ => unreachable!(),
        }
        assert_eq!(alg.run(&engine), Ok((-4, vec![1])));
    }
}
