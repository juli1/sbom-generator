use derive_builder::Builder;

use crate::model::location::Location;

#[derive(Builder, Clone)]
pub struct Dependency {
    #[allow(dead_code)]
    r#type: String,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version: Option<String>,
    #[allow(dead_code)]
    purl: String,
    #[allow(dead_code)]
    locations: Vec<Location>,
}
