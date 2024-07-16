use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Executes a [`Command`], returning true if the command finished with exit status 0, otherwise false
fn run<F>(name: &str, mut configure: F) -> bool
where
    F: FnMut(&mut Command) -> &mut Command,
{
    let mut command = Command::new(name);
    let configured = configure(&mut command);
    println!("Running {configured:?}");
    configured
        .status()
        .unwrap_or_else(|_| panic!("failed to execute {configured:?}"))
        .success()
}

fn main() {
    struct TreeSitterProject {
        /// The directory where we clone the project
        name: String,
        /// The name of the unit we compile
        compilation_unit: String,
        /// The git repository to clone
        repository: String,
        /// The git commit hash that will be passed to `git checkout`
        commit_hash: String,
        /// The directory we use to build the tree-sitter project
        build_dir: PathBuf,
        /// The files to pass to the `cc::Build` instance
        files: Vec<String>,
        /// Whether compilation of this project requires C++ support or not
        cpp: bool,
    }

    fn compile_project(tree_sitter_project: &TreeSitterProject) {
        let dir = &tree_sitter_project.build_dir;
        let files: Vec<PathBuf> = tree_sitter_project
            .files
            .iter()
            .map(|x| dir.join(x))
            .collect();
        let cpp = tree_sitter_project.cpp;
        cc::Build::new()
            .include(dir)
            .files(files)
            .warnings(false)
            .cpp(cpp)
            .compile(tree_sitter_project.compilation_unit.as_str());
    }

    let tree_sitter_projects: Vec<TreeSitterProject> = vec![
        TreeSitterProject {
            name: "tree-sitter-xml".to_string(),
            compilation_unit: "tree-sitter-xml".to_string(),
            repository: "https://github.com/dorgnarg/tree-sitter-xml.git".to_string(),
            commit_hash: "36dd54f701e3b3e030412a854295af971cf74ad1".to_string(),
            build_dir: "src".into(),
            files: vec!["parser.c".to_string(), "binding.cc".to_string()],
            cpp: true,
        },
        TreeSitterProject {
            name: "tree-sitter-json".to_string(),
            compilation_unit: "tree-sitter-json".to_string(),
            repository: "https://github.com/tree-sitter/tree-sitter-json.git".to_string(),
            commit_hash: "3fef30de8aee74600f25ec2e319b62a1a870d51e".to_string(),
            build_dir: "src".into(),
            files: vec!["parser.c".to_string()],
            cpp: false,
        },
        TreeSitterProject {
            name: "tree-sitter-yaml".to_string(),
            compilation_unit: "tree-sitter-yaml-parser".to_string(),
            repository: "https://github.com/tree-sitter-grammars/tree-sitter-yaml.git".to_string(),
            build_dir: "src".into(),
            commit_hash: "ee093118211be521742b9866a8ed8ce6d87c7a94".to_string(),
            files: vec!["parser.c".to_string(), "scanner.c".to_string()],
            cpp: false,
        },
    ];

    // For each project:
    // 1. Check if the source is already present in the folder. It not, fetch it at the specified hash via git
    // 2. Build the project
    let base_dir = env::current_dir().unwrap();
    for proj in &tree_sitter_projects {
        let project_dir = format!(".vendor/{}@{}", &proj.name, &proj.commit_hash);
        if !Path::new(&project_dir).exists() {
            assert!(run("mkdir", |cmd| { cmd.args(["-p", &project_dir]) }));
            env::set_current_dir(&project_dir).unwrap();
            assert!(run("git", |cmd| { cmd.args(["init", "-q"]) }));
            assert!(run("git", |cmd| {
                cmd.args(["remote", "add", "origin", &proj.repository])
            }));
            assert!({
                let mut ok = false;
                let mut retry_time = std::time::Duration::from_secs(1);
                for _ in 0..5 {
                    ok = run("git", |cmd| {
                        cmd.args(["fetch", "-q", "--depth", "1", "origin", &proj.commit_hash])
                    });
                    if ok {
                        break;
                    }
                    std::thread::sleep(retry_time);
                    retry_time *= 2; // Exponential backoff
                }
                ok
            });
            assert!(run("git", |cmd| {
                cmd.args(["checkout", "-q", "FETCH_HEAD"])
            }));
            assert!(run("rm", |cmd| { cmd.args(["-rf", ".git"]) }));
            env::set_current_dir(&base_dir).unwrap();
        }
        env::set_current_dir(&project_dir).unwrap();
        compile_project(proj);
        env::set_current_dir(&base_dir).unwrap();
    }
}
