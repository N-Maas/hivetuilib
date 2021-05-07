use crate::RatingType;

pub struct Params {
    // general parameters
    pub depth: usize,
    pub limit_multiplier_base: usize,

    // parameters for branch cutting
    pub branch_limit_first_cut: usize,
    pub branch_limit_long_tail: usize,
    pub expanse_length: usize,
    pub branch_differences: DifferenceParams,

    // parameters for moves
    pub move_limit: usize,
    pub move_differences: DifferenceParams,
}

pub struct DifferenceParams {
    pub noticable: RatingType,
    pub probably_worse: RatingType,
    pub surely_worse: RatingType,
}

impl DifferenceParams {
    pub fn new(probably_worse: RatingType) -> Self {
        Self {
            noticable: (probably_worse + 3) / 4,
            probably_worse,
            surely_worse: 2 * probably_worse,
        }
    }

    pub fn detailed(
        noticable: RatingType,
        probably_worse: RatingType,
        surely_worse: RatingType,
    ) -> Self {
        Self {
            noticable,
            probably_worse,
            surely_worse,
        }
    }
}
