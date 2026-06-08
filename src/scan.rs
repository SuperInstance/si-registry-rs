use crate::types::{Capability, LocalRepo, Result};
use std::fs;
use std::path::Path;

/// Capabilities manifest parsed from a CAPABILITY.toml file.
#[derive(Debug, serde::Deserialize)]
struct CapabilityManifest {
    #[serde(default)]
    capabilities: Vec<CapabilityEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct CapabilityEntry {
    name: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    provides: String,
    #[serde(default)]
    description: String,
}

/// Scan a directory for subdirectories containing a `CAPABILITY.toml` file.
///
/// Each subdirectory with a `CAPABILITY.toml` is treated as a local repo.
/// The manifest is parsed and its capabilities are collected.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
pub fn scan_dir(path: &str) -> Result<Vec<LocalRepo>> {
    let root = Path::new(path);
    if !root.is_dir() {
        return Ok(vec![]);
    }

    let mut repos = Vec::new();

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let subpath = entry.path();
        if !subpath.is_dir() {
            continue;
        }

        let cap_file = subpath.join("CAPABILITY.toml");
        if !cap_file.exists() {
            continue;
        }

        let dir_name = subpath
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let capabilities = match parse_capability_toml(&cap_file) {
            Ok(caps) => caps,
            Err(e) => {
                eprintln!(
                    "Warning: failed to parse {}: {}",
                    cap_file.display(),
                    e
                );
                continue;
            }
        };

        repos.push(LocalRepo {
            path: subpath.to_string_lossy().to_string(),
            name: dir_name,
            capabilities,
        });
    }

    Ok(repos)
}

/// Parse a single CAPABILITY.toml file into a list of capabilities.
pub fn parse_capability_toml(path: &Path) -> Result<Vec<Capability>> {
    let content = fs::read_to_string(path)?;
    let manifest: CapabilityManifest = toml::from_str(&content)?;
    Ok(manifest
        .capabilities
        .into_iter()
        .map(|c| Capability {
            name: c.name,
            category: c.category,
            provides: c.provides,
            description: c.description,
        })
        .collect())
}

/// Extract all unique capabilities from a list of local repos.
pub fn all_capabilities(repos: &[LocalRepo]) -> Vec<Capability> {
    let mut seen = std::collections::HashSet::new();
    let mut caps = Vec::new();
    for repo in repos {
        for cap in &repo.capabilities {
            let key = format!("{}:{}", cap.name, cap.category);
            if seen.insert(key) {
                caps.push(cap.clone());
            }
        }
    }
    caps
}

/// Find repos that provide a specific capability by name.
pub fn find_by_capability<'a>(repos: &'a [LocalRepo], capability_name: &str) -> Vec<&'a LocalRepo> {
    repos
        .iter()
        .filter(|r| {
            r.capabilities
                .iter()
                .any(|c| c.name == capability_name)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_scan_dir(base: &std::path::Path) {
        // repo1 with capabilities
        let r1 = base.join("repo-alpha");
        fs::create_dir_all(&r1).unwrap();
        fs::write(
            r1.join("CAPABILITY.toml"),
            r#"
[[capabilities]]
name = "compute"
category = "ml"
provides = "inference"
description = "ML inference engine"
"#,
        )
        .unwrap();

        // repo2 with capabilities
        let r2 = base.join("repo-beta");
        fs::create_dir_all(&r2).unwrap();
        fs::write(
            r2.join("CAPABILITY.toml"),
            r#"
[[capabilities]]
name = "storage"
category = "data"
provides = "persistence"
description = "Persistent storage"

[[capabilities]]
name = "cache"
category = "data"
provides = "caching"
description = "In-memory cache"
"#,
        )
        .unwrap();

        // repo3 without CAPABILITY.toml (should be skipped)
        let r3 = base.join("repo-gamma");
        fs::create_dir_all(&r3).unwrap();

        // A file (not a dir, should be skipped)
        fs::write(base.join("some-file.txt"), "hello").unwrap();
    }

    #[test]
    fn test_scan_dir_finds_repos() {
        let tmp = tempfile::tempdir().unwrap();
        setup_scan_dir(tmp.path());
        let repos = scan_dir(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(repos.len(), 2);

        let names: Vec<&str> = repos.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"repo-alpha"));
        assert!(names.contains(&"repo-beta"));
    }

    #[test]
    fn test_scan_dir_capabilities() {
        let tmp = tempfile::tempdir().unwrap();
        setup_scan_dir(tmp.path());
        let repos = scan_dir(tmp.path().to_str().unwrap()).unwrap();

        let alpha = repos.iter().find(|r| r.name == "repo-alpha").unwrap();
        assert_eq!(alpha.capabilities.len(), 1);
        assert_eq!(alpha.capabilities[0].name, "compute");

        let beta = repos.iter().find(|r| r.name == "repo-beta").unwrap();
        assert_eq!(beta.capabilities.len(), 2);
    }

    #[test]
    fn test_scan_nonexistent_dir() {
        let repos = scan_dir("/tmp/this-does-not-exist-asdf").unwrap();
        // scan_dir returns Ok(vec![]) for non-existent dirs
        assert!(repos.is_empty());
    }

    #[test]
    fn test_all_capabilities_dedup() {
        let tmp = tempfile::tempdir().unwrap();
        setup_scan_dir(tmp.path());
        let repos = scan_dir(tmp.path().to_str().unwrap()).unwrap();
        let caps = all_capabilities(&repos);
        assert_eq!(caps.len(), 3); // compute, storage, cache
    }

    #[test]
    fn test_find_by_capability() {
        let tmp = tempfile::tempdir().unwrap();
        setup_scan_dir(tmp.path());
        let repos = scan_dir(tmp.path().to_str().unwrap()).unwrap();
        let found = find_by_capability(&repos, "compute");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "repo-alpha");
    }

    #[test]
    fn test_find_by_capability_none() {
        let tmp = tempfile::tempdir().unwrap();
        setup_scan_dir(tmp.path());
        let repos = scan_dir(tmp.path().to_str().unwrap()).unwrap();
        let found = find_by_capability(&repos, "nonexistent");
        assert!(found.is_empty());
    }

    #[test]
    fn test_invalid_toml_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let r1 = tmp.path().join("bad-repo");
        fs::create_dir_all(&r1).unwrap();
        fs::write(r1.join("CAPABILITY.toml"), "this is not valid toml [[[[").unwrap();
        let repos = scan_dir(tmp.path().to_str().unwrap()).unwrap();
        assert!(repos.is_empty()); // bad toml skipped, no crash
    }
}
