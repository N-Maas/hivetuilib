use crate::{Decision, Effect, GameData, Outcome};

pub struct VecDecision<T: GameData> {
    options: Vec<Box<dyn Fn(&T) -> Outcome<T>>>,
    player: usize,
    context: T::Context,
}

// TODO: graceful context handling
// TODO: support follow-up decisions
impl<T: GameData> VecDecision<T> {
    pub fn new(player: usize, context: T::Context) -> Self {
        Self {
            options: Vec::new(),
            player,
            context,
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

    pub fn context_mut(&mut self) -> &mut T::Context {
        &mut self.context
    }
}

impl<T: GameData> Decision<T> for VecDecision<T> {
    fn select_option(&self, data: &T, index: usize) -> Outcome<T> {
        assert!(
            index < self.option_count(),
            "Invalid option: {}. Only {} options available.",
            index,
            self.option_count()
        );

        let outcome_fn = self.options.get(index).expect(&format!(
            "Invalid option: {}. Only {} options available.",
            index,
            self.option_count()
        ));
        outcome_fn(data)
    }

    fn option_count(&self) -> usize {
        self.options.len()
    }

    fn player(&self) -> usize {
        self.player
    }

    fn context(&self, _data: &T) -> &T::Context {
        &self.context
    }
}
