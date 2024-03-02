use std::{cmp::Ordering, convert::TryFrom, fmt::Debug, marker::PhantomData, ops::ControlFlow};

use tgp::{
    engine::{logging::EventLog, CloneError, Engine, EventListener, GameEngine},
    GameData, RevEffect,
};

use crate::{
    engine_stepper::EngineStepper,
    rater::{DecisionType, Rater},
    search_tree_state::SearchTreeState,
    IndexType, Params, RatingType, Sliding, INTERNAL_ERROR,
};

pub trait RateAndMap<T: GameData> {
    fn apply_type_mapping(&self, context: &T::Context) -> DecisionType;

    fn rate_moves(
        &self,
        rater: &mut Rater,
        context: &[T::Context],
        data: &T,
        old_context: &[(T::Context, usize)],
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinMaxError {
    PendingEffect,
    FollowUp,
    Finished,
    Cancelled,
}

impl MinMaxError {
    pub fn into_engine_state_error(self) -> Option<InvalidEngineState> {
        match self {
            MinMaxError::PendingEffect => Some(InvalidEngineState::PendingEffect),
            MinMaxError::FollowUp => Some(InvalidEngineState::FollowUp),
            MinMaxError::Finished => Some(InvalidEngineState::Finished),
            MinMaxError::Cancelled => None,
        }
    }
}

impl From<CloneError> for MinMaxError {
    fn from(e: CloneError) -> Self {
        match e {
            CloneError::PendingEffect => MinMaxError::PendingEffect,
            CloneError::FollowUp => MinMaxError::FollowUp,
        }
    }
}

pub struct MinMaxAlgorithm<T: GameData + Debug, R: RateAndMap<T>>
where
    T::EffectType: RevEffect<T>,
{
    params: Params,
    rate_and_map: R,
    _t: PhantomData<T>,
}

type RatingList = Vec<(RatingType, Box<[IndexType]>)>;

impl<T: GameData + Debug, R: RateAndMap<T>> MinMaxAlgorithm<T, R>
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
        for &i in index_list.iter() {
            match engine.pull() {
                tgp::engine::GameState::PendingDecision(dec) => dec.select_option(i),
                _ => panic!("{}", INTERNAL_ERROR),
            }
        }
    }

    pub fn run<L>(
        &self,
        engine: &Engine<T, L>,
    ) -> Result<(RatingType, Box<[usize]>), InvalidEngineState>
    where
        T: Clone,
        L: EventListener<T>,
    {
        self.run_with_cancellation(engine, || false)
            .map_err(|e| e.into_engine_state_error().unwrap())
    }

    pub fn run_with_cancellation<L, F>(
        &self,
        engine: &Engine<T, L>,
        should_cancel: F,
    ) -> Result<(RatingType, Box<[usize]>), MinMaxError>
    where
        T: Clone,
        L: EventListener<T>,
        F: Fn() -> bool,
    {
        let ratings = self.run_all_ratings_with_cancellation(&engine, should_cancel)?;
        let (rating, indizes) = ratings
            .into_iter()
            .max_by(|(r1, _), (r2, _)| r1.cmp(r2))
            .expect(INTERNAL_ERROR);

        // return result
        Ok((rating, indizes))
    }

    pub fn run_all_ratings<L>(
        &self,
        engine: &Engine<T, L>,
    ) -> Result<Vec<(RatingType, Box<[usize]>)>, InvalidEngineState>
    where
        T: Clone,
        L: EventListener<T>,
    {
        self.run_all_ratings_with_cancellation(engine, || false)
            .map_err(|e| e.into_engine_state_error().unwrap())
    }

    pub fn run_all_ratings_with_cancellation<L, F>(
        &self,
        engine: &Engine<T, L>,
        should_cancel: F,
    ) -> Result<Vec<(RatingType, Box<[usize]>)>, MinMaxError>
    where
        T: Clone,
        L: EventListener<T>,
        F: Fn() -> bool,
    {
        if engine.is_finished() {
            return Err(MinMaxError::Finished);
        }
        let mut engine = engine.try_clone_with_listener(EventLog::new())?;

        self.params.integrity_check();
        let mut stepper = EngineStepper::new(&mut engine);
        let player = stepper.player();

        // calculate move
        let mut tree = SearchTreeState::new();
        let num_runs = self
            .params
            .depth
            .saturating_sub(self.params.first_cut_delay_depth)
            + 1;
        for _ in 0..num_runs {
            self.extend_search_tree(&mut stepper, &mut tree, player, &should_cancel)?;
            if tree.root_moves().count() == 1 {
                break;
            }
            // TODO: prune
        }
        Ok(tree
            .root_moves()
            .map(|(val, path)| {
                (
                    val,
                    path.iter()
                        .map(|&val| usize::try_from(val).unwrap())
                        .collect::<Box<_>>(),
                )
            })
            .collect::<Vec<_>>())
    }

    fn extend_search_tree<F: Fn() -> bool>(
        &self,
        stepper: &mut EngineStepper<T>,
        tree: &mut SearchTreeState,
        player: usize,
        should_cancel: F,
    ) -> Result<(), MinMaxError> {
        assert!(tree.depth() < 2 * self.params.depth);
        let depth = {
            let mut depth = self.params.first_cut_delay_depth;
            if tree.depth() == 0 {
                depth += self.params.first_move_added_delay_depth;
            }
            2 * usize::min(depth, self.params.depth)
        };
        let sliding = self.params.sliding.get(tree.depth()..);
        tree.new_levels();
        tree.for_each_leaf(stepper, |tree, stepper, t_index| {
            assert!(
                stepper.is_finished() || stepper.player() == player,
                "Min-max algorithm requires alternating turns!"
            );
            let new_moves = self.collect_recursive(
                depth,
                stepper,
                player,
                self.params.first_cut_delay_depth,
                sliding,
            );
            for (rating, index, children) in new_moves {
                tree.push_child(t_index, rating, index, children);
            }
            if should_cancel() {
                ControlFlow::Break(MinMaxError::Cancelled)
            } else {
                ControlFlow::Continue(())
            }
        })?;
        // TODO: end of game handling
        tree.extend();
        tree.update_ratings();
        Ok(())
    }

    fn collect_recursive(
        &self,
        depth: usize,
        stepper: &mut EngineStepper<T>,
        player: usize,
        delay_depth: usize,
        sliding: Sliding,
    ) -> Vec<(RatingType, Box<[IndexType]>, RatingList)> {
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
            sliding.move_cut_difference(),
            sliding.move_limit(),
            Rater::cut_and_sort_with_equivalency,
        );
        let mut moves = moves
            .into_iter()
            .map(|(_, indizes, eq)| {
                stepper.forward_step(&indizes);
                let (rating, children) =
                    self.collect_and_cut(depth - 1, stepper, player, sliding.next());
                stepper.backward_step();
                (rating, indizes, eq, children)
            })
            .collect::<Vec<_>>();

        // cut the moves to the defined limit
        if depth >= 2 * delay_depth {
            moves.sort_unstable_by(|&(r1, _, _, _), &(r2, _, _, _)| compare(r1, r2));
            let min = moves.first().unwrap().0;
            moves.retain(|(rating, _, _, _)| {
                RatingType::abs(*rating - min) <= sliding.branch_cut_difference()
            });
            if moves.len() > sliding.branch_cut_limit() {
                // TODO: Clustering
                moves.truncate(sliding.branch_cut_limit());
            }
        }

        // resolve equivalency classes
        moves
            .into_iter()
            .map(
                |(mut rating, mut indizes, equivalent_moves, mut children)| {
                    for m_idz in equivalent_moves
                        .into_iter()
                        .take(sliding.equivalency_class_limit())
                    {
                        stepper.forward_step(&m_idz);
                        let (m_rating, m_children) =
                            self.collect_and_cut(depth - 1, stepper, player, sliding.next());
                        if compare(m_rating, rating) == Ordering::Less {
                            rating = m_rating;
                            indizes = m_idz;
                            children = m_children;
                        }
                        stepper.backward_step();
                    }
                    (rating, indizes, children)
                },
            )
            .collect()
    }

    fn collect_and_cut(
        &self,
        depth: usize,
        stepper: &mut EngineStepper<T>,
        player: usize,
        sliding: Sliding,
    ) -> (RatingType, RatingList) {
        if depth == 0 || stepper.is_finished() {
            return (self.final_rating(depth, stepper, player), Vec::new());
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
            sliding.move_cut_difference(),
            sliding.move_limit(),
            Rater::cut_and_sort_with_equivalency,
        );
        for (rating, indizes, _) in moves.iter_mut() {
            stepper.forward_step(&indizes);
            *rating = self.min_max_rating(depth - 1, stepper, player, sliding.next());
            stepper.backward_step();
        }

        // cut the moves to the defined limit
        if depth >= (2 * self.params.first_cut_delay_depth - 1) {
            moves.sort_unstable_by(|&(r1, _, _), &(r2, _, _)| compare(r1, r2));
            let min = moves.first().unwrap().0;
            moves.retain(|(rating, _, _)| {
                RatingType::abs(*rating - min) <= sliding.branch_cut_difference()
            });
            if moves.len() > sliding.branch_cut_limit() {
                // TODO: Clustering
                moves.truncate(sliding.branch_cut_limit());
            }
        }

        // resolve equivalency classes
        let result = moves
            .into_iter()
            .map(|(mut rating, mut index, equivalent_moves)| {
                for m_idzs in equivalent_moves
                    .into_iter()
                    .take(sliding.equivalency_class_limit())
                {
                    stepper.forward_step(&m_idzs);
                    let m_rating = self.min_max_rating(depth - 1, stepper, player, sliding.next());
                    if compare(m_rating, rating) == Ordering::Less {
                        rating = m_rating;
                        index = m_idzs;
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

    fn min_max_rating(
        &self,
        depth: usize,
        stepper: &mut EngineStepper<T>,
        player: usize,
        sliding: Sliding,
    ) -> RatingType {
        if depth == 0 || stepper.is_finished() {
            return self.final_rating(depth, stepper, player);
        }

        let is_own_turn = stepper.player() == player;
        let moves = self.create_move_ratings(
            stepper,
            sliding.move_cut_difference(),
            sliding.move_limit(),
            Rater::cut_and_sort,
        );
        let ratings = moves.into_iter().map(|(_, indizes)| {
            stepper.forward_step(&indizes);
            let result = self.min_max_rating(depth - 1, stepper, player, sliding.next());
            stepper.backward_step();
            result
        });
        if is_own_turn {
            ratings.max().expect(INTERNAL_ERROR)
        } else {
            ratings.min().expect(INTERNAL_ERROR)
        }
    }

    fn final_rating(
        &self,
        depth: usize,
        stepper: &mut EngineStepper<T>,
        player: usize,
    ) -> RatingType {
        let val =
            self.rate_and_map
                .rate_game_state(stepper.data(), stepper.decision_context(), player);
        if stepper.is_finished() {
            RatingType::try_from(depth + 1).unwrap() * val
        } else {
            val
        }
    }

    #[inline]
    fn create_move_ratings<E: Ord + Debug>(
        &self,
        stepper: &mut EngineStepper<T>,
        move_cut_difference: RatingType,
        move_limit: usize,
        rater_fn: fn(Rater, RatingType) -> Vec<E>,
    ) -> Vec<E> {
        let (mut rater, context) = Rater::new(stepper.engine(), |context| {
            self.rate_and_map.apply_type_mapping(context)
        });
        self.rate_and_map.rate_moves(
            &mut rater,
            &context,
            stepper.data(),
            stepper.decision_context(),
        );
        let min = rater.current_max() - move_cut_difference;
        let mut result = rater_fn(rater, min);
        assert!(!result.is_empty());
        if result.len() > move_limit {
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
        test::{RateAndMapZeroOne, ZeroOneGame},
        IndexType, MinMaxAlgorithm, Params, SlidingParams,
    };

    fn indizes(input: &[IndexType]) -> Box<[IndexType]> {
        Box::from(input)
    }

    #[test]
    fn min_max_test() {
        let sliding = SlidingParams::with_defaults(4, 2, 4, 4, 2, 2, 4);
        let params = Params::new(4, sliding.clone());
        let alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        let data = ZeroOneGame::new(false, 6);
        let mut engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(&mut engine);

        assert_eq!(alg.min_max_rating(0, &mut stepper, 0, sliding.get(1..)), 0);
        assert_eq!(alg.min_max_rating(0, &mut stepper, 1, sliding.get(1..)), 0);
        assert_eq!(alg.min_max_rating(1, &mut stepper, 0, sliding.get(1..)), 1);
        assert_eq!(alg.min_max_rating(1, &mut stepper, 1, sliding.get(1..)), -1);
        assert_eq!(alg.min_max_rating(2, &mut stepper, 0, sliding.get(1..)), -1);
        assert_eq!(alg.min_max_rating(2, &mut stepper, 1, sliding.get(1..)), 1);
        assert_eq!(alg.min_max_rating(3, &mut stepper, 0, sliding.get(1..)), 0);
        assert_eq!(alg.min_max_rating(3, &mut stepper, 1, sliding.get(1..)), 0);
        assert_eq!(alg.min_max_rating(4, &mut stepper, 0, sliding.get(1..)), -4);
        assert_eq!(alg.min_max_rating(4, &mut stepper, 1, sliding.get(1..)), 4);
        assert_eq!(
            alg.min_max_rating(6, &mut stepper, 0, sliding.get(1..)),
            -12
        );
        assert_eq!(alg.min_max_rating(6, &mut stepper, 1, sliding.get(1..)), 12);
    }

    #[test]
    fn collect_and_cut_test() {
        let sliding = SlidingParams::with_defaults(4, 2, 4, 4, 2, 2, 4);
        let params = Params::new(4, sliding.clone());
        let mut alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        let data = ZeroOneGame::new(false, 6);
        let mut engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(&mut engine);

        assert_eq!(
            alg.collect_and_cut(0, &mut stepper, 0, sliding.get(1..)),
            (0, Vec::new())
        );
        assert_eq!(
            alg.collect_and_cut(0, &mut stepper, 1, sliding.get(1..)),
            (0, Vec::new())
        );
        assert_eq!(
            alg.collect_and_cut(1, &mut stepper, 0, sliding.get(1..)),
            (1, vec![(1, indizes(&[0])), (-1, indizes(&[1]))])
        );
        assert_eq!(
            alg.collect_and_cut(1, &mut stepper, 1, sliding.get(1..)),
            (-1, vec![(-1, indizes(&[0])), (1, indizes(&[1]))])
        );
        assert_eq!(
            alg.collect_and_cut(2, &mut stepper, 0, sliding.get(1..)),
            (-1, vec![(-1, indizes(&[0])), (-9, indizes(&[1]))])
        );
        assert_eq!(
            alg.collect_and_cut(2, &mut stepper, 1, sliding.get(1..)),
            (1, vec![(1, indizes(&[0])), (9, indizes(&[1]))])
        );
        alg.params.first_cut_delay_depth = 1;
        assert_eq!(
            alg.collect_and_cut(2, &mut stepper, 0, sliding.get(1..)),
            (-1, vec![(-1, indizes(&[0]))])
        );
        assert_eq!(
            alg.collect_and_cut(2, &mut stepper, 1, sliding.get(1..)),
            (1, vec![(1, indizes(&[0]))])
        );
    }

    #[test]
    fn collect_recursive_test() {
        let sliding = SlidingParams::with_defaults(4, 2, 4, 4, 2, 2, 4);
        let params = Params::new(4, sliding.clone());
        let alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        let data = ZeroOneGame::new(false, 6);
        let mut engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(&mut engine);

        assert_eq!(
            alg.collect_recursive(0, &mut stepper, 0, 2, sliding.get(1..)),
            Vec::new()
        );
        assert_eq!(
            alg.collect_recursive(0, &mut stepper, 1, 2, sliding.get(1..)),
            Vec::new()
        );
        assert_eq!(
            alg.collect_recursive(1, &mut stepper, 0, 2, sliding.get(1..)),
            vec![
                (1, indizes(&[0]), Vec::new()),
                (-1, indizes(&[1]), Vec::new())
            ]
        );
        assert_eq!(
            alg.collect_recursive(1, &mut stepper, 1, 2, sliding.get(1..)),
            vec![
                (-1, indizes(&[0]), Vec::new()),
                (1, indizes(&[1]), Vec::new())
            ]
        );
        assert_eq!(
            alg.collect_recursive(2, &mut stepper, 0, 2, sliding.get(1..)),
            vec![
                (
                    -1,
                    indizes(&[0]),
                    vec![
                        (-1, indizes(&[1, 1])),
                        (1, indizes(&[0, 1])),
                        (1, indizes(&[1, 0]))
                    ]
                ),
                (
                    -9,
                    indizes(&[1]),
                    vec![
                        (-9, indizes(&[1, 1])),
                        (-1, indizes(&[0, 1])),
                        (-1, indizes(&[1, 0]))
                    ]
                )
            ]
        );
        assert_eq!(
            alg.collect_recursive(2, &mut stepper, 1, 2, sliding.get(1..)),
            vec![
                (
                    1,
                    indizes(&[0]),
                    vec![
                        (1, indizes(&[1, 1])),
                        (-1, indizes(&[0, 1])),
                        (-1, indizes(&[1, 0]))
                    ]
                ),
                (
                    9,
                    indizes(&[1]),
                    vec![
                        (9, indizes(&[1, 1])),
                        (1, indizes(&[0, 1])),
                        (1, indizes(&[1, 0]))
                    ]
                )
            ]
        );
        assert_eq!(
            alg.collect_recursive(2, &mut stepper, 0, 1, sliding.get(1..)),
            vec![(
                -1,
                indizes(&[0]),
                vec![
                    (-1, indizes(&[1, 1])),
                    (1, indizes(&[0, 1])),
                    (1, indizes(&[1, 0]))
                ]
            )]
        );
        assert_eq!(
            alg.collect_recursive(2, &mut stepper, 1, 1, sliding.get(1..)),
            vec![(
                1,
                indizes(&[0]),
                vec![
                    (1, indizes(&[1, 1])),
                    (-1, indizes(&[0, 1])),
                    (-1, indizes(&[1, 0]))
                ]
            )]
        );
    }

    #[test]
    fn run_test() {
        let sliding = SlidingParams::with_defaults(1, 2, 4, 4, 2, 2, 4);
        let params = Params::new(1, sliding.clone());
        let mut alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        alg.params.first_cut_delay_depth = 1;
        let data = ZeroOneGame::new(true, 8);
        let mut engine = Engine::new_logging(2, data);
        assert_eq!(alg.run(&engine), Ok((1, Box::from([0, 0]))));

        let sliding = SlidingParams::with_defaults(2, 1, 4, 4, 4, 2, 4);
        let params = Params::new(2, sliding.clone());
        let mut alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        alg.params.first_cut_delay_depth = 1;
        assert_eq!(alg.run(&engine), Ok((4, Box::from([0, 0]))));

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
        assert_eq!(alg.run(&engine), Ok((-4, Box::from([1]))));
    }
}
