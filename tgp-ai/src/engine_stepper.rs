use std::{convert::TryFrom, fmt::Debug};

use tgp::{
    engine::{logging::EventLog, Engine, GameEngine},
    GameData, RevEffect,
};

use crate::{
    rater::{for_each_decision_flat, DecisionType},
    IndexType, INTERNAL_ERROR,
};

pub(crate) struct EngineStepper<T: GameData, F>
where
    T::EffectType: RevEffect<T>,
    F: Fn(&T::Context) -> DecisionType,
{
    engine: Engine<T, EventLog<T>>,
    type_mapping: F,
}

impl<T: GameData + Debug, F> Debug for EngineStepper<T, F>
where
    T::EffectType: RevEffect<T>,
    F: Fn(&T::Context) -> DecisionType,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EngineStepper {{ engine: {:?} }}", self.engine)
    }
}

impl<T: GameData, F> EngineStepper<T, F>
where
    T::EffectType: RevEffect<T>,
    F: Fn(&T::Context) -> DecisionType,
{
    pub fn new(engine: Engine<T, EventLog<T>>, type_mapping: F) -> Self {
        Self {
            engine,
            type_mapping,
        }
    }

    pub fn forward_step(&mut self, index: IndexType) {
        let mut current_index = usize::try_from(index).unwrap();
        for_each_decision_flat(&mut self.engine, &self.type_mapping, |dec, _| {
            if current_index < dec.option_count() {
                true
            } else {
                current_index -= dec.option_count();
                false
            }
        });
        match self.engine.pull() {
            tgp::engine::GameState::PendingDecision(dec) => {
                assert!(current_index < dec.option_count(), "{}", INTERNAL_ERROR);
                dec.select_option(current_index);
            }
            _ => panic!("{}", INTERNAL_ERROR),
        }
        match self.engine.pull() {
            tgp::engine::GameState::PendingEffect(eff) => {
                eff.all_effects();
            }
            _ => panic!("{}", INTERNAL_ERROR),
        }
    }

    pub fn backward_step(&mut self) {
        if !self.engine.undo_last_decision() {
            panic!("{}", INTERNAL_ERROR)
        }
    }

    pub fn is_finished(&self) -> bool {
        self.engine.is_finished()
    }

    pub fn data(&self) -> &T {
        self.engine.data()
    }

    // TODO: probably shouldn't be mut
    pub fn player(&mut self) -> usize {
        match self.engine.pull() {
            tgp::engine::GameState::PendingDecision(dec) => dec.player(),
            _ => panic!("{}", INTERNAL_ERROR),
        }
    }
}

#[cfg(test)]
mod test {
    use tgp::engine::Engine;

    use crate::{
        engine_stepper::EngineStepper,
        test::{type_mapping, ZeroOneGame},
    };

    #[test]
    fn stepping_test() {
        let data = ZeroOneGame::new(false, 3);
        let engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(engine, type_mapping);
        assert!(!stepper.is_finished());

        stepper.forward_step(0);
        assert!(!stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 1);
        assert_eq!(stepper.data().num_ones, 0);

        stepper.backward_step();
        assert!(!stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 0);
        assert_eq!(stepper.data().num_ones, 0);

        stepper.forward_step(1);
        stepper.forward_step(1);
        assert!(stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 1);
        assert_eq!(stepper.data().num_ones, 2);

        stepper.backward_step();
        assert!(!stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 0);
        assert_eq!(stepper.data().num_ones, 1);

        stepper.forward_step(3);
        assert!(stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 0);
        assert_eq!(stepper.data().num_ones, 3);
    }
}
