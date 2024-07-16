use std::path::{Path, PathBuf};

use crate::model::dependency::Dependency;

pub struct SbomProducerConfiguration {
    pub use_debug: bool,
}

/// Generic trait for SBOM producer
pub trait SbomProducer {
    /// Report if a file should be scanned or not
    fn use_file(&self, path: &Path, configuration: &SbomProducerConfiguration) -> bool;
    fn find_dependencies(
        &self,
        paths: &[PathBuf],
        configuration: &SbomProducerConfiguration,
    ) -> anyhow::Result<Vec<Dependency>>;
}
