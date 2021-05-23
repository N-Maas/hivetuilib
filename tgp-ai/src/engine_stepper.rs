use std::{
    convert::TryFrom,
    fmt::{self, Debug},
};

use tgp::{
    engine::{logging::EventLog, Engine, GameEngine, GameState},
    GameData, RevEffect,
};

use crate::{IndexType, INTERNAL_ERROR};

// TODO: abstract over multiple decisions by same player? --> is probably hard
pub(crate) struct EngineStepper<'a, T: GameData + Debug>
where
    T::EffectType: RevEffect<T>,
{
    engine: &'a mut Engine<T, EventLog<T>>,
    decision_context: Vec<(T::Context, usize)>,
}

impl<T: GameData + Debug> Debug for EngineStepper<'_, T>
where
    T::EffectType: RevEffect<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EngineStepper {{ engine: {:#?} }}", self.engine)
    }
}

impl<'a, T: GameData + Debug> EngineStepper<'a, T>
where
    T::EffectType: RevEffect<T>,
{
    pub fn new(engine: &'a mut Engine<T, EventLog<T>>) -> Self {
        Self {
            engine,
            decision_context: Vec::new(),
        }
    }

    pub fn forward_step(&mut self, indizes: &[IndexType]) {
        let mut chosen_context = None;
        for (i, &index) in indizes.iter().enumerate() {
            let index = usize::try_from(index).unwrap();
            match self.engine.pull() {
                GameState::PendingDecision(dec) => {
                    if i + 1 == indizes.len() {
                        chosen_context = Some((dec.context(), index));
                    }
                    assert!(
                        index < dec.option_count(),
                        "{} - indizes={:?}, option_count={}",
                        INTERNAL_ERROR,
                        indizes,
                        dec.option_count()
                    );
                    dec.select_option(index);
                }
                _ => panic!("{}", INTERNAL_ERROR),
            }
        }
        match self.engine.pull() {
            GameState::PendingEffect(eff) => {
                eff.all_effects();
            }
            _ => panic!("{}", INTERNAL_ERROR),
        }
        self.decision_context
            .push(chosen_context.expect(INTERNAL_ERROR));
    }

    pub fn backward_step(&mut self) {
        self.decision_context.pop();
        if !self.engine.undo_last_decision() {
            panic!("{}", INTERNAL_ERROR)
        }
    }

    pub fn is_finished(&self) -> bool {
        self.engine.is_finished()
    }

    pub fn decision_context(&self) -> &[(T::Context, usize)] {
        &self.decision_context
    }

    pub fn engine(&mut self) -> &mut Engine<T, EventLog<T>> {
        &mut self.engine
    }

    pub fn data(&self) -> &T {
        self.engine.data()
    }

    // TODO: probably shouldn't be mut
    pub fn player(&mut self) -> usize {
        match self.engine.pull() {
            GameState::PendingDecision(dec) => dec.player(),
            _ => panic!("{}", INTERNAL_ERROR),
        }
    }
}

#[cfg(test)]
mod test {
    use tgp::engine::Engine;

    use crate::{engine_stepper::EngineStepper, test::ZeroOneGame};

    #[test]
    fn stepping_test() {
        let data = ZeroOneGame::new(false, 3);
        let mut engine = Engine::new_logging(2, data);
        let mut stepper = EngineStepper::new(&mut engine);
        assert!(!stepper.is_finished());

        stepper.forward_step(&[0]);
        assert!(!stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 1);
        assert_eq!(stepper.data().num_ones, 0);

        stepper.backward_step();
        assert!(!stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 0);
        assert_eq!(stepper.data().num_ones, 0);

        stepper.forward_step(&[1]);
        stepper.forward_step(&[0, 1]);
        assert!(stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 1);
        assert_eq!(stepper.data().num_ones, 2);

        stepper.backward_step();
        assert!(!stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 0);
        assert_eq!(stepper.data().num_ones, 1);

        stepper.forward_step(&[1, 1]);
        assert!(stepper.is_finished());
        assert_eq!(stepper.data().num_zeros, 0);
        assert_eq!(stepper.data().num_ones, 3);
    }
}
