use std::fmt::Debug;

use crate::{Decision, Effect, GameData, Outcome};

/// A simple representation of a decision consisting of the player,
/// a list of effects and a cloneable context.
pub struct PlainDecision<T: GameData>
where
    T::Context: Clone,
{
    options: Vec<Box<dyn Fn(&T) -> Outcome<T>>>,
    context: T::Context,
    player: usize,
}

impl<T: GameData> Debug for PlainDecision<T>
where
    T::Context: Clone + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FlatDecision {{ player: {:?}, context: {:?}, options.len(): {:?} }}",
            self.player,
            &self.context,
            self.options.len()
        )
    }
}

// TODO: graceful context handling
impl<T: GameData> PlainDecision<T>
where
    T::Context: Clone + Default,
{
    pub fn new(player: usize) -> Self {
        Self::with_context(player, Default::default())
    }
}

impl<T: GameData> PlainDecision<T>
where
    T::Context: Clone,
{
    pub fn with_context(player: usize, context: T::Context) -> Self {
        Self {
            options: Vec::new(),
            context,
            player,
        }
    }

    pub fn add_option(&mut self, outcome_fn: Box<dyn Fn(&T) -> Outcome<T>>) -> &mut Self {
        self.options.push(outcome_fn);
        self
    }

    pub fn add_effect<E>(&mut self, effect: E) -> &mut Self
    where
        E: Effect<T> + Clone + 'static,
    {
        self.add_option(Box::new(move |_| {
            let new_effect = effect.clone();
            Outcome::Effect(Box::new(new_effect))
        }))
    }

    pub fn add_follow_up<D, F>(&mut self, decision_fn: F) -> &mut Self
    where
        F: Fn(&T) -> D + 'static,
        D: Decision<T> + 'static,
    {
        self.add_option(Box::new(move |data| {
            let new_decision = decision_fn(data);
            Outcome::FollowUp(Box::new(new_decision))
        }))
    }

    pub fn len(&self) -> usize {
        self.options.len()
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    // TODO: not a good name - but collides with decision trait..
    pub fn context_ref(&self) -> &T::Context {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut T::Context {
        &mut self.context
    }
}

impl<T: GameData> Decision<T> for PlainDecision<T>
where
    T::Context: Clone,
{
    fn select_option(&self, data: &T, index: usize) -> Outcome<T> {
        let outcome_fn = self.options.get(index).unwrap_or_else(|| {
            panic!(
                "Invalid option: {}. Only {} options available.",
                index,
                self.option_count()
            )
        });
        outcome_fn(data)
    }

    fn option_count(&self) -> usize {
        self.len()
    }

    fn player(&self) -> usize {
        self.player
    }

    fn context(&self, _data: &T) -> T::Context {
        self.context.clone()
    }
}
