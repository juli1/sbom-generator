use crate::analyze::producers::maven::constants::{ARTIFACT_ID, GROUP_ID, SCOPE, TYPE, VERSION};
use crate::analyze::producers::maven::context::MavenProducerContext;
use crate::analyze::producers::maven::model::{MavenDependencyScope, MavenDependencyType};
use crate::model::dependency::DependencyLocation;
use crate::model::location::Location;
use crate::model::position::get_position_in_string;
use crate::utils::tree_sitter::tree::get_tree;
use anyhow::anyhow;
use derive_builder::Builder;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Clone, Builder)]
pub struct MavenDependency {
    group_id: String,
    artifact_id: String,
    #[builder(default = "None")]
    version: Option<String>,
    #[builder(default = "None")]
    r#type: Option<MavenDependencyType>,
    #[builder(default = "None")]
    scope: Option<MavenDependencyScope>,
    #[builder(default = "None")]
    #[allow(dead_code)]
    location: Option<DependencyLocation>,
}

#[derive(Clone, Builder, Default)]
pub struct MavenFile {
    #[allow(dead_code)]
    path: PathBuf,
    properties: HashMap<String, String>,
    dependency_management: Vec<MavenDependency>,
    dependencies: Vec<MavenDependency>,
}


fn get_dependencies_from_dependency_maanagement(
    tree: &tree_sitter::Tree,
    path: &Path,
    content: &str,
    context: &MavenProducerContext,
) -> anyhow::Result<Vec<MavenDependency>> {
    let mut cursor = tree_sitter::QueryCursor::new();
    let mut dependencies: Vec<MavenDependency> = vec![];

    let matches = cursor.matches(
        &context.query_dependency_management,
        tree.root_node(),
        content.as_bytes(),
    );

    let path_string = path.display().to_string();

    for m in matches {
        let mut group_id_opt = None;
        let mut artifact_id_opt = None;
        let mut version_opt = None;
        let mut type_opt = None;
        let mut scope_opt = None;

        let mut name_position_opt: Option<Location> = None;
        let mut version_position_opt: Option<Location> = None;

        if m.captures.len() <= 1 {
            continue;
        }

        // the @element query
        let element_block = m.captures[0].node;

        let block_position_opt = Some(Location {
            file: path_string.clone(),
            start: get_position_in_string(content, element_block.start_byte())
                .expect("cannot find start"),
            end: get_position_in_string(content, element_block.end_byte())
                .expect("cannot find end"),
        });

        // get the version, name, option, artifact id, etc.
        for i in (5..m.captures.len()).step_by(2) {
            if m.captures[i].index != 4 {
                continue;
            }
            let tag_node = m.captures[i].node;
            let value_node = m.captures[i + 1].node;

            let tag = content[tag_node.start_byte()..tag_node.end_byte()].to_string();
            let value = content[value_node.start_byte()..value_node.end_byte()].to_string();
            if tag == ARTIFACT_ID {
                artifact_id_opt = Some(value.clone());

                name_position_opt = Some(Location {
                    file: path_string.clone(),
                    start: get_position_in_string(content, value_node.start_byte())
                        .expect("cannot find start"),
                    end: get_position_in_string(content, value_node.end_byte())
                        .expect("cannot find end"),
                });
            }
            if tag == GROUP_ID {
                group_id_opt = Some(value.clone());
            }
            if tag == VERSION {
                version_opt = Some(value.clone());

                version_position_opt = Some(Location {
                    file: path_string.clone(),
                    start: get_position_in_string(content, value_node.start_byte())
                        .expect("cannot find start"),
                    end: get_position_in_string(content, value_node.end_byte())
                        .expect("cannot find end"),
                });
            }
            if tag == TYPE {
                type_opt = MavenDependencyType::from_str(value.as_str()).ok();
            }
            if tag == SCOPE {
                scope_opt = MavenDependencyScope::from_str(value.as_str()).ok();
            }
        }

        if let (Some(group_id), Some(artifact_id)) =
            (group_id_opt.clone(), artifact_id_opt.clone())
        {
            let location = if let (Some(block_pos), Some(name_pos)) =
                (block_position_opt, name_position_opt)
            {
                Some(DependencyLocation {
                    block: block_pos,
                    name: name_pos,
                    version: version_position_opt,
                })
            } else {
                None
            };


            dependencies.push(
                MavenDependencyBuilder::default()
                    .group_id(group_id)
                    .artifact_id(artifact_id)
                    .version(version_opt)
                    .location(location)
                    .r#type(type_opt)
                    .scope(scope_opt)
                    .build()
                    .unwrap(),
            );
            continue;
        }
    }

    Ok(dependencies)
}

fn get_dependencies(
    tree: &tree_sitter::Tree,
    path: &Path,
    content: &str,
    context: &MavenProducerContext,
) -> anyhow::Result<Vec<MavenDependency>> {
    let mut cursor = tree_sitter::QueryCursor::new();
    let mut dependencies: Vec<MavenDependency> = vec![];

    let matches = cursor.matches(
        &context.query_dependencies,
        tree.root_node(),
        content.as_bytes(),
    );

    let path_string = path.display().to_string();

    for m in matches {
        let mut group_id_opt = None;
        let mut artifact_id_opt = None;
        let mut version_opt = None;
        let mut scope_opt = None;

        let mut name_position_opt: Option<Location> = None;
        let mut version_position_opt: Option<Location> = None;

        if m.captures.len() <= 1 {
            continue;
        }

        // the @element query
        let element_block = m.captures[0].node;

        let block_position_opt = Some(Location {
            file: path_string.clone(),
            start: get_position_in_string(content, element_block.start_byte())
                .expect("cannot find start"),
            end: get_position_in_string(content, element_block.end_byte())
                .expect("cannot find end"),
        });

        // get the version, name, option, artifact id, etc.
        for i in (0..m.captures.len()).step_by(2) {
            if m.captures[i].index != 3 {
                continue;
            }
            let tag_node = m.captures[i].node;
            let value_node = m.captures[i + 1].node;

            let tag = content[tag_node.start_byte()..tag_node.end_byte()].to_string();
            let value = content[value_node.start_byte()..value_node.end_byte()].to_string();
            if tag == ARTIFACT_ID {
                artifact_id_opt = Some(value.clone());

                name_position_opt = Some(Location {
                    file: path_string.clone(),
                    start: get_position_in_string(content, value_node.start_byte())
                        .expect("cannot find start"),
                    end: get_position_in_string(content, value_node.end_byte())
                        .expect("cannot find end"),
                });
            }
            if tag == GROUP_ID {
                group_id_opt = Some(value.clone());
            }
            if tag == VERSION {
                version_opt = Some(value.clone());

                version_position_opt = Some(Location {
                    file: path_string.clone(),
                    start: get_position_in_string(content, value_node.start_byte())
                        .expect("cannot find start"),
                    end: get_position_in_string(content, value_node.end_byte())
                        .expect("cannot find end"),
                });
            }
            if tag == SCOPE {
                scope_opt = MavenDependencyScope::from_str(value.as_str()).ok();
            }
        }

        if let (Some(group_id), Some(artifact_id)) =
            (group_id_opt.clone(), artifact_id_opt.clone())
        {
            let location = if let (Some(block_pos), Some(name_pos)) =
                (block_position_opt, name_position_opt)
            {
                Some(DependencyLocation {
                    block: block_pos,
                    name: name_pos,
                    version: version_position_opt,
                })
            } else {
                None
            };


            dependencies.push(
                MavenDependencyBuilder::default()
                    .group_id(group_id)
                    .artifact_id(artifact_id)
                    .version(version_opt)
                    .scope(scope_opt)
                    .location(location)
                    .build()
                    .unwrap(),
            );
            continue;
        }
    }

    Ok(dependencies)
}

fn get_variables(
    tree: &tree_sitter::Tree,
    file_content: &str,
    maven_producer_context: &MavenProducerContext,
) -> HashMap<String, String> {
    let mut variables = HashMap::new();

    // Get the project version is any
    let mut cursor = tree_sitter::QueryCursor::new();
    let matches = cursor.matches(
        &maven_producer_context.query_project_version,
        tree.root_node(),
        file_content.as_bytes(),
    );

    for m in matches {
        let value_node = m.captures[2].node;
        let value = file_content[value_node.start_byte()..value_node.end_byte()].to_string();
        variables.insert("project.version".to_string(), value);
    }

    // Get the project properties
    cursor = tree_sitter::QueryCursor::new();
    let matches = cursor.matches(
        &maven_producer_context.query_project_properties,
        tree.root_node(),
        file_content.as_bytes(),
    );
    for m in matches {
        let key_node = m.captures[2].node;
        let value_node = m.captures[3].node;
        let key = file_content[key_node.start_byte()..key_node.end_byte()].to_string();
        let value = file_content[value_node.start_byte()..value_node.end_byte()].to_string();
        variables.insert(key, value);
    }

    variables
}

impl MavenFile {
    fn new(path: &PathBuf, context: &MavenProducerContext) -> anyhow::Result<Self> {
        let file_content = fs::read_to_string(path);
        if let Ok(content) = file_content {
            if let Some(t) = get_tree(content.as_str(), &context.language) {
                let variables = get_variables(&t, content.as_str(), context);
                let dependencies = get_dependencies(&t, path, content.as_str(), context);
                let dependency_management = get_dependencies_from_dependency_maanagement(&t, path, content.as_str(), context);

                let maven_file = MavenFile {
                    path: path.clone(),
                    properties: variables,
                    dependency_management: dependency_management?,
                    dependencies: dependencies?,
                };
                Ok(maven_file)
            } else {
                Err(anyhow!("cannot parse tree"))
            }
        } else {
            Err(anyhow!("cannot parse file"))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pom() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/maven/simple/pom.xml");
        let context = MavenProducerContext::new();
        let maven_file = MavenFile::new(&d, &context).expect("maven file is parsed");

        assert_eq!(maven_file.dependencies.len(), 15);
        assert_eq!(maven_file.properties.len(), 9);
        assert_eq!(maven_file.properties.get("project.build.sourceEncoding").unwrap(), "UTF-8");
        assert_eq!(maven_file.properties.get("json.version").unwrap(), "20090211");
        assert_eq!(maven_file.properties.get("project.version").unwrap(), "1.2-SNAPSHOT");
    }

    #[test]
    fn test_parse_pom_with_dependency() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/maven/pom-import/pom.xml");
        let context = MavenProducerContext::new();
        let maven_file = MavenFile::new(&d, &context).expect("maven file is parsed");

        assert_eq!(maven_file.dependencies.len(), 7);
        assert_eq!(maven_file.properties.len(), 11);
        assert_eq!(maven_file.dependencies[0].group_id, "io.quarkus");
        assert_eq!(maven_file.dependencies[0].artifact_id, "quarkus-arc");
        assert!(maven_file.dependencies[0].r#type.is_none());
        assert!(maven_file.dependencies[0].scope.is_none());
        assert!(maven_file.dependencies[0].version.is_none());

        assert_eq!(maven_file.dependencies[1].group_id, "io.quarkus");
        assert_eq!(maven_file.dependencies[1].artifact_id, "quarkus-rest");
        assert!(maven_file.dependencies[1].r#type.is_none());
        assert!(maven_file.dependencies[1].scope.is_none());
        assert!(maven_file.dependencies[1].version.is_none());

        assert_eq!(maven_file.dependencies[2].group_id, "io.quarkus");
        assert_eq!(maven_file.dependencies[2].artifact_id, "quarkus-rest-jackson");
        assert!(maven_file.dependencies[2].r#type.is_none());
        assert!(maven_file.dependencies[2].scope.is_none());
        assert!(maven_file.dependencies[2].version.is_none());

        assert_eq!(maven_file.dependencies[3].group_id, "io.quarkus");
        assert_eq!(maven_file.dependencies[3].artifact_id, "quarkus-rest-client-jackson");
        assert!(maven_file.dependencies[3].r#type.is_none());
        assert!(maven_file.dependencies[3].scope.is_none());
        assert!(maven_file.dependencies[3].version.is_none());

        assert_eq!(maven_file.dependencies[4].group_id, "io.quarkus");
        assert_eq!(maven_file.dependencies[4].artifact_id, "quarkus-junit5");
        assert!(maven_file.dependencies[4].r#type.is_none());
        assert_eq!(maven_file.dependencies[4].scope.clone().unwrap(), MavenDependencyScope::Test);
        assert!(maven_file.dependencies[4].version.is_none());

        assert_eq!(maven_file.dependencies[5].group_id, "io.rest-assured");
        assert_eq!(maven_file.dependencies[5].artifact_id, "rest-assured");
        assert!(maven_file.dependencies[5].r#type.is_none());
        assert_eq!(maven_file.dependencies[5].scope.clone().unwrap(), MavenDependencyScope::Test);
        assert!(maven_file.dependencies[5].version.is_none());

        assert_eq!(maven_file.dependencies[6].group_id, "org.wiremock");
        assert_eq!(maven_file.dependencies[6].artifact_id, "wiremock");
        assert!(maven_file.dependencies[6].r#type.is_none());
        assert_eq!(maven_file.dependencies[6].scope.clone().unwrap(), MavenDependencyScope::Test);
        assert_eq!(maven_file.dependencies[6].version.clone().unwrap(), "${wiremock.version}");

        assert_eq!(maven_file.dependency_management.len(), 1);
        assert_eq!(maven_file.dependency_management[0].artifact_id, "${quarkus.platform.artifact-id}");
        assert_eq!(maven_file.dependency_management[0].scope.clone().unwrap(), MavenDependencyScope::Import);
        assert_eq!(maven_file.dependency_management[0].group_id, "${quarkus.platform.group-id}");
        assert_eq!(maven_file.dependency_management[0].clone().version.unwrap().as_str(), "${quarkus.platform.version}");
        assert_eq!(maven_file.dependency_management[0].clone().r#type.unwrap(), MavenDependencyType::Pom);
    }


    #[test]
    fn test_parse_pom_with_dependency_management() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/maven/hierarchy/pom.xml");
        let context = MavenProducerContext::new();
        let maven_file = MavenFile::new(&d, &context).expect("maven file is parsed");

        assert_eq!(maven_file.dependencies.len(), 3);
        assert_eq!(maven_file.dependency_management.len(), 11);
        assert_eq!(maven_file.properties.len(), 6);

        assert_eq!(maven_file.dependencies[0].group_id, "com.google.code.findbugs");
        assert_eq!(maven_file.dependencies[0].artifact_id, "jsr305");
        assert!(maven_file.dependencies[0].r#type.is_none());
        assert!(maven_file.dependencies[0].scope.is_none());
        assert!(maven_file.dependencies[0].version.is_none());

        assert_eq!(maven_file.dependencies[1].group_id, "org.immutables");
        assert_eq!(maven_file.dependencies[1].artifact_id, "value-annotations");
        assert!(maven_file.dependencies[1].r#type.is_none());
        assert_eq!(maven_file.dependencies[1].clone().scope.unwrap(), MavenDependencyScope::Provided);
        assert!(maven_file.dependencies[1].version.is_none());

        assert_eq!(maven_file.dependencies[2].group_id, "org.junit.jupiter");
        assert_eq!(maven_file.dependencies[2].artifact_id, "junit-jupiter-api");
        assert!(maven_file.dependencies[2].r#type.is_none());
        assert_eq!(maven_file.dependencies[2].clone().scope.unwrap(), MavenDependencyScope::Test);
        assert!(maven_file.dependencies[2].version.is_none());

        assert_eq!(maven_file.dependency_management[0].group_id, "com.typesafe.akka");
        assert_eq!(maven_file.dependency_management[0].artifact_id, "akka-actor_${akka-scala.version}");
        assert!(maven_file.dependency_management[0].r#type.is_none());
        assert!(maven_file.dependency_management[0].scope.is_none());
        assert_eq!(maven_file.dependency_management[0].version.clone().unwrap().as_str(), "${akka.version}");

        assert_eq!(maven_file.dependency_management[1].group_id, "com.typesafe.akka");
        assert_eq!(maven_file.dependency_management[1].artifact_id, "akka-slf4j_${akka-scala.version}");
        assert!(maven_file.dependency_management[1].r#type.is_none());
        assert!(maven_file.dependency_management[1].scope.is_none());
        assert_eq!(maven_file.dependency_management[1].version.clone().unwrap().as_str(), "${akka.version}");
    }
}
