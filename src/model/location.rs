use derive_builder::Builder;

use crate::model::position::Position;

#[derive(Builder, Clone)]
pub struct Location {
    #[allow(dead_code)]
    file: String,
    #[allow(dead_code)]
    start: Position,
    #[allow(dead_code)]
    end: Position,
}
