use std::fmt::Debug;

// TODO: better lifetime?
// TODO: reversible effect

/// An effect changes the data of the game.
pub trait Effect<T> {
    fn apply(&self, data: &mut T) -> Option<Box<dyn Effect<T>>>;
}

impl<T, F> Effect<T> for F
where
    F: Fn(&mut T) -> Option<Box<dyn Effect<T>>>,
{
    fn apply(&self, data: &mut T) -> Option<Box<dyn Effect<T>>> {
        self(data)
    }
}

/// Outcome of a decision - either an effect or a follow-up decision.
pub enum Outcome<T: GameData> {
    Effect(Box<dyn Effect<T>>),
    FollowUp(Box<dyn Decision<T>>),
}

impl<T: GameData> Debug for Outcome<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Effect(_) => {
                write!(f, "Outcome::Effect")
            }
            Outcome::FollowUp(_) => {
                write!(f, "Outcome::FollowUp")
            }
        }
    }
}

/// A game decision.
/// `T`: GameData
/// `C`: Context
pub trait Decision<T: GameData> {
    // panics at wrong index
    fn select_option(&self, data: &T, index: usize) -> Outcome<T>;

    fn option_count(&self) -> usize;

    fn player(&self) -> usize;

    fn context<'a>(&'a self, data: &'a T) -> &'a T::Context;
}

/// Interface between the data and the GameEngine.
pub trait GameData {
    /// Context that is added to each decision.
    type Context;

    fn next_decision(&self) -> Option<Box<dyn Decision<Self>>>;
}
