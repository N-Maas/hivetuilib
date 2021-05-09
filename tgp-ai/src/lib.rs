const INTERNAL_ERROR: &str = "Internal error in AI algorithm - impossible state";

type IndexType = u32;
type RatingType = i32;

mod engine_stepper;
mod params;
mod search_tree_state;

pub mod rater;

pub use params::*;

#[cfg(test)]
pub(crate) mod test {
    use tgp::{plain_decision::PlainDecision, GameData, RevEffect};

    /// A game where zeros and ones are counted and the state is represented by the sum of each.
    pub(crate) struct ZeroOneGame {
        num_zeros: u32,
        num_ones: u32,
        use_high_level: bool,
        player: usize,
        finished_at: u32,
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
                data.update();
                None
            };
            let undo_zero = |data: &mut ZeroOneGame| {
                data.num_zeros -= 1;
                data.update();
            };
            let apply_one = |data: &mut ZeroOneGame| {
                data.num_ones += 1;
                data.update();
                None
            };
            let undo_one = |data: &mut ZeroOneGame| {
                data.num_ones -= 1;
                data.update();
            };

            if self.use_high_level {
                let mut dec = PlainDecision::with_context(player, ZeroOneContext::Base);
                dec.add_follow_up(move |_| {
                    let mut zero_dec = PlainDecision::with_context(player, ZeroOneContext::ZeroAnd);
                    zero_dec.add_rev_effect(apply_zero, undo_zero);
                    zero_dec.add_rev_effect(apply_one, undo_one);
                    zero_dec
                });
                dec.add_follow_up(move |_| {
                    let mut one_dec = PlainDecision::with_context(player, ZeroOneContext::OneAnd);
                    one_dec.add_rev_effect(apply_zero, undo_zero);
                    one_dec.add_rev_effect(apply_one, undo_one);
                    one_dec
                });
                Some(Box::new(dec))
            } else {
                let mut dec = PlainDecision::with_context(player, ZeroOneContext::Flat);
                dec.add_rev_effect(apply_zero, undo_zero);
                dec.add_rev_effect(apply_one, undo_one);
                Some(Box::new(dec))
            }
        }
    }
}
