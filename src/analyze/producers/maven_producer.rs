use std::path::{Path, PathBuf};

use derive_builder::Builder;

use crate::analyze::producers::producer::{SbomProducer, SbomProducerConfiguration};
use crate::model::dependency::Dependency;

#[derive(Builder, Clone)]
pub struct MavenProducer {}

impl SbomProducer for MavenProducer {
    fn use_file(&self, path: &Path, _configuration: &SbomProducerConfiguration) -> bool {
        match path.file_name() {
            Some(e) => e.eq_ignore_ascii_case("pom.xml"),
            None => false,
        }
    }

    fn find_dependencies(
        &self,
        paths: &[PathBuf],
        configuration: &SbomProducerConfiguration,
    ) -> anyhow::Result<Vec<Dependency>> {
        if configuration.use_debug {
            for p in paths.iter() {
                println!("paths: {}", p.to_str().unwrap_or(""))
            }
        }
        anyhow::Ok(vec![])
    }
}
