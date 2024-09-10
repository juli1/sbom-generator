use crate::model::dependency::Dependency;
use crate::utils::tree_sitter::language::get_tree_sitter_xml;
use std::collections::HashMap;

const TREE_SITTER_PARENT_INFORMATION: &str = r###"
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
                  (Name) @relativePath
               )
               (content) @pathValue
            )

         )
      )
   )
   (#eq? @project "project")
   (#eq? @parent "parent")
   (#eq? @relativePath "relativePath")
)
"###;

const TREE_SITTER_PROJECT_VERSION: &str = r###"
(element
   (STag
      (Name) @project
   )
   (content
      (element
         (STag
            (Name) @version
         )
         (content) @value
      )
   )
   (#eq? @project "project")
   (#eq? @version "version")
)
"###;

const TREE_SITTER_PROJECT_PROPERTIES: &str = r###"
(element
   (STag
      (Name) @project
   )
   (content
      (element
         (STag
            (Name) @properties
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
   (#eq? @properties "properties")
)
"###;

const TREE_SITTER_QUERY_DEPENDENCY_MANAGEMENT: &str = r###"
(element
   (STag
      (Name)@project
   )
   (content
        (element
            (STag
                (Name)@dependencyManagement
            )
            (content
                (element
                    (STag
                        (Name) @dependencies
                    )
                    (content
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
                        )@element
                    )
                    (#eq? @dependencies "dependencies")
                )
            )
        )
   )
   (#eq? @dependencyManagement "dependencyManagement")
   (#eq? @project "project")
)
"###;

const TREE_SITTER_QUERY_DEPENDENCIES: &str = r###"
(element
   (STag
      (Name)@project
   )
   (content
        (element
            (STag
                (Name) @dependencies
            )
            (content
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
                )@element
            )
            (#eq? @dependencies "dependencies")
        )
   )
   (#eq? @project "project")
)
"###;

/// Context everything you need to write your SBOM. This includes all the tree-sitter
/// queries but also all the files that were parsed. This way, if we need to resolve
/// variables post-processing, we can use them using this structure.
pub struct MavenProducerContext {
    pub query_project_version: tree_sitter::Query,
    pub query_project_properties: tree_sitter::Query,
    pub query_parent_information: tree_sitter::Query,
    pub query_dependencies: tree_sitter::Query,
    pub query_dependency_management: tree_sitter::Query,
    pub language: tree_sitter::Language,
    pub files_information: HashMap<String, HashMap<String, Dependency>>,
}

impl MavenProducerContext {
    pub fn new() -> Self {
        let xml_language = get_tree_sitter_xml();

        MavenProducerContext {
            query_project_version: tree_sitter::Query::new(&xml_language, TREE_SITTER_PROJECT_VERSION)
                .expect("got query variables"),
            query_dependencies: tree_sitter::Query::new(
                &xml_language,
                TREE_SITTER_QUERY_DEPENDENCIES,
            ).expect("got query dependencies"),
            query_dependency_management: tree_sitter::Query::new(
                &xml_language,
                TREE_SITTER_QUERY_DEPENDENCY_MANAGEMENT,
            ).expect("got query dependency management"),
            query_project_properties: tree_sitter::Query::new(
                &xml_language,
                TREE_SITTER_PROJECT_PROPERTIES,
            ).expect("got query project properties"),
            query_parent_information: tree_sitter::Query::new(
                &xml_language,
                TREE_SITTER_PARENT_INFORMATION,
            ).expect("got query parent info"),
            language: xml_language,
            files_information: HashMap::new(),
        }
    }
}
