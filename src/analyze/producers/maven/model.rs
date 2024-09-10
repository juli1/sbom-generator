use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub enum MavenDependencyType {
    Pom
}

impl FromStr for MavenDependencyType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pom" => Ok(MavenDependencyType::Pom),
            _ => Err(()),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum MavenDependencyScope {
    Import,
    Test,
    Provided,
}

impl FromStr for MavenDependencyScope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "import" => Ok(MavenDependencyScope::Import),
            "test" => Ok(MavenDependencyScope::Test),
            "provided" => Ok(MavenDependencyScope::Provided),
            _ => Err(()),
        }
    }
}