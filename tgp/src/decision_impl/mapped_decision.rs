use std::marker::PhantomData;

use crate::{vec_context::VecContext, Decision, Effect, GameData, Outcome};

pub trait MapToOutcome<T: GameData, C: Clone, I: Clone> {
    fn apply_mapping(&self, data: &T, inner: &I, context: &C) -> Outcome<T>;
}

impl<F, T: GameData, C: Clone, I: Clone> MapToOutcome<T, C, I> for F
where
    F: Fn(&T, &I, &C) -> Outcome<T>,
{
    fn apply_mapping(&self, data: &T, inner: &I, context: &C) -> Outcome<T> {
        self(data, inner, context)
    }
}

#[derive(Debug, Clone)]
pub struct MapToEffect<F> {
    mapping: F,
}

impl<F, E, T: GameData, C: Clone, I: Clone> MapToOutcome<T, C, I> for MapToEffect<F>
where
    F: Fn(&I, &C) -> E,
    E: Effect<T> + 'static,
{
    fn apply_mapping(&self, _data: &T, inner: &I, context: &C) -> Outcome<T> {
        Outcome::Effect(Box::new((self.mapping)(inner, context)))
    }
}

#[derive(Debug, Clone)]
pub struct MapToFollowUp<F> {
    mapping: F,
}

impl<F, D, T: GameData, C: Clone, I: Clone> MapToOutcome<T, C, I> for MapToFollowUp<F>
where
    F: Fn(&T, &I, &C) -> D,
    D: Decision<T> + 'static,
{
    fn apply_mapping(&self, data: &T, inner: &I, context: &C) -> Outcome<T> {
        Outcome::FollowUp(Box::new((self.mapping)(data, inner, context)))
    }
}

// TODO: Debug?
/// A powerful and performant representation of a decision.
/// This decision type uses a mapping function and a list of context
/// elements to calculate the outcome lazily.
#[derive(Debug, Clone)]
struct MappedDecisionImpl<F, T: GameData, C: Clone, I: Clone = ()>
where
    F: MapToOutcome<T, C, I>,
    T::Context: From<VecContext<C, I>>,
{
    mapping: F,
    context: VecContext<C, I>,
    player: usize,
    _t: PhantomData<T>,
}

impl<T: GameData, F, C: Clone, I: Clone> MappedDecisionImpl<F, T, C, I>
where
    F: MapToOutcome<T, C, I>,
    T::Context: From<VecContext<C, I>>,
{
    fn new(mapping: F, context: VecContext<C, I>, player: usize) -> Self {
        Self {
            mapping,
            context,
            player,
            _t: PhantomData,
        }
    }
}

impl<T: GameData, F, C: Clone, I: Clone> Decision<T> for MappedDecisionImpl<F, T, C, I>
where
    F: MapToOutcome<T, C, I>,
    T::Context: From<VecContext<C, I>>,
{
    fn select_option(&self, data: &T, index: usize) -> Outcome<T> {
        self.mapping
            .apply_mapping(data, &self.context.inner(), &self.context[index])
    }

    fn option_count(&self) -> usize {
        self.context.len()
    }

    fn player(&self) -> usize {
        self.player
    }

    fn context(&self, _data: &T) -> T::Context {
        T::Context::from(self.context.clone())
    }
}

// ----- builder pattern -----
#[derive(Debug, Clone)]
pub struct MappedDecision<T: GameData + 'static, C: Clone + 'static, I: Clone + 'static = ()>
where
    T::Context: From<VecContext<C, I>>,
{
    context: VecContext<C, I>,
    player: usize,
    _t: PhantomData<T>,
}

impl<T: GameData + 'static, C: Clone + 'static> MappedDecision<T, C, ()>
where
    T::Context: From<VecContext<C>>,
{
    pub fn new(player: usize) -> Self {
        Self::with_inner(player, ())
    }
}

impl<T: GameData + 'static, C: Clone + 'static, I: Clone + 'static> MappedDecision<T, C, I>
where
    T::Context: From<VecContext<C, I>>,
{
    pub fn with_default(player: usize) -> Self
    where
        I: Default,
    {
        Self::with_inner(player, Default::default())
    }

    pub fn with_inner(player: usize, inner: I) -> Self {
        Self {
            context: VecContext::with_inner(inner),
            player,
            _t: PhantomData,
        }
    }

    pub fn add_option(&mut self, option: C) -> &mut Self {
        self.context.push(option);
        self
    }

    pub fn to_outcome<F>(self, mapping: F) -> Box<dyn Decision<T>>
    where
        F: MapToOutcome<T, C, I> + 'static,
    {
        Box::new(MappedDecisionImpl::new(mapping, self.context, self.player))
    }

    pub fn to_effect<F, E>(self, mapping: F) -> Box<dyn Decision<T>>
    where
        F: Fn(&I, &C) -> E + 'static,
        E: Effect<T> + 'static,
    {
        Box::new(MappedDecisionImpl::new(
            MapToEffect { mapping },
            self.context,
            self.player,
        ))
    }

    pub fn to_follow_up<F, D>(self, mapping: F) -> Box<dyn Decision<T>>
    where
        F: Fn(&T, &I, &C) -> D + 'static,
        D: Decision<T> + 'static,
    {
        Box::new(MappedDecisionImpl::new(
            MapToFollowUp { mapping },
            self.context,
            self.player,
        ))
    }

    pub fn len(&self) -> usize {
        self.context.len()
    }

    pub fn is_empty(&self) -> bool {
        self.context.is_empty()
    }

    pub fn context(&self) -> &VecContext<C, I> {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut VecContext<C, I> {
        &mut self.context
    }
}
