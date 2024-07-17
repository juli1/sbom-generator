pub fn get_tree(
    code: &str,
    tree_sitter_language: &tree_sitter::Language,
) -> Option<tree_sitter::Tree> {
    let mut tree_sitter_parser = tree_sitter::Parser::new();
    tree_sitter_parser.set_language(tree_sitter_language).ok()?;
    tree_sitter_parser.parse(code, None)
}
