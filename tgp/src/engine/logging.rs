use std::fmt::{self, Debug};

use crate::{GameData, RevEffect};

use super::EventListener;

#[derive(Clone)]
pub enum Event<T: GameData> {
    Effect(Box<T::EffectType>),
    /// index, player
    Decision(usize, usize),
}

impl<T: GameData> Event<T> {
    pub fn is_decision(&self) -> bool {
        match self {
            Event::Effect(_) => false,
            Event::Decision(_, _) => true,
        }
    }
}

impl<T: GameData> Debug for Event<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Effect(_) => {
                write!(f, "Event::Effect(Box<_>)")
            }
            Event::Decision(val, player) => write!(f, "Event::Decision({val:?}, {player:?})"),
        }
    }
}

// TODO: some verification? E.g. player, hash?
// TODO: safe point you can reset to (via generational indizes)?
// TODO: snapshots (requires clone) - lift RevEffect requirement?
pub struct EventLog<T: GameData>
where
    T::EffectType: RevEffect<T>,
{
    log: Vec<Event<T>>,
    redo_stack: Vec<(usize, usize)>,
}

impl<T: GameData> Debug for EventLog<T>
where
    T::EffectType: RevEffect<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EventLog: {{log: {:#?}, redo_stack: {:#?}}}",
            &self.log, &self.redo_stack
        )
    }
}

impl<T: GameData> Default for EventLog<T>
where
    T::EffectType: RevEffect<T>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: GameData> EventLog<T>
where
    T::EffectType: RevEffect<T>,
{
    pub fn new() -> Self {
        Self {
            log: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn redo_effect(&mut self, effect: Box<T::EffectType>) {
        self.log.push(Event::Effect(effect));
    }

    // TODO: correct behavior when in subdecision state?
    pub fn undo_last_decision(&mut self, data: &mut T) -> bool {
        // initialize with sentinel value to avoid edge case
        let mut current_event = Event::Decision(0, 0);
        // pop subdecision
        while current_event.is_decision() {
            if let Some(event) = self.log.pop() {
                current_event = event;
            } else {
                return false;
            }
        }
        // undo effects
        while let Event::Effect(effect) = current_event {
            current_event = self.log.pop().expect("Internal error: Inconsistent log.");
            effect.undo(data);
        }
        // push decision to redo stack
        while let Event::Decision(index, player) = current_event {
            self.redo_stack.push((index, player));
            if let Some(event) = self.log.pop() {
                current_event = event;
            } else {
                return true;
            }
        }
        self.log.push(current_event);
        true
    }

    pub fn redo_available(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// The event is also pushed to the log (so don't do this a second time).
    pub fn redo_step(&mut self) -> Option<usize> {
        self.redo_stack.pop().map(|(index, player)| {
            self.log.push(Event::Decision(index, player));
            index
        })
    }
}

impl<T: GameData> EventListener<T> for EventLog<T>
where
    T::EffectType: RevEffect<T>,
{
    fn effect_applied(&mut self, effect: Box<T::EffectType>) {
        self.log.push(Event::Effect(effect));
        self.redo_stack.clear();
    }

    fn option_selected(&mut self, index: usize, player: usize) {
        self.log.push(Event::Decision(index, player));
        self.redo_stack.clear();
    }

    fn retracted_by_n(&mut self, mut n: usize) {
        while n > 0 {
            let top = self.log.pop().unwrap();
            n -= 1;
            assert!(top.is_decision());
        }
    }
}
