use std::marker::PhantomData;

use crate::{Decision, GameData, Outcome};

// TODO: Debug
/// Most powerful (and often, most performant) representation of a decision.
/// This decision type consists of two mapping functions that lazily
/// calculate an effect respective the context when required.
#[derive(Clone)]
pub struct LazyDecision<T: GameData, MapO, MapC, C>
where
    MapO: Fn(&T, &C, usize) -> Outcome<T>,
    MapC: Fn(&T, &C) -> T::Context,
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
    MapO: Fn(&T, &C, usize) -> Outcome<T>,
    MapC: Fn(&T, &C) -> T::Context,
{
    pub fn new(
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
    MapO: Fn(&T, &C, usize) -> Outcome<T>,
    MapC: Fn(&T, &C) -> T::Context,
{
    fn select_option(&self, data: &T, index: usize) -> Outcome<T> {
        assert!(
            index < self.option_count,
            "Invalid option: {}. Only {} options available.",
            index,
            self.option_count()
        );
        (self.outcome_mapping)(data, &self.context, index)
    }

    fn option_count(&self) -> usize {
        self.option_count
    }

    fn player(&self) -> usize {
        self.player
    }

    fn context(&self, data: &T) -> T::Context {
        (self.context_mapping)(data, &self.context)
    }
}
