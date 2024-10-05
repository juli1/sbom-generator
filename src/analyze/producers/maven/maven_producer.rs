use std::path::{Path, PathBuf};

use crate::analyze::producers::maven::context::MavenProducerContext;
use crate::analyze::producers::maven::maven_file::MavenFile;
use crate::analyze::producers::producer::{SbomProducer, SbomProducerConfiguration};
use crate::model::dependency::Dependency;
use derive_builder::Builder;

#[derive(Clone, Builder)]
pub struct MavenProducer {}

impl MavenProducer {}

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
        let mut result = vec![];

        let mut maven_context = MavenProducerContext::new(configuration.base_path.clone());

        // First pass, we are getting the dependency files
        for p in paths.iter() {
            let maven_file = MavenFile::new(p, &maven_context).expect("maven file is parsed");
            maven_context.add_maven_file(&maven_file);
        }

        // Second pass, we are resolving variables and extracting dependencies
        for maven_file in maven_context.get_all_files() {
            let deps: Vec<Dependency> = maven_file
                .get_dependencies_for_sbom(&maven_context)
                .iter()
                .map(|d| d.into())
                .collect();

            result.extend(deps)
        }

        anyhow::Ok(result)
    }
}
