use crate::analyze::producers::maven::constants::{ARTIFACT_ID, GROUP_ID, SCOPE, TYPE, VERSION};
use crate::analyze::producers::maven::context::MavenProducerContext;
use crate::analyze::producers::maven::model::{MavenDependencyScope, MavenDependencyType};
use crate::model::dependency::{Dependency, DependencyBuilder, DependencyLocation, DependencyType};
use crate::model::location::Location;
use crate::model::position::get_position_in_string;
use crate::utils::tree_sitter::tree::get_tree;
use anyhow::anyhow;
use derive_builder::Builder;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

lazy_static! {
    static ref REGEX_VARIABLE: Regex = Regex::new(r"\$\{(.+)\}").unwrap();
}

fn enrich_string_with_properties(s: &str, properties: &HashMap<String, String>) -> String {
    if let Some(caps) = REGEX_VARIABLE.captures(s) {
        let total_capture_opt = caps.get(0);
        let var_capture_opt = caps.get(1);

        match (total_capture_opt, var_capture_opt) {
            (Some(total_capture), Some(var_capture)) => {
                let var_val = s;
                let var_name = var_val.get(var_capture.start()..var_capture.end()).unwrap();
                println!("var capture: {}", var_name);

                if let Some(prop) = properties.get(var_name) {
                    println!("val: {}", prop);
                    let mut to_replace = var_val.get(0..total_capture.start()).unwrap().to_string();
                    to_replace.push_str(prop.as_str());
                    to_replace.push_str(var_val.get(total_capture.end()..var_val.len()).unwrap());
                    return to_replace;
                }
            }
            _ => {
                return s.to_string();
            }
        }
    }
    s.to_string()
}

#[derive(Clone, Builder)]
pub struct MavenDependency {
    pub group_id: String,
    pub artifact_id: String,
    #[builder(default = "None")]
    pub version: Option<String>,
    #[builder(default = "None")]
    pub r#type: Option<MavenDependencyType>,
    #[builder(default = "None")]
    pub scope: Option<MavenDependencyScope>,
    #[builder(default = "None")]
    #[allow(dead_code)]
    pub location: Option<DependencyLocation>,
}

impl MavenDependency {
    pub fn enrich(&self, properties: &HashMap<String, String>) -> Self {
        MavenDependency {
            group_id: enrich_string_with_properties(self.group_id.as_str(), properties),
            artifact_id: enrich_string_with_properties(self.artifact_id.as_str(), properties),
            version: self
                .version
                .clone()
                .map(|x| enrich_string_with_properties(x.as_str(), properties)),
            r#type: self.r#type.clone(),
            scope: self.scope.clone(),
            location: self.location.clone(),
        }
    }
}

impl From<&MavenDependency> for Dependency {
    fn from(value: &MavenDependency) -> Self {
        DependencyBuilder::default()
            .name(format!("{}:{}", value.group_id, value.artifact_id))
            .version(value.version.clone())
            .location(value.location.clone())
            .r#type(DependencyType::Library)
            .purl("bla".to_string())
            .build()
            .unwrap()
    }
}

#[derive(Clone, Builder, Default)]
pub struct MavenFileParent {
    pub relative_path: Option<String>,
}

#[derive(Clone, Builder, Default)]
pub struct MavenFile {
    pub path: PathBuf,
    pub properties: HashMap<String, String>,
    pub dependency_management: Vec<MavenDependency>,
    pub dependencies: Vec<MavenDependency>,
    pub parent: Option<MavenFileParent>,
}

fn get_dependencies_from_dependency_management(
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

        if let (Some(group_id), Some(artifact_id)) = (group_id_opt.clone(), artifact_id_opt.clone())
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

        if let (Some(group_id), Some(artifact_id)) = (group_id_opt.clone(), artifact_id_opt.clone())
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

pub fn get_variables(
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

fn get_parent_information(
    tree: &tree_sitter::Tree,
    _path: &Path,
    content: &str,
    context: &MavenProducerContext,
) -> Option<MavenFileParent> {
    let mut cursor = tree_sitter::QueryCursor::new();

    let mut matches = cursor.matches(
        &context.query_parent_information,
        tree.root_node(),
        content.as_bytes(),
    );

    if let Some(m) = matches.next() {
        let value_node = m.captures[3].node;
        let relative_path = content[value_node.start_byte()..value_node.end_byte()].to_string();
        return Some(MavenFileParent {
            relative_path: Some(relative_path),
        });
    }

    None
}

fn replace_properties(properties: HashMap<String, String>) -> HashMap<String, String> {
    let mut result = HashMap::new();
    // replace variables inside the properties
    for (k, v) in &properties {
        if let Some(caps) = REGEX_VARIABLE.captures(v.as_str()) {
            let total_capture_opt = caps.get(0);
            let var_capture_opt = caps.get(1);

            if let (Some(total_capture), Some(var_capture)) = (total_capture_opt, var_capture_opt) {
                let var_val = v.as_str();
                let var_name = var_val.get(var_capture.start()..var_capture.end()).unwrap();
                println!("var capture: {}", var_name);

                if let Some(prop) = properties.get(var_name) {
                    println!("val: {}", prop);
                    let mut to_replace = var_val.get(0..total_capture.start()).unwrap().to_string();
                    to_replace.push_str(prop.as_str());
                    to_replace.push_str(var_val.get(total_capture.end()..var_val.len()).unwrap());

                    result.insert(k.clone(), to_replace);
                }
            }
        } else {
            result.insert(k.clone(), v.clone());
        }
    }
    result
}

impl MavenFile {
    pub fn new(path: &PathBuf, context: &MavenProducerContext) -> anyhow::Result<Self> {
        let file_content = fs::read_to_string(path);
        if let Ok(content) = file_content {
            if let Some(t) = get_tree(content.as_str(), &context.language) {
                let variables = get_variables(&t, content.as_str(), context);
                let dependencies = get_dependencies(&t, path, content.as_str(), context);
                let dependency_management = get_dependencies_from_dependency_management(
                    &t,
                    path,
                    content.as_str(),
                    context,
                );
                let parent_information =
                    get_parent_information(&t, path, content.as_str(), context);

                let maven_file = MavenFile {
                    path: path.clone(),
                    properties: variables,
                    dependency_management: dependency_management?,
                    dependencies: dependencies?,
                    parent: parent_information,
                };
                Ok(maven_file)
            } else {
                Err(anyhow!("cannot parse tree"))
            }
        } else {
            Err(anyhow!("cannot parse file"))
        }
    }

    fn get_parent_file_path(&self, context: &MavenProducerContext) -> Option<PathBuf> {
        if let Some(relative_path) = self.parent.clone().and_then(|x| x.relative_path) {
            let bp = fs::canonicalize(&context.base_path).expect("cannot get base path");
            let mut f = self.path.clone().parent().unwrap().to_path_buf();
            f.push(relative_path);
            println!("test {:?}", f);
            let full_path = fs::canonicalize(f).expect("cannot get full path");
            println!("bp {:?}", bp);
            println!("fp {:?}", full_path);

            let mut rel_path = full_path
                .strip_prefix(&bp)
                .expect("get rel path")
                .to_path_buf();
            rel_path.push("pom.xml");
            println!("rel path: {:?}", rel_path);
            return Some(rel_path);
        }
        None
    }

    /// Get all properties related to this file and sub-files and put them in a HashMap.
    /// Also resolve variables when appropriate/possible.
    fn get_all_properties(
        &self,
        maven_files: &HashMap<PathBuf, MavenFile>,
        context: &MavenProducerContext,
    ) -> HashMap<String, String> {
        fn get_all_properties_int(
            maven_file: &MavenFile,
            maven_files: &HashMap<PathBuf, MavenFile>,
            context: &MavenProducerContext,
        ) -> HashMap<String, String> {
            let mut res: HashMap<String, String> = HashMap::new();

            let parent_path = maven_file.get_parent_file_path(context);

            if let Some(parent) = parent_path {
                if let Some(parent_maven_file) = maven_files.get(&parent) {
                    println!("found parent file");
                    res.extend(parent_maven_file.get_all_properties(maven_files, context));
                }
            }

            res.extend(maven_file.properties.clone());

            res
        }
        replace_properties(get_all_properties_int(self, maven_files, context))
    }

    fn get_all_dependencies_from_dependency_management(
        &self,
        maven_files: &HashMap<PathBuf, MavenFile>,
        context: &MavenProducerContext,
    ) -> Vec<MavenDependency> {
        let mut res: Vec<MavenDependency> = vec![];

        let parent_path = self.get_parent_file_path(context);

        if let Some(parent) = parent_path {
            if let Some(parent_maven_file) = maven_files.get(&parent) {
                res.extend(
                    parent_maven_file
                        .get_all_dependencies_from_dependency_management(maven_files, context),
                );
            }
        }

        res.extend(self.dependency_management.clone());

        res
    }

    pub fn get_dependencies_for_sbom(
        &self,
        maven_files: &HashMap<PathBuf, MavenFile>,
        context: &MavenProducerContext,
    ) -> Vec<MavenDependency> {
        let mut res = vec![];

        // get all properties from the current file and its parent
        let properties = &self.get_all_properties(maven_files, context);

        let dependencies_from_property_management =
            &self.get_all_dependencies_from_dependency_management(maven_files, context);

        for dependency in &self.dependencies {
            if dependency.version.is_none() {
                let dep_from_dep_management =
                    dependencies_from_property_management.iter().find(|x| {
                        x.artifact_id == dependency.artifact_id && x.group_id == dependency.group_id
                    });

                if let Some(dep) = dep_from_dep_management {
                    res.push(dep.clone().enrich(properties));
                }
            } else {
                res.push(dependency.clone().enrich(properties));
            }
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pom() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let context = MavenProducerContext::new(d.clone());
        d.push("resources/maven/simple/pom.xml");
        let maven_file = MavenFile::new(&d, &context).expect("maven file is parsed");

        assert_eq!(maven_file.dependencies.len(), 15);
        assert_eq!(maven_file.properties.len(), 9);
        assert_eq!(
            maven_file
                .properties
                .get("project.build.sourceEncoding")
                .unwrap(),
            "UTF-8"
        );
        assert_eq!(
            maven_file.properties.get("json.version").unwrap(),
            "20090211"
        );
        assert_eq!(
            maven_file.properties.get("project.version").unwrap(),
            "1.2-SNAPSHOT"
        );
    }

    #[test]
    fn test_parse_pom_with_dependency() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let context = MavenProducerContext::new(d.clone());
        d.push("resources/maven/pom-import/pom.xml");
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
        assert_eq!(
            maven_file.dependencies[2].artifact_id,
            "quarkus-rest-jackson"
        );
        assert!(maven_file.dependencies[2].r#type.is_none());
        assert!(maven_file.dependencies[2].scope.is_none());
        assert!(maven_file.dependencies[2].version.is_none());

        assert_eq!(maven_file.dependencies[3].group_id, "io.quarkus");
        assert_eq!(
            maven_file.dependencies[3].artifact_id,
            "quarkus-rest-client-jackson"
        );
        assert!(maven_file.dependencies[3].r#type.is_none());
        assert!(maven_file.dependencies[3].scope.is_none());
        assert!(maven_file.dependencies[3].version.is_none());

        assert_eq!(maven_file.dependencies[4].group_id, "io.quarkus");
        assert_eq!(maven_file.dependencies[4].artifact_id, "quarkus-junit5");
        assert!(maven_file.dependencies[4].r#type.is_none());
        assert_eq!(
            maven_file.dependencies[4].scope.clone().unwrap(),
            MavenDependencyScope::Test
        );
        assert!(maven_file.dependencies[4].version.is_none());

        assert_eq!(maven_file.dependencies[5].group_id, "io.rest-assured");
        assert_eq!(maven_file.dependencies[5].artifact_id, "rest-assured");
        assert!(maven_file.dependencies[5].r#type.is_none());
        assert_eq!(
            maven_file.dependencies[5].scope.clone().unwrap(),
            MavenDependencyScope::Test
        );
        assert!(maven_file.dependencies[5].version.is_none());

        assert_eq!(maven_file.dependencies[6].group_id, "org.wiremock");
        assert_eq!(maven_file.dependencies[6].artifact_id, "wiremock");
        assert!(maven_file.dependencies[6].r#type.is_none());
        assert_eq!(
            maven_file.dependencies[6].scope.clone().unwrap(),
            MavenDependencyScope::Test
        );
        assert_eq!(
            maven_file.dependencies[6].version.clone().unwrap(),
            "${wiremock.version}"
        );

        assert_eq!(maven_file.dependency_management.len(), 1);
        assert_eq!(
            maven_file.dependency_management[0].artifact_id,
            "${quarkus.platform.artifact-id}"
        );
        assert_eq!(
            maven_file.dependency_management[0].scope.clone().unwrap(),
            MavenDependencyScope::Import
        );
        assert_eq!(
            maven_file.dependency_management[0].group_id,
            "${quarkus.platform.group-id}"
        );
        assert_eq!(
            maven_file.dependency_management[0]
                .clone()
                .version
                .unwrap()
                .as_str(),
            "${quarkus.platform.version}"
        );
        assert_eq!(
            maven_file.dependency_management[0].clone().r#type.unwrap(),
            MavenDependencyType::Pom
        );
    }

    #[test]
    fn test_parse_pom_with_dependency_management() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let context = MavenProducerContext::new(d.clone());
        d.push("resources/maven/hierarchy/pom.xml");
        let maven_file = MavenFile::new(&d, &context).expect("maven file is parsed");

        assert_eq!(maven_file.dependencies.len(), 2);
        assert_eq!(maven_file.dependency_management.len(), 11);
        assert_eq!(maven_file.properties.len(), 6);

        assert_eq!(
            maven_file.dependencies[0].group_id,
            "com.google.code.findbugs"
        );
        assert_eq!(maven_file.dependencies[0].artifact_id, "jsr305");
        assert!(maven_file.dependencies[0].r#type.is_none());
        assert!(maven_file.dependencies[0].scope.is_none());
        assert!(maven_file.dependencies[0].version.is_none());

        assert_eq!(maven_file.dependencies[1].group_id, "org.immutables");
        assert_eq!(maven_file.dependencies[1].artifact_id, "value-annotations");
        assert!(maven_file.dependencies[1].r#type.is_none());
        assert_eq!(
            maven_file.dependencies[1].clone().scope.unwrap(),
            MavenDependencyScope::Provided
        );
        assert!(maven_file.dependencies[1].version.is_none());

        assert_eq!(
            maven_file.dependency_management[0].group_id,
            "com.typesafe.akka"
        );
        assert_eq!(
            maven_file.dependency_management[0].artifact_id,
            "akka-actor_${akka-scala.version}"
        );
        assert!(maven_file.dependency_management[0].r#type.is_none());
        assert!(maven_file.dependency_management[0].scope.is_none());
        assert_eq!(
            maven_file.dependency_management[0]
                .version
                .clone()
                .unwrap()
                .as_str(),
            "${akka.version}"
        );

        assert_eq!(
            maven_file.dependency_management[1].group_id,
            "com.typesafe.akka"
        );
        assert_eq!(
            maven_file.dependency_management[1].artifact_id,
            "akka-slf4j_${akka-scala.version}"
        );
        assert!(maven_file.dependency_management[1].r#type.is_none());
        assert!(maven_file.dependency_management[1].scope.is_none());
        assert_eq!(
            maven_file.dependency_management[1]
                .version
                .clone()
                .unwrap()
                .as_str(),
            "${akka.version}"
        );
    }
}
