const INTERNAL_ERROR: &str = "Internal error in AI algorithm!";

type IndexType = u32;
// used by rating algorithm
pub type RatingType = i32;

mod algorithm;
mod engine_stepper;
mod params;
mod search_tree_state;

pub mod rater;

pub use algorithm::*;
pub use params::*;

#[cfg(test)]
pub(crate) mod test {
    use crate::{
        rater::{DecisionType, Rater},
        RateAndMap, RatingType,
    };
    use tgp::{plain_decision::PlainDecision, GameData, RevEffect};

    pub(crate) fn type_mapping(context: &ZeroOneContext) -> DecisionType {
        match context {
            ZeroOneContext::Flat | ZeroOneContext::ZeroAnd | ZeroOneContext::OneAnd => {
                DecisionType::BottomLevel
            }
            ZeroOneContext::Base => DecisionType::HigherLevel,
        }
    }

    /// A game where zeros and ones are counted and the state is represented by the sum of each.
    #[derive(Debug, Clone)]
    pub(crate) struct ZeroOneGame {
        pub num_zeros: u32,
        pub num_ones: u32,
        pub use_high_level: bool,
        pub player: usize,
        pub finished_at: u32,
    }

    impl ZeroOneGame {
        pub(crate) fn new(use_high_level: bool, finished_at: u32) -> Self {
            Self {
                num_zeros: 0,
                num_ones: 0,
                use_high_level,
                player: 0,
                finished_at,
            }
        }

        fn update(&mut self) {
            self.use_high_level = !self.use_high_level;
            self.player = (self.player + 1) % 2;
        }
    }

    fn chain<F, G, R>(f: F, g: G) -> impl Fn(&mut ZeroOneGame) -> R + Clone
    where
        F: Fn(&mut ZeroOneGame) -> R + Clone,
        G: Fn(&mut ZeroOneGame) -> R + Clone,
    {
        move |data| {
            f(data);
            g(data)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum ZeroOneContext {
        Flat,
        Base,
        ZeroAnd,
        OneAnd,
    }

    impl GameData for ZeroOneGame {
        type Context = ZeroOneContext;
        type EffectType = dyn RevEffect<Self>;

        fn next_decision(&self) -> Option<Box<dyn tgp::Decision<Self>>> {
            if self.num_zeros + self.num_ones >= self.finished_at {
                return None;
            }

            let player = self.player;
            let apply_zero = |data: &mut ZeroOneGame| {
                data.num_zeros += 1;
                None
            };
            let undo_zero = |data: &mut ZeroOneGame| {
                data.num_zeros -= 1;
            };
            let apply_one = |data: &mut ZeroOneGame| {
                data.num_ones += 1;
                None
            };
            let undo_one = |data: &mut ZeroOneGame| {
                data.num_ones -= 1;
            };
            let apply_update = |data: &mut ZeroOneGame| {
                data.update();
                None
            };
            let update = |data: &mut ZeroOneGame| {
                data.update();
            };

            if self.use_high_level {
                let mut dec = PlainDecision::with_context(player, ZeroOneContext::Base);
                dec.add_follow_up(move |_| {
                    let mut zero_dec = PlainDecision::with_context(player, ZeroOneContext::ZeroAnd);
                    zero_dec.add_rev_effect(
                        chain(chain(apply_zero, apply_zero), apply_update),
                        chain(chain(undo_zero, undo_zero), update),
                    );
                    zero_dec.add_rev_effect(
                        chain(chain(apply_zero, apply_one), apply_update),
                        chain(chain(undo_one, undo_zero), update),
                    );
                    zero_dec
                });
                dec.add_follow_up(move |_| {
                    let mut one_dec = PlainDecision::with_context(player, ZeroOneContext::OneAnd);
                    one_dec.add_rev_effect(
                        chain(chain(apply_one, apply_zero), apply_update),
                        chain(chain(undo_one, undo_zero), update),
                    );
                    one_dec.add_rev_effect(
                        chain(chain(apply_one, apply_one), apply_update),
                        chain(chain(undo_one, undo_one), update),
                    );
                    one_dec
                });
                Some(Box::new(dec))
            } else {
                let mut dec = PlainDecision::with_context(player, ZeroOneContext::Flat);
                dec.add_rev_effect(chain(apply_zero, apply_update), chain(undo_zero, update));
                dec.add_rev_effect(chain(apply_one, apply_update), chain(undo_one, update));
                Some(Box::new(dec))
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub(crate) struct RateAndMapZeroOne;

    impl RateAndMap<ZeroOneGame> for RateAndMapZeroOne {
        fn apply_type_mapping(&self, context: &ZeroOneContext) -> DecisionType {
            type_mapping(context)
        }

        fn rate_moves(
            &self,
            rater: &mut Rater<ZeroOneGame>,
            _data: &ZeroOneGame,
            _old_context: &[(ZeroOneContext, usize)],
            player: usize,
        ) {
            for i in 0..rater.num_decisions() {
                match rater.context(i) {
                    ZeroOneContext::Flat => {
                        if player == 0 {
                            rater.rate(i, 0, 1);
                            rater.rate(i, 1, 0);
                        } else {
                            rater.rate(i, 0, 0);
                            rater.rate(i, 1, 1);
                        }
                    }
                    ZeroOneContext::ZeroAnd => {
                        if player == 0 {
                            rater.rate(i, 0, 2);
                            rater.rate(i, 1, 0);
                        } else {
                            rater.rate(i, 0, -2);
                            rater.rate(i, 1, 0);
                        }
                    }
                    ZeroOneContext::OneAnd => {
                        if player == 0 {
                            rater.rate(i, 0, 0);
                            rater.rate(i, 1, -2);
                        } else {
                            rater.rate(i, 0, 0);
                            rater.rate(i, 1, 2);
                        }
                    }
                    ZeroOneContext::Base => unreachable!(),
                }
            }
        }

        fn rate_game_state(
            &self,
            data: &ZeroOneGame,
            _old_context: &[(ZeroOneContext, usize)],
            player: usize,
        ) -> RatingType {
            let mut diff = data.num_ones as i32 - data.num_zeros as i32;
            if player == 0 {
                diff = -diff;
            }
            diff * diff.abs()
        }
    }
}
