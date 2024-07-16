use std::num::NonZeroU64;

use derive_builder::Builder;

#[derive(Builder, Clone, Copy)]
pub struct Position {
    #[allow(dead_code)]
    line: NonZeroU64,
    #[allow(dead_code)]
    col: NonZeroU64,
}
