//! Project manifest discovery and entry point resolution.
//!
//! Loads `txtcode.toml` (or the legacy `Txtcode.toml`) from the project root,
//! and resolves the source entry file from either a direct path or a manifest.

use std::path::{Path, PathBuf};

/// Contents of `[project]` in `txtcode.toml`.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub entry: PathBuf,
    pub backend: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "unnamed".to_string(),
            entry: PathBuf::from("src/main.tc"),
            backend: "ast".to_string(),
        }
    }
}

impl ProjectConfig {
    /// Walk upward from `start` looking for `txtcode.toml` or `Txtcode.toml`.
    /// Returns `None` if no manifest is found before the filesystem root.
    pub fn discover_near(start: &Path) -> Option<(Self, PathBuf)> {
        let mut dir = if start.is_file() {
            start.parent()?.to_path_buf()
        } else {
            start.to_path_buf()
        };

        loop {
            for name in &["txtcode.toml", "Txtcode.toml"] {
                let candidate = dir.join(name);
                if candidate.exists() {
                    if let Ok(cfg) = Self::load_from(&candidate) {
                        return Some((cfg, dir));
                    }
                }
            }
            match dir.parent() {
                Some(p) if p != dir => dir = p.to_path_buf(),
                _ => return None,
            }
        }
    }

    /// Parse a `txtcode.toml` file.
    fn load_from(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        Self::parse_toml(&text)
    }

    fn parse_toml(text: &str) -> Result<Self, String> {
        let mut name = "unnamed".to_string();
        let mut entry = "src/main.tc".to_string();
        let mut backend = "ast".to_string();
        let mut in_project = false;

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line == "[project]" {
                in_project = true;
                continue;
            }
            if line.starts_with('[') {
                in_project = false;
                continue;
            }
            if !in_project {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                let val = v.trim().trim_matches('"').to_string();
                match key {
                    "name" => name = val,
                    "entry" => entry = val,
                    "backend" => backend = val,
                    _ => {}
                }
            }
        }

        Ok(Self {
            name,
            entry: PathBuf::from(entry),
            backend,
        })
    }

    /// Resolve the source entry file given a CLI input path.
    ///
    /// Rules:
    /// - If `input` is an existing `.tc` or `.txtc` file, use it directly.
    /// - If `input` is a directory, look for a `txtcode.toml` and use `project.entry`.
    /// - Otherwise return an error.
    pub fn resolve_entry(input: &Path) -> Result<PathBuf, String> {
        if input.is_file() {
            return Ok(input.to_path_buf());
        }

        if input.is_dir() {
            if let Some((cfg, root)) = Self::discover_near(input) {
                let abs = root.join(&cfg.entry);
                if abs.exists() {
                    return Ok(abs);
                }
                return Err(format!(
                    "Project entry '{}' listed in txtcode.toml does not exist",
                    cfg.entry.display()
                ));
            }
            // No manifest — look for src/main.tc as a convention fallback.
            let fallback = input.join("src/main.tc");
            if fallback.exists() {
                return Ok(fallback);
            }
            return Err(format!(
                "'{}' is a directory with no txtcode.toml and no src/main.tc",
                input.display()
            ));
        }

        Err(format!("'{}' does not exist", input.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_toml() {
        let toml = r#"
[project]
name = "hello"
entry = "src/main.tc"
backend = "ast"
"#;
        let cfg = ProjectConfig::parse_toml(toml).unwrap();
        assert_eq!(cfg.name, "hello");
        assert_eq!(cfg.entry, PathBuf::from("src/main.tc"));
        assert_eq!(cfg.backend, "ast");
    }

    #[test]
    fn parse_toml_with_other_sections() {
        let toml = r#"
[project]
name = "myapp"
entry = "app/entry.tc"

[dependencies]
npl-http = "0.1"
"#;
        let cfg = ProjectConfig::parse_toml(toml).unwrap();
        assert_eq!(cfg.name, "myapp");
        assert_eq!(cfg.entry, PathBuf::from("app/entry.tc"));
    }

    #[test]
    fn parse_toml_defaults_when_keys_missing() {
        let cfg = ProjectConfig::parse_toml("[project]\n").unwrap();
        assert_eq!(cfg.name, "unnamed");
        assert_eq!(cfg.backend, "ast");
    }
}
