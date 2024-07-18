use derive_builder::Builder;

use crate::model::location::Location;

#[derive(Clone, Copy, Default)]
pub enum DependencyType {
    #[default]
    Library,
}

#[derive(Builder, Clone, Default)]
pub struct DependencyLocation {
    #[allow(dead_code)]
    pub block: Location,
    #[allow(dead_code)]
    pub name: Location,
    #[allow(dead_code)]
    pub version: Location,
}

#[derive(Builder, Clone, Default)]
pub struct Dependency {
    #[allow(dead_code)]
    pub r#type: DependencyType,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub version: Option<String>,
    #[allow(dead_code)]
    pub purl: String,
    #[allow(dead_code)]
    pub location: Option<DependencyLocation>,
}
