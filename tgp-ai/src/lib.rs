const INTERNAL_ERROR: &str = "Internal error in AI algorithm - impossible state";

type IndexType = u32;
type RatingType = i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecIndex(IndexType, IndexType);

mod engine_stepper;
mod params;
mod search_tree_state;

pub mod rater;

pub use params::*;
