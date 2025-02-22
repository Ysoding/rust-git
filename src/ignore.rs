use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{index_read, object_read, repo_find, Blob, Repository};

pub fn check_ignore(paths: &Vec<PathBuf>) -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();
    let rules = gitignore_read(&repo)?;
    for path in paths {
        if check_ignore_path(&rules, path) {
            println!("{}", path.to_str().unwrap());
        }
    }

    Ok(())
}

pub struct Ignore {
    pub absolute: Vec<(String, bool)>, // (pattern, include?) — true means ignore
    pub scoped: HashMap<String, Vec<(String, bool)>>,
}

pub fn check_ignore_path(ignore: &Ignore, path: &Path) -> bool {
    if path.is_absolute() {
        panic!("check_ignore requires a path relative to repository root");
    }

    if let Some(result) = check_ignore_scoped(&ignore.scoped, path) {
        return result;
    }
    check_ignore_absolute(&ignore.absolute, path)
}

pub fn gitignore_read(repo: &Repository) -> Result<Ignore> {
    let mut absolute = Vec::new();
    let mut scoped = HashMap::new();

    let repo_exclude = repo.gitdir.join("info").join("exclude");
    if repo_exclude.exists() {
        let content = fs::read_to_string(&repo_exclude)?;
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        absolute.extend(gitignore_parse(lines));
    }

    let global_ignore = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("git").join("ignore")
    } else {
        PathBuf::from("~/.config").join("git").join("ignore")
    };

    if global_ignore.exists() {
        let content = fs::read_to_string(&global_ignore)?;
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        absolute.extend(gitignore_parse(lines));
    }

    let index = index_read(repo)?;

    for entry in index.entries {
        if entry.name == ".gitignore" || entry.name.ends_with("/.gitignore") {
            let obj = object_read(repo, &entry.sha)?;
            let blob = obj
                .as_any()
                .downcast_ref::<Blob>()
                .ok_or_else(|| anyhow!(".gitignore is not a blob"))?;
            let content = String::from_utf8(blob.blobdata.clone())?;

            let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

            let dir = std::path::Path::new(&entry.name)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            scoped.insert(dir, gitignore_parse(lines));
        }
    }

    Ok(Ignore { absolute, scoped })
}

fn gitignore_parse1(raw: &str) -> Option<(String, bool)> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with('#') {
        None
    } else if let Some(r) = raw.strip_prefix('!') {
        Some((r.to_string(), false))
    } else if let Some(r) = raw.strip_prefix('\\') {
        Some((r.to_string(), true))
    } else {
        Some((raw.to_string(), true))
    }
}

fn gitignore_parse(lines: Vec<String>) -> Vec<(String, bool)> {
    lines.iter().filter_map(|l| gitignore_parse1(l)).collect()
}

pub fn check_ignore1(rules: &[(String, bool)], path: &Path) -> Option<bool> {
    let mut result = None;
    for (pattern, include) in rules {
        if let Ok(glob_pat) = glob::Pattern::new(pattern) {
            if glob_pat.matches(path.to_string_lossy().as_ref()) {
                result = Some(*include);
            }
        }
    }
    result
}

pub fn check_ignore_scoped(
    scoped: &HashMap<String, Vec<(String, bool)>>,
    path: &Path,
) -> Option<bool> {
    let mut current = path;
    while let Some(parent) = current.parent() {
        let parent_str = parent.to_string_lossy().to_string();
        if let Some(rules) = scoped.get(&parent_str) {
            if let Some(result) = check_ignore1(rules, path) {
                return Some(result);
            }
        }
        current = parent;
    }
    None
}

pub fn check_ignore_absolute(rules: &[(String, bool)], path: &Path) -> bool {
    check_ignore1(rules, path).unwrap_or(false)
}
