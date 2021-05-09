use tgp::{
    engine::{logging::EventLog, Engine, GameEngine},
    GameData, RevEffect,
};

use crate::{IndexType, INTERNAL_ERROR};

pub(crate) struct EngineStepper<T: GameData>
where
    T::EffectType: RevEffect<T>,
{
    engine: Engine<T, EventLog<T>>,
}

impl<T: GameData> EngineStepper<T>
where
    T::EffectType: RevEffect<T>,
{
    pub fn new(engine: Engine<T, EventLog<T>>) -> Self {
        Self { engine }
    }

    pub fn forward_step(&mut self, index: IndexType) {
        todo!()
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
