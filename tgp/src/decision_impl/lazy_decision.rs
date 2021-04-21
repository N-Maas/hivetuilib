use std::marker::PhantomData;

use crate::{vec_context::VecContext, Decision, Effect, GameData, Outcome};

/// Most powerful (and often, most performant) representation of a decision.
/// This decision type consists of two mapping functions that lazily
/// calculate an effect respective the context when required.
pub struct LazyDecision<T: GameData, MapO, MapC, C>
where
    MapO: MapToOutcome<T, C>,
    MapC: MapToContext<T, C>,
{
    outcome_mapping: MapO,
    context_mapping: MapC,
    context: C,
    option_count: usize,
    player: usize,
    _t: PhantomData<T>,
}

impl<T: GameData, MapO, MapC, C> LazyDecision<T, MapO, MapC, C>
where
    MapO: MapToOutcome<T, C>,
    MapC: MapToContext<T, C>,
{
    pub fn from_mappings(
        player: usize,
        outcome_mapping: MapO,
        context_mapping: MapC,
        context: C,
        option_count: usize,
    ) -> Self {
        Self {
            outcome_mapping,
            context_mapping,
            context,
            option_count,
            player,
            _t: PhantomData,
        }
    }
}

impl<T: GameData, MapO, MapC, C> Decision<T> for LazyDecision<T, MapO, MapC, C>
where
    MapO: MapToOutcome<T, C>,
    MapC: MapToContext<T, C>,
{
    fn select_option(&self, data: &T, index: usize) -> Outcome<T> {
        assert!(
            index < self.option_count,
            "Invalid option: {}. Only {} options available.",
            index,
            self.option_count()
        );
        self.outcome_mapping
            .apply_mapping(data, &self.context, index)
    }

    fn option_count(&self) -> usize {
        self.option_count
    }

    fn player(&self) -> usize {
        self.player
    }

    fn context(&self, data: &T) -> T::Context {
        self.context_mapping.apply_mapping(data, &self.context)
    }
}

// ----- specialized implementation -----

pub type MappedDecision<T, F, C, I> =
    LazyDecision<T, MapByEffect<F>, MapToVecContext, VecContext<C, I>>;

pub type GeneralMappedDecision<T, F, C, I> =
    LazyDecision<T, MapByOutcome<F>, MapToVecContext, VecContext<C, I>>;

pub trait MapToOutcome<T: GameData, C> {
    fn apply_mapping(&self, data: &T, context: &C, index: usize) -> Outcome<T>;
}

impl<F, T: GameData, C> MapToOutcome<T, C> for F
where
    F: Fn(&T, &C, usize) -> Outcome<T>,
{
    fn apply_mapping(&self, data: &T, context: &C, index: usize) -> Outcome<T> {
        self(data, context, index)
    }
}

pub trait MapToContext<T: GameData, C> {
    fn apply_mapping(&self, data: &T, context: &C) -> T::Context;
}

impl<F, T: GameData, C> MapToContext<T, C> for F
where
    F: Fn(&T, &C) -> T::Context,
{
    fn apply_mapping(&self, data: &T, context: &C) -> T::Context {
        self(data, context)
    }
}

#[derive(Debug, Clone)]
pub struct MapToVecContext {}

impl<T: GameData, C: Clone, I: Clone> MapToContext<T, VecContext<C, I>> for MapToVecContext
where
    T::Context: From<VecContext<C, I>>,
{
    fn apply_mapping(&self, _data: &T, context: &VecContext<C, I>) -> T::Context {
        T::Context::from(context.clone())
    }
}

#[derive(Debug)]
pub struct MapByOutcome<F> {
    mapping: F,
}

impl<T: GameData, F, C: Clone, I: Clone> MapToOutcome<T, VecContext<C, I>> for MapByOutcome<F>
where
    F: Fn(&I, &C) -> Outcome<T>,
{
    fn apply_mapping(&self, _data: &T, context: &VecContext<C, I>, index: usize) -> Outcome<T> {
        (self.mapping)(context.inner(), &context[index])
    }
}

#[derive(Debug)]
pub struct MapByEffect<F> {
    mapping: F,
}

impl<T: GameData, F, C: Clone, I: Clone> MapToOutcome<T, VecContext<C, I>> for MapByEffect<F>
where
    F: Fn(&I, &C) -> Box<dyn Effect<T>>,
{
    fn apply_mapping(&self, _data: &T, context: &VecContext<C, I>, index: usize) -> Outcome<T> {
        Outcome::Effect((self.mapping)(context.inner(), &context[index]))
    }
}

impl<T: GameData, F, C: Clone, I: Clone>
    LazyDecision<T, MapByEffect<F>, MapToVecContext, VecContext<C, I>>
where
    F: Fn(&I, &C) -> Box<dyn Effect<T>>,
    T::Context: From<VecContext<C, I>>,
    I: Default,
{
    pub fn new(player: usize, effect_mapping: F) -> Self {
        Self::with_inner(player, effect_mapping, Default::default())
    }
}

impl<T: GameData, F, C: Clone, I: Clone>
    LazyDecision<T, MapByOutcome<F>, MapToVecContext, VecContext<C, I>>
where
    F: Fn(&I, &C) -> Outcome<T>,
    T::Context: From<VecContext<C, I>>,
    I: Default,
{
    pub fn new_general(player: usize, outcome_mapping: F) -> Self {
        Self::with_inner_general(player, outcome_mapping, Default::default())
    }
}

impl<T: GameData, F, C: Clone, I: Clone>
    LazyDecision<T, MapByEffect<F>, MapToVecContext, VecContext<C, I>>
where
    F: Fn(&I, &C) -> Box<dyn Effect<T>>,
    T::Context: From<VecContext<C, I>>,
{
    pub fn with_inner(player: usize, effect_mapping: F, inner: I) -> Self {
        Self::from_mappings(
            player,
            MapByEffect {
                mapping: effect_mapping,
            },
            MapToVecContext {},
            VecContext::with_inner(inner),
            0,
        )
    }
}

impl<T: GameData, F, C: Clone, I: Clone>
    LazyDecision<T, MapByOutcome<F>, MapToVecContext, VecContext<C, I>>
where
    F: Fn(&I, &C) -> Outcome<T>,
    T::Context: From<VecContext<C, I>>,
{
    pub fn with_inner_general(player: usize, outcome_mapping: F, inner: I) -> Self {
        Self::from_mappings(
            player,
            MapByOutcome {
                mapping: outcome_mapping,
            },
            MapToVecContext {},
            VecContext::with_inner(inner),
            0,
        )
    }

    pub fn add_option(&mut self, option: C) -> &mut Self {
        self.context.push(option);
        self
    }

    pub fn len(&self) -> usize {
        self.context.len()
    }

    pub fn is_empty(&self) -> bool {
        self.context.is_empty()
    }

    // TODO: not a good name - but collides with decision trait..
    pub fn context_ref(&self) -> &VecContext<C, I> {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut VecContext<C, I> {
        &mut self.context
    }
}
