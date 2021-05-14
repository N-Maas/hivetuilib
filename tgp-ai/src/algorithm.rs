use std::marker::PhantomData;

use tgp::{GameData, RevEffect};

use crate::{
    engine_stepper::EngineStepper,
    rater::{DecisionType, Rater},
    search_tree_state::SearchTreeState,
    Params, RatingType, INTERNAL_ERROR,
};

pub trait RateAndMap<T: GameData> {
    fn apply_type_mapping(&self, context: &T::Context) -> DecisionType;

    fn rate_moves(
        &self,
        rater: &mut Rater<T>,
        data: &T,
        old_context: &[(T::Context, usize)],
        player: usize,
    );

    fn rate_game_state(
        &self,
        data: &T,
        old_context: &[(T::Context, usize)],
        player: usize,
    ) -> RatingType;
}

pub struct MinMaxAlgorithm<T: GameData, R: RateAndMap<T>>
where
    T::EffectType: RevEffect<T>,
{
    params: Params,
    rate_and_map: R,
    tree: SearchTreeState,
    _t: PhantomData<T>,
}

impl<T: GameData, R: RateAndMap<T>> MinMaxAlgorithm<T, R>
where
    T::EffectType: RevEffect<T>,
{
    pub fn new(params: Params, rate_and_map: R) -> Self {
        Self {
            params,
            rate_and_map,
            tree: SearchTreeState::new(),
            _t: PhantomData,
        }
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
        let min = rater.current_max() - self.params.move_differences.probably_worse;
        let mut result = rater.cut_and_sort(min);
        debug_assert!(result[0].0 >= result.last().unwrap().0);
        if result.len() > self.params.move_limit {
            // TODO: Clustering
            result.truncate(self.params.move_limit);
        }

        let ratings = result.into_iter().map(|(_, index)| {
            stepper.forward_step(index);
            let result = self.min_max_rating(depth - 1, stepper, player);
            stepper.backward_step();
            result
        });
        let is_own_turn = current_player == player;
        if is_own_turn {
            ratings.max().expect(INTERNAL_ERROR)
        } else {
            ratings.min().expect(INTERNAL_ERROR)
        }
    }
}

#[cfg(test)]
mod test {
    use tgp::engine::Engine;

    use crate::{MinMaxAlgorithm, Params, engine_stepper::EngineStepper, test::{RateAndMapZeroOne, ZeroOneGame, type_mapping}};

    #[test]
    fn min_max_test() {
        let params = Params::new(4, 4, 2, 2);
        let alg = MinMaxAlgorithm::new(params, RateAndMapZeroOne);
        let data = ZeroOneGame::new(false, 6);
        let mut stepper = EngineStepper::new(Engine::new_logging(2, data), type_mapping);

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
}
