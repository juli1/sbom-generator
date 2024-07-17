use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use derive_builder::Builder;
use regex::Regex;

use crate::analyze::producers::producer::{SbomProducer, SbomProducerConfiguration};
use crate::model::dependency::{Dependency, DependencyBuilder, DependencyType};
use crate::utils::tree_sitter::language::get_tree_sitter_xml;
use crate::utils::tree_sitter::tree::get_tree;

const ARTIFACT_ID: &str = "artifactId";
const GROUP_ID: &str = "groupId";
const VERSION: &str = "version";

const TREE_SITTER_QUERY_VARIABLES: &str = r###"
(element
   (STag
      (Name) @project
   )
   (content
      (element
         (STag
            (Name) @parent
         )
         (content
            (element
               (STag
                  (Name) @key
               )
               (content) @value
            )

         )
      )
   )
   (#eq? @project "project")
   (#eq? @parent "parent")
)
"###;

const TREE_SITTER_QUERY_DEPENDENCIES: &str = r###"
(element
   (STag
      (Name)@name
   )
   (content
     (
         (element
            (STag
               (Name)@tag
            )
            (content)@value
         )
         (CharData)
     )+
   )
   (#eq? @name "dependency")
   (#match? @tag "^artifactId$|^groupId$|^version$")
)
"###;

pub struct MavenProducerContext {
    query_variables: tree_sitter::Query,
    query_dependencies: tree_sitter::Query,
    language: tree_sitter::Language,
}

#[derive(Clone, Builder)]
pub struct MavenProducer {}

impl MavenProducer {
    fn get_variables(
        &self,
        tree: &tree_sitter::Tree,
        file_content: &str,
        maven_producer_context: &MavenProducerContext,
    ) -> HashMap<String, String> {
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut variables = HashMap::new();
        let matches = cursor.matches(
            &maven_producer_context.query_variables,
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
        // println!("variables={:?}", variables);
        variables
    }

    fn find_dependencies_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        content: &str,
        _configuration: &SbomProducerConfiguration,
        context: &MavenProducerContext,
    ) -> anyhow::Result<Vec<Dependency>> {
        let variable_regex = Regex::new(r"^\$\{[a-zA-Z0-9]+\.(.+)\}$").unwrap();
        let variables = self.get_variables(tree, content, context);
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut dependencies: Vec<Dependency> = vec![];
        let matches = cursor.matches(
            &context.query_dependencies,
            tree.root_node(),
            content.as_bytes(),
        );
        for m in matches {
            let mut group_id_opt = None;
            let mut artifact_id_opt = None;
            let mut version_opt = None;

            if m.captures.len() == 1 {
                continue;
            }

            for i in (1..m.captures.len()).step_by(2) {
                let tag_node = m.captures[i].node;
                let value_node = m.captures[i + 1].node;

                let tag = content[tag_node.start_byte()..tag_node.end_byte()].to_string();
                let value = content[value_node.start_byte()..value_node.end_byte()].to_string();

                if tag == ARTIFACT_ID {
                    artifact_id_opt = Some(value.clone());
                }
                if tag == GROUP_ID {
                    group_id_opt = Some(value.clone());
                }
                if tag == VERSION {
                    version_opt = Some(value.clone());
                }
            }

            if let (Some(group_id), Some(_artifact_id), Some(mut version)) =
                (group_id_opt, artifact_id_opt, version_opt)
            {
                let captures_opt = variable_regex.captures(&version);
                if let Some(caps) = captures_opt {
                    let variable_value = caps
                        .get(1)
                        .map_or("".to_string(), |m| m.as_str().to_string());
                    // println!("var value={}", variable_value);
                    if variables.contains_key(&variable_value) {
                        version.clone_from(variables.get(&variable_value).unwrap())
                    }
                }
                dependencies.push(
                    DependencyBuilder::default()
                        .r#type(DependencyType::Library)
                        .name(group_id.clone())
                        .version(Some(version.to_string()))
                        .purl("purl".to_string())
                        .locations(vec![])
                        .build()
                        .unwrap(),
                )
            }
        }

        Ok(dependencies)
    }

    fn find_dependencies_from_file(
        &self,
        path: &PathBuf,
        configuration: &SbomProducerConfiguration,
        context: &MavenProducerContext,
    ) -> anyhow::Result<Vec<Dependency>> {
        let file_content = fs::read_to_string(path);
        if let Ok(content) = file_content {
            if let Some(t) = get_tree(content.as_str(), &context.language) {
                self.find_dependencies_from_tree(&t, content.as_str(), configuration, context)
            } else {
                if configuration.use_debug {
                    eprintln!("cannot build tree for file {}", path.display());
                }
                Ok(vec![])
            }
        } else {
            if configuration.use_debug {
                eprintln!("cannot read file {}", path.display());
            }
            Ok(vec![])
        }
    }
}

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
        if configuration.use_debug {
            for p in paths.iter() {
                println!("paths: {}", p.to_str().unwrap_or(""))
            }
        }

        let xml_language = get_tree_sitter_xml();

        let context = MavenProducerContext {
            query_variables: tree_sitter::Query::new(&xml_language, TREE_SITTER_QUERY_VARIABLES)
                .expect("got query variables"),
            query_dependencies: tree_sitter::Query::new(
                &xml_language,
                TREE_SITTER_QUERY_DEPENDENCIES,
            )
            .expect("got query dependencies"),
            language: xml_language,
        };

        for p in paths.iter() {
            let dependencies = self.find_dependencies_from_file(p, configuration, &context);
            if let Ok(deps) = dependencies {
                result.extend(deps);
            }
        }

        anyhow::Ok(result)
    }
}
