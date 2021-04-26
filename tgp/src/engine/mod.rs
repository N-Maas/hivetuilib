pub mod abstract_engine;
mod concrete_engine;
pub mod logging;

pub use concrete_engine::*;

use std::{
    fmt::{self, Debug},
    mem,
};

use crate::{Decision, Effect, GameData, Outcome, RevEffect};

use self::logging::EventLog;

const INTERNAL_ERROR: &str = "Internal error - invalid state";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneError {
    /// Cloning is not possible in pending effect state,
    /// the state must be either pending decision or finished.
    PendingEffect,
    /// Cloning is not possible for a pending follow-up decision,
    /// the state must be either a pending top-level decision or finished.
    FollowUp,
}

pub trait EventListener<T: GameData> {
    fn effect_applied(&mut self, effect: Box<T::EffectType>);

    fn option_selected(&mut self, index: usize);

    fn retracted_by_n(&mut self, n: usize);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotListening {}

impl<T: GameData> EventListener<T> for NotListening {
    fn effect_applied(&mut self, _effect: Box<T::EffectType>) {}

    fn option_selected(&mut self, _index: usize) {}

    fn retracted_by_n(&mut self, _n: usize) {}
}

enum InternalState<T: GameData> {
    PEffect(Box<T::EffectType>),
    PDecision(Box<dyn Decision<T>>, Vec<Box<dyn Decision<T>>>),
    Finished,
    Invalid,
}

pub struct Engine<T: GameData, L: EventListener<T> = NotListening> {
    state: InternalState<T>,
    // TODO: use mutable reference instead?
    data: T,
    listener: L,
    num_players: usize,
}

pub type LoggingEngine<T> = Engine<T, EventLog<T>>;

impl<T: GameData> Engine<T> {
    pub fn new(num_players: usize, data: T) -> Self {
        Self::with_listener(num_players, data, NotListening {})
    }
}

impl<T: GameData> Engine<T, EventLog<T>>
where
    T::EffectType: RevEffect<T>,
{
    pub fn new_logging(num_players: usize, data: T) -> Self {
        Self::with_listener(num_players, data, EventLog::new())
    }
}

impl<T: GameData, L: EventListener<T>> Engine<T, L> {
    pub fn with_listener(num_players: usize, data: T, listener: L) -> Self {
        let mut result = Self {
            state: InternalState::Invalid,
            data,
            listener,
            num_players,
        };
        result.state = result.fetch_next_state();
        result
    }
}

impl<T: GameData + Clone, L: EventListener<T>> Engine<T, L> {
    pub fn try_clone_data(&self) -> Result<Engine<T>, CloneError> {
        self.try_clone_with_listener(NotListening {})
    }

    pub fn try_clone(&self) -> Result<Self, CloneError>
    where
        L: Clone,
    {
        self.try_clone_with_listener(self.listener.clone())
    }

    pub fn try_clone_with_listener<M: EventListener<T>>(
        &self,
        listener: M,
    ) -> Result<Engine<T, M>, CloneError> {
        let state = match &self.state {
            InternalState::PEffect(_) => {
                return Err(CloneError::PendingEffect);
            }
            InternalState::PDecision(_, stack) => {
                if stack.is_empty() {
                    self.fetch_next_state()
                } else {
                    return Err(CloneError::FollowUp);
                }
            }
            InternalState::Finished => InternalState::Finished,
            InternalState::Invalid => panic!(INTERNAL_ERROR),
        };
        Ok(Engine {
            state,
            data: self.data.clone(),
            listener,
            num_players: self.num_players,
        })
    }
}

impl<T: GameData + Debug, L: EventListener<T> + Debug> Debug for Engine<T, L> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_str = match self.state {
            InternalState::PEffect(_) => "PendingEffect",
            InternalState::PDecision(_, _) => "PendingDecision",
            InternalState::Finished => "Finished",
            InternalState::Invalid => "INVALID",
        };
        write!(
            f,
            "Engine {{ state: {}, data: {:?}, listener: {:?}, num_players: {:?} }}",
            state_str, &self.data, &self.listener, self.num_players
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

impl<T: GameData, L: EventListener<T>> Engine<T, L> {
    fn fetch_next_state(&self) -> InternalState<T> {
        match self.data.next_decision() {
            Some(decision) => {
                assert!(
                    decision.player() < self.num_players,
                    "Illegal player for decision: {:?}",
                    decision.player()
                );
                InternalState::PDecision(decision, Vec::new())
            }
            None => InternalState::Finished,
        }
    }

    fn take_effect(&mut self) -> Box<T::EffectType> {
        let state = mem::replace(&mut self.state, InternalState::Invalid);
        match state {
            InternalState::PEffect(effect) => effect,
            _ => panic!(INTERNAL_ERROR),
        }
    }

    fn select_and_apply_option(&mut self, index: usize) {
        match self.decision().select_option(&self.data, index) {
            Outcome::Effect(effect) => {
                self.state = InternalState::PEffect(effect);
            }
            Outcome::FollowUp(decision) => {
                self.decision_stack_mut().push(decision);
            }
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

impl<T: GameData, L: EventListener<T>> PEffectState for Engine<T, L> {
    fn next_effect(&mut self) -> Option<&mut dyn PEffectState> {
        let effect = self.take_effect();
        let next = effect.apply(&mut self.data);
        self.listener.effect_applied(effect);

        if let Some(effect) = next {
            self.state = InternalState::PEffect(effect);
            Some(self)
        } else {
            self.state = self.fetch_next_state();
            None
        }
    }
}

impl<T: GameData, L: EventListener<T>> PDecisionState for Engine<T, L> {
    fn select_option(&mut self, index: usize) {
        self.select_and_apply_option(index);
        self.listener.option_selected(index);
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
            self.listener.retracted_by_n(n);
            true
        } else {
            false
        }
    }

    fn retract_all(&mut self) {
        self.listener.retracted_by_n(self.decision_stack().len());
        self.decision_stack_mut().clear();
    }
}

impl<T: GameData> Engine<T, EventLog<T>>
where
    T::EffectType: RevEffect<T>,
{
    pub fn undo_last_decision(&mut self) -> bool {
        if self.listener.undo_last_decision(&mut self.data) {
            self.state = self.fetch_next_state();
            true
        } else {
            false
        }
    }
}
