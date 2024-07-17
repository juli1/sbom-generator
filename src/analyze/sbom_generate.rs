use std::path::PathBuf;

use crate::analyze::producers::maven_producer::MavenProducerBuilder;
use crate::analyze::producers::producer::{SbomProducer, SbomProducerConfiguration};
use crate::model::configuration::Configuration;
use crate::utils::file_utils::get_files;

/// Analyze paths, find dependencies and write the SBOM to disk.
/// The [configuration] is the configuration of the tool (directory to scan, etc)
pub fn analyze(configuration: &Configuration) -> anyhow::Result<()> {
    let mut dependencies = vec![];
    if configuration.use_debug {
        configuration.print_configuration();
    }

    let all_producers = vec![MavenProducerBuilder::default()
        .build()
        .expect("build producer")];

    let all_files = get_files(configuration.directory.as_str()).expect("cannot read directory");
    let producer_configuration = SbomProducerConfiguration {
        use_debug: configuration.use_debug,
    };

    for sbom_producer in all_producers {
        let producer_files = all_files
            .clone()
            .iter()
            .filter(|f| sbom_producer.use_file(f, &producer_configuration))
            .map(|v| (*v).clone())
            .collect::<Vec<PathBuf>>();
        let dependencies_found =
            sbom_producer.find_dependencies(producer_files.as_slice(), &producer_configuration);

        if let Ok(deps) = dependencies_found {
            dependencies.extend(deps);
        }
    }

    for dep in dependencies.iter() {
        println!(
            "dependency name={} version={}",
            dep.name,
            dep.version.clone().unwrap_or("no version".to_string())
        )
    }

    Ok(())
}
