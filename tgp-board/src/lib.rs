mod concrete_boards;
mod field;
mod trait_definitions;

pub mod hypothetical;
pub mod index_map;
pub mod search;
pub mod structures;

pub mod prelude {
    pub use crate::field::*;
    pub use crate::trait_definitions::*;
}

pub use concrete_boards::*;
pub use field::*;
pub use trait_definitions::*;
