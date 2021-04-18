// TODO: better lifetime?
// TODO: reversible effect

/**
 * An effect changes the data of the game.
 */
pub trait Effect<M> {
    // TODO: multiple effects
    fn apply(&self, data: &mut M);
}

impl<M, F: Fn(&mut M)> Effect<M> for F {
    fn apply(&self, data: &mut M) {
        self(data)
    }
}

/**
 * A game decision.
 * `T`: GameData
 * `C`: Context
 */
pub trait Decision<T: GameData> {
    // panics at wrong index
    fn select_option(self: Box<Self>, data: &T, index: usize) -> Box<dyn Effect<T>>;

    fn option_count(&self) -> usize;

    fn player(&self) -> usize;

    fn context<'a>(&'a self, data: &'a T) -> &'a T::Context;
}

/**
 * Interface between the data and the GameEngine.
 */
pub trait GameData {
    /**
     * Context that is added to each decision.
     */
    type Context;

    fn next_decision(&self) -> Option<Box<dyn Decision<Self>>>;
}
