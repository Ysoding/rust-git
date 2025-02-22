use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use walkdir::WalkDir;

use crate::{
    check_ignore, check_ignore_path, gitignore_read, index_read, object_find, object_hash,
    object_read, repo_file, repo_find, Index, Repository, Tree,
};

pub fn status() -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();
    let index = index_read(&repo)?;
    status_branch(&repo)?;
    status_head_index(&repo, &index)?;
    println!();
    status_index_worktree(&repo, &index)?;
    Ok(())
}

pub fn branch_get_active(repo: &Repository) -> Result<Option<String>> {
    let head_path = repo_file(&repo, PathBuf::from("HEAD"), false)?;
    let content = fs::read_to_string(head_path)?;
    if let Some(strip) = content.strip_prefix("ref: refs/heads/") {
        Ok(Some(strip.trim().to_string()))
    } else {
        Ok(None)
    }
}

pub fn status_branch(repo: &Repository) -> Result<()> {
    if let Some(branch) = branch_get_active(repo)? {
        println!("On branch {}.", branch);
    } else {
        let head_sha = object_find(repo, "HEAD", None, true)?.unwrap();
        println!("HEAD detached at {}", head_sha);
    }
    Ok(())
}

pub fn tree_to_dict(
    repo: &Repository,
    tree_ref: &str,
    prefix: &str,
) -> Result<HashMap<String, String>> {
    let mut ret = HashMap::new();

    let tree_sha = object_find(repo, tree_ref, Some(b"tree"), true)?.unwrap();
    let obj = object_read(repo, &tree_sha)?;
    let tree = obj
        .as_any()
        .downcast_ref::<Tree>()
        .ok_or_else(|| anyhow!("Not a tree object"))?;

    for leaf in &tree.items {
        let full_path = if prefix.is_empty() {
            leaf.path.clone()
        } else {
            format!("{}/{}", prefix, leaf.path)
        };
        // If the mode starts with "04", treat it as a subtree.
        if leaf.mode.starts_with(b"04") {
            let sub = tree_to_dict(repo, &leaf.sha, &full_path)?;
            ret.extend(sub);
        } else {
            ret.insert(full_path, leaf.sha.clone());
        }
    }
    Ok(ret)
}

pub fn status_head_index(repo: &Repository, index: &Index) -> Result<()> {
    println!("Changes to be committed:");
    let head = tree_to_dict(repo, "HEAD", "")?;

    let mut head_map = head.clone();
    for entry in &index.entries {
        if head_map.contains_key(&entry.name) {
            if head_map[&entry.name] != entry.sha {
                println!("  modified:    {}", entry.name);
            }
            head_map.remove(&entry.name);
        } else {
            println!("  added:       {}", entry.name);
        }
    }

    for name in head_map.keys() {
        println!("  deleted:     {}", name);
    }
    Ok(())
}

pub fn status_index_worktree(repo: &Repository, index: &Index) -> Result<()> {
    println!("Changes not staged for commit:");

    let ignore = gitignore_read(repo)?;

    let mut all_files = Vec::new();

    // Walk the worktree (excluding .git)
    for entry in WalkDir::new(&repo.worktree) {
        let entry = entry?;
        let path = entry.path();
        if path.starts_with(&repo.gitdir) {
            continue;
        }
        if path.is_file() {
            let rel = path
                .strip_prefix(&repo.worktree)?
                .to_string_lossy()
                .to_string();
            all_files.push(rel);
        }
    }

    use std::os::unix::fs::MetadataExt;
    for entry in &index.entries {
        let full_path = repo.worktree.join(&entry.name);
        if !full_path.exists() {
            println!("  deleted:     {}", entry.name);
        } else {
            let metadata = fs::metadata(&full_path)?;
            let file_ctime_ns =
                metadata.ctime() as u64 * 1_000_000_000 + metadata.ctime_nsec() as u64;
            let file_mtime_ns =
                metadata.mtime() as u64 * 1_000_000_000 + metadata.mtime_nsec() as u64;
            let index_ctime = entry.ctime.0 as u64 * 1_000_000_000 + entry.ctime.1 as u64;
            let index_mtime = entry.mtime.0 as u64 * 1_000_000_000 + entry.mtime.1 as u64;
            if file_ctime_ns != index_ctime || file_mtime_ns != index_mtime {
                let mut f = File::open(&full_path)?;
                let new_sha = object_hash(&mut f, b"blob", None)?;
                if new_sha != entry.sha {
                    println!("  modified:    {}", entry.name);
                }
            }
        }
        all_files.retain(|f| f != &entry.name);
    }
    println!();
    println!("Untracked files:");
    for f in all_files {
        if !check_ignore_path(&ignore, &PathBuf::from(&f)) {
            println!("  {}", f);
        }
    }
    Ok(())
}
