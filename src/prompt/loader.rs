use regex::Regex;
use std::fs;
use std::path::PathBuf;

/// Load a template file from disk
pub fn load_template(template_dir: &PathBuf, name: &str) -> Result<String, String> {
    let path = template_dir.join(format!("{}.md", name));
    fs::read_to_string(&path)
        .map_err(|e| format!("Template '{}' not found at {:?}: {}", name, path, e))
}

/// Render {{variable}} placeholders in a template
pub fn render_template(template: &str, bindings: &[(&str, &str)]) -> String {
    let re = Regex::new(r"\{\{(.+?)\}\}").unwrap();
    re.replace_all(template, |caps: &regex::Captures| {
        let var = caps[1].trim();
        bindings.iter()
            .find(|(k, _)| *k == var)
            .map(|(_, v)| v.to_string())
            .unwrap_or_else(|| format!("{{{{{}}}}}", var))
    }).to_string()
}
