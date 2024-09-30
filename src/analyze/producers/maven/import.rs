/// if
/// group_id is io.quarkus.platform
/// artifact_id is quarkus-bom
/// version is 3.14.1
/// url will be https://repo1.maven.org/maven2/io/quarkus/platform/quarkus-bom/3.14.1/quarkus-bom-3.14.1.pom
fn get_url(group_id: &str, artifact_id: &str, version: &str) -> String {
    let parts = group_id.replace(".", "/");
    format!(
        "https://repo1.maven.org/maven2/{}/{}/{}/{}-{}.pom",
        parts, artifact_id, version, artifact_id, version
    )
}

pub fn get_pom_content(group_id: &str, artifact_id: &str, version: &str) -> anyhow::Result<String> {
    let resp = reqwest::blocking::get(get_url(group_id, artifact_id, version))?.text()?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_url() {
        assert_eq!(get_url("io.quarkus.platform", "quarkus-bom", "3.14.1"), "https://repo1.maven.org/maven2/io/quarkus/platform/quarkus-bom/3.14.1/quarkus-bom-3.14.1.pom");
    }

    #[test]
    fn test_get_pom_content() {
        let pom_content = get_pom_content("io.quarkus.platform", "quarkus-bom", "3.14.1");
        assert!(pom_content.is_ok());
    }
}
