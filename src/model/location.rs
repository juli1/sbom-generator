use derive_builder::Builder;

use crate::model::position::Position;

#[derive(Builder, Clone, Default, Debug)]
pub struct Location {
    #[allow(dead_code)]
    pub file: String,
    #[allow(dead_code)]
    pub start: Position,
    #[allow(dead_code)]
    pub end: Position,
}
