use std::fmt::Debug;

use crate::GameData;

use super::{Engine, InternalState, PDecisionState, PEffectState, INTERNAL_ERROR};

/// Abstracted engine trait (with erased type parameter).
pub trait AbstractEngine {
    fn pull_abstract(&mut self) -> AbstractState<'_>;
}

#[derive(Debug)]
pub enum AbstractState<'a> {
    PendingEffect(AbstractPendingEffect<'a>),
    PendingDecision(AbstractPendingDecision<'a>),
    Finished(AbstractFinished),
}

impl<T: GameData> AbstractEngine for Engine<T> {
    fn pull_abstract(&mut self) -> AbstractState<'_> {
        match &self.state {
            InternalState::PEffect(_) => {
                AbstractState::PendingEffect(AbstractPendingEffect { state: self })
            }
            InternalState::PDecision(_, _) => {
                AbstractState::PendingDecision(AbstractPendingDecision { state: self })
            }
            InternalState::Finished => AbstractState::Finished(AbstractFinished {}),
            InternalState::Invalid => panic!("Internal error - invalid state"),
        }
    }
}

pub struct AbstractPendingEffect<'a> {
    state: &'a mut dyn PEffectState,
}

impl<'a> AbstractPendingEffect<'a> {
    pub fn next_effect(self) {
        self.state.next_effect();
    }

    pub fn all_effects(mut self) {
        while let Some(state) = self.state.next_effect() {
            self.state = state;
        }
    }
}

impl<'a> Debug for AbstractPendingEffect<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AbstractPendingEffect")
    }
}

pub struct AbstractPendingDecision<'a> {
    // panics at wrong index
    state: &'a mut dyn PDecisionState,
}

impl<'a> AbstractPendingDecision<'a> {
    pub fn select_option(self, index: usize) {
        self.state.select_option(index)
    }

    pub fn option_count(&self) -> usize {
        self.state.option_count()
    }

    pub fn player(&self) -> usize {
        self.state.player()
    }

    pub fn level_in_chain(&self) -> usize {
        self.state.level_in_chain()
    }

    pub fn is_follow_up_decision(&self) -> bool {
        self.state.level_in_chain() > 0
    }

    pub fn into_follow_up_decision(self) -> Option<AbstractFollowUpDecision<'a>> {
        if self.is_follow_up_decision() {
            Some(AbstractFollowUpDecision { state: self.state })
        } else {
            None
        }
    }
}

impl<'a> Debug for AbstractPendingDecision<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AbstractPendingDecision")
    }
}

pub struct AbstractFollowUpDecision<'a> {
    // panics at wrong index
    state: &'a mut dyn PDecisionState,
}

impl<'a> AbstractFollowUpDecision<'a> {
    pub fn select_option(self, index: usize) {
        self.state.select_option(index)
    }

    pub fn option_count(&self) -> usize {
        self.state.option_count()
    }

    pub fn player(&self) -> usize {
        self.state.player()
    }

    pub fn level_in_chain(&self) -> usize {
        self.state.level_in_chain()
    }

    pub fn is_follow_up_decision(&self) -> bool {
        self.state.level_in_chain() > 0
    }

    /// Retracts from the current subdecision.
    pub fn retract(self) {
        assert!(self.state.retract_n(1), "{}", INTERNAL_ERROR)
    }

    /// Retracts from n subdecisions and returns whether the retraction was successful.
    ///
    /// This is the case if and only if n <= #{pending decisions}.
    /// Otherwise, it has no effect.
    pub fn retract_n(self, n: usize) -> bool {
        self.state.retract_n(n)
    }

    /// Retracts from all subdecisions until the root decision is reached.
    pub fn retract_all(self) {
        self.state.retract_all()
    }
}

#[derive(Debug)]
pub struct AbstractFinished {
    // TODO additional information?
}
