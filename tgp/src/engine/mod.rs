pub mod abstract_engine;
mod concrete_engine;

pub use concrete_engine::*;

use std::{
    fmt::{self, Debug},
    mem,
};

use crate::{Decision, Effect, GameData, Outcome};

const INTERNAL_ERROR: &str = "Internal error - invalid state";

enum InternalState<T: GameData> {
    PEffect(Box<dyn Effect<T>>),
    PDecision(Box<dyn Decision<T>>, Vec<Box<dyn Decision<T>>>),
    Finished,
    Invalid,
}

pub struct Engine<T: GameData> {
    state: InternalState<T>,
    data: T,
    num_players: usize,
}

impl<T: GameData> Engine<T> {
    pub fn new(num_players: usize, data: T) -> Self {
        Self {
            state: Self::fetch_decision(num_players, &data),
            data,
            num_players,
        }
    }

    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T: GameData + Debug> Debug for Engine<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_str = match self.state {
            InternalState::PEffect(_) => "PendingEffect",
            InternalState::PDecision(_, _) => "PendingDecision",
            InternalState::Finished => "Finished",
            InternalState::Invalid => "INVALID",
        };
        write!(
            f,
            "Engine {{ state: {}, data: {:?}, num_players: {:?} }}",
            state_str, &self.data, self.num_players
        )
    }
}

// ----- internal implementation -----
// TODO: transition-based API?
// internal traits for dynamic state handling
// must not be used for anything else!

trait PEffectState {
    fn next_effect(&mut self) -> Option<&mut dyn PEffectState>;
}

trait PDecisionState {
    fn select_option(&mut self, index: usize);

    fn option_count(&self) -> usize;

    fn player(&self) -> usize;

    fn level_in_chain(&self) -> usize;

    fn retract_n(&mut self, n: usize) -> bool;

    fn retract_all(&mut self);
}

impl<T: GameData> Engine<T> {
    fn fetch_decision(num_players: usize, data: &T) -> InternalState<T> {
        match data.next_decision() {
            Some(decision) => {
                assert!(
                    decision.player() < num_players,
                    "Illegal player for decision: {:?}",
                    decision.player()
                );
                InternalState::PDecision(decision, Vec::new())
            }
            None => InternalState::Finished,
        }
    }

    fn take_effect(state: &mut InternalState<T>) -> Box<dyn Effect<T>> {
        let state = mem::replace(state, InternalState::Invalid);
        match state {
            InternalState::PEffect(effect) => effect,
            _ => panic!(INTERNAL_ERROR),
        }
    }

    fn decision_stack(&self) -> &Vec<Box<dyn Decision<T>>> {
        match &self.state {
            InternalState::PDecision(_, stack) => stack,
            _ => panic!(INTERNAL_ERROR),
        }
    }

    fn decision_stack_mut(&mut self) -> &mut Vec<Box<dyn Decision<T>>> {
        match &mut self.state {
            InternalState::PDecision(_, stack) => stack,
            _ => panic!(INTERNAL_ERROR),
        }
    }

    fn decision(&self) -> &dyn Decision<T> {
        match &self.state {
            InternalState::PDecision(bottom, stack) => match stack.last() {
                Some(decision) => decision,
                None => bottom,
            }
            .as_ref(),
            _ => panic!(INTERNAL_ERROR),
        }
    }

    fn context(&self) -> T::Context {
        self.decision().context(&self.data)
    }
}

impl<T: GameData> PEffectState for Engine<T> {
    fn next_effect(&mut self) -> Option<&mut dyn PEffectState> {
        let next = Self::take_effect(&mut self.state).apply(&mut self.data);

        if let Some(effect) = next {
            self.state = InternalState::PEffect(effect);
            Some(self)
        } else {
            self.state = Self::fetch_decision(self.num_players, self.data());
            None
        }
    }
}

impl<T: GameData> PDecisionState for Engine<T> {
    fn select_option(&mut self, index: usize) {
        match self.decision().select_option(&self.data, index) {
            Outcome::Effect(effect) => {
                self.state = InternalState::PEffect(effect);
            }
            Outcome::FollowUp(decision) => {
                self.decision_stack_mut().push(decision);
            }
        }
    }

    fn option_count(&self) -> usize {
        self.decision().option_count()
    }

    fn player(&self) -> usize {
        self.decision().player()
    }

    fn level_in_chain(&self) -> usize {
        self.decision_stack().len()
    }

    fn retract_n(&mut self, n: usize) -> bool {
        let len = self.decision_stack().len();
        if n <= len {
            self.decision_stack_mut().truncate(len - n);
            true
        } else {
            false
        }
    }

    fn retract_all(&mut self) {
        self.decision_stack_mut().clear()
    }
}
