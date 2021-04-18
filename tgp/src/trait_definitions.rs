// TODO: better lifetime?
// TODO: multiple effects

/**
 * An effect changes the data of the game.
 */
pub trait Effect<M> {
    fn apply(&self, data: &mut M);
}

impl<M, F: Fn(&mut M)> Effect<M> for F {
    fn apply(&self, data: &mut M) {
        self(data)
    }
}

/**
 * A game decision.
 */
pub trait Decision<T> {
    // panics at wrong index
    fn select_option(self: Box<Self>, index: usize) -> Box<dyn Effect<T>>;

    fn option_count(&self) -> usize;

    fn player(&self) -> usize;
}

/**
 * Interface between the data and the GameEngine.
 */
pub trait GameData {
    fn next_decision(&self) -> Option<Box<dyn Decision<Self>>>;
}
