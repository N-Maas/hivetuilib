use std::fmt::{self, Debug};

// TODO: better lifetime?
// TODO: chained effect

/// An effect changes the data of the game.
pub trait Effect<T: GameData>: Send {
    /// Modifies the data. Can optionally return a new effect
    /// which is applied after this one.
    fn apply(&self, data: &mut T) -> Option<Box<T::EffectType>>;
}

pub trait RevEffect<T: GameData>: Effect<T> {
    fn undo(&self, data: &mut T);
}

/// Outcome of a decision - either an effect or a follow-up decision.
pub enum Outcome<T: GameData> {
    Effect(Box<T::EffectType>),
    FollowUp(Box<dyn Decision<T>>),
}

impl<T: GameData> Debug for Outcome<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    /// panics at wrong index
    fn select_option(&self, data: &T, index: usize) -> Outcome<T>;

    fn option_count(&self) -> usize;

    fn player(&self) -> usize;

    // seems like the best/most simple we can do in absence of GATs
    fn context(&self, data: &T) -> T::Context;
}

/// A context that where a specific element is provided for each
/// option of the associated decision.
pub trait IndexableContext {
    type ContextElement;

    fn select(&self, index: usize) -> Self::ContextElement;
}

/// Interface between the data and the GameEngine.
pub trait GameData: Sized + 'static {
    /// Context that is added to each decision.
    type Context;
    /// The type of used effects (in most cases, you want to use `dyn Effect<Self>`
    /// for non-reversible or `dyn RevEffect<T>` for reversible effects).
    type EffectType: Effect<Self> + ?Sized;

    fn next_decision(&self) -> Option<Box<dyn Decision<Self>>>;
}
