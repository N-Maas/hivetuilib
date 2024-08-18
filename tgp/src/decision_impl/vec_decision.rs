use std::fmt::{self, Debug};

use crate::{
    new_effect, new_rev_effect, vec_context::VecContext, Decision, Effect, GameData, Outcome,
    RevEffect,
};

/// Represents a decision with a player, a list of options
/// and a corresponding `VecContext`.
pub struct VecDecision<T: GameData, C: Clone, I: Clone = ()>
where
    T::Context: From<VecContext<C, I>>,
{
    options: Vec<Box<dyn Fn(&T) -> Outcome<T>>>,
    context: VecContext<C, I>,
    player: usize,
}

impl<T: GameData, C: Clone, I: Clone> Debug for VecDecision<T, C, I>
where
    T::Context: From<VecContext<C, I>>,
    C: Debug,
    I: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VecDecision {{ player: {:?}, context: {:#?}, options.len(): {:?} }}",
            self.player,
            &self.context,
            self.options.len()
        )
    }
}

impl<T: GameData, C: Clone, I: Clone> VecDecision<T, C, I>
where
    T::Context: From<VecContext<C, I>>,
    I: Default,
{
    pub fn new(player: usize) -> Self {
        Self::with_inner(player, Default::default())
    }
}

impl<T: GameData, C: Clone, I: Clone> VecDecision<T, C, I>
where
    T::Context: From<VecContext<C, I>>,
{
    pub fn with_inner(player: usize, inner: I) -> Self {
        Self {
            options: Vec::new(),
            context: VecContext::with_inner(inner),
            player,
        }
    }

    pub fn add_option(
        &mut self,
        outcome_fn: Box<dyn Fn(&T) -> Outcome<T>>,
        context: C,
    ) -> &mut Self {
        self.options.push(outcome_fn);
        self.context.push(context);
        self
    }

    pub fn add_follow_up<D, F>(&mut self, decision_fn: F, context: C) -> &mut Self
    where
        F: Fn(&T) -> D + 'static,
        D: Decision<T> + 'static,
    {
        self.add_option(
            Box::new(move |data| {
                let new_decision = decision_fn(data);
                Outcome::FollowUp(Box::new(new_decision))
            }),
            context,
        )
    }

    pub fn len(&self) -> usize {
        self.options.len()
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    // TODO: not a good name - but collides with decision trait..
    pub fn context_ref(&self) -> &VecContext<C, I> {
        &self.context
    }
}

impl<T, C: Clone, I: Clone> VecDecision<T, C, I>
where
    T: GameData<EffectType = dyn Effect<T>>,
    T::Context: From<VecContext<C, I>>,
{
    pub fn add_effect<A>(&mut self, apply: A, context: C) -> &mut Self
    where
        A: Fn(&mut T) -> Option<Box<dyn Effect<T>>> + Clone + Send + 'static,
    {
        self.add_option(
            Box::new(move |_| Outcome::Effect(new_effect(apply.clone()))),
            context,
        )
    }
}

impl<T, C: Clone, I: Clone> VecDecision<T, C, I>
where
    T: GameData<EffectType = dyn RevEffect<T>>,
    T::Context: From<VecContext<C, I>>,
{
    pub fn add_rev_effect<A, U>(&mut self, apply: A, undo: U, context: C) -> &mut Self
    where
        A: Fn(&mut T) -> Option<Box<dyn RevEffect<T>>> + Clone + Send + 'static,
        U: Fn(&mut T) + Clone + Send + 'static,
    {
        self.add_option(
            Box::new(move |_| Outcome::Effect(new_rev_effect(apply.clone(), undo.clone()))),
            context,
        )
    }
}

impl<T: GameData, C: Clone, I: Clone> Decision<T> for VecDecision<T, C, I>
where
    T::Context: From<VecContext<C, I>>,
{
    fn select_option(&self, data: &T, index: usize) -> Outcome<T> {
        debug_assert!(self.options.len() == self.context.len());
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
        debug_assert!(self.options.len() == self.context.len());
        T::Context::from(self.context.clone())
    }
}
