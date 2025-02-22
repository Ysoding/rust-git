use std::collections::HashSet;
use std::path::Path;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;

use crate::object_find;
use crate::object_read;
use crate::repo_find;
use crate::Commit;
use crate::Repository;
use crate::Tree;

pub fn log(commit: &str) -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();
    println!("digraph wyaglog{{");
    println!("  node[shape=rect]");
    let mut seen = HashSet::new();
    let sha = object_find(&repo, commit, None, false)?.unwrap();
    log_graphviz(&repo, &sha, &mut seen)?;
    println!("}}");
    Ok(())
}

fn log_graphviz(repo: &Repository, sha: &str, seen: &mut HashSet<String>) -> Result<()> {
    if seen.contains(sha) {
        return Ok(());
    }
    seen.insert(sha.to_string());

    let obj = object_read(repo, sha)?;
    let commit = obj
        .as_any()
        .downcast_ref::<Commit>()
        .ok_or_else(|| anyhow!("Object {} is not a commit", sha))?;

    let tmp = Vec::new();
    let msg_bytes = commit
        .kvlm
        .get(&None)
        .and_then(|vecs| vecs.first())
        .unwrap_or(&tmp);

    let mut message = String::from_utf8_lossy(msg_bytes).to_string();
    message = message.trim().to_string();
    if let Some(pos) = message.find('\n') {
        message = message[..pos].to_string();
    }
    let message = message.replace("\\", "\\\\").replace("\"", "\\\"");
    println!("  c_{} [label=\"{}: {}\"];", sha, &sha[..7], message);

    let parent_key = Some(b"parent".to_vec());
    if !commit.kvlm.contains_key(&parent_key) {
        // No parent: initial commit.
        return Ok(());
    }

    let parents = commit.kvlm.get(&parent_key).unwrap();

    for parent in parents {
        let parent_str = String::from_utf8_lossy(parent).to_string();
        println!("  c_{} -> c_{};", sha, parent_str);
        log_graphviz(repo, &parent_str, seen)?;
    }

    Ok(())
}

pub fn ls_tree(tree_ref: &str, recursive: bool) -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();
    ls_tree_inner(&repo, tree_ref, recursive, "")?;
    Ok(())
}

fn ls_tree_inner(repo: &Repository, tree_ref: &str, recursive: bool, prefix: &str) -> Result<()> {
    let sha = object_find(repo, tree_ref, Some(b"tree"), true)?.unwrap();
    let obj = object_read(repo, &sha)?;
    let tree = obj
        .as_any()
        .downcast_ref::<Tree>()
        .ok_or_else(|| anyhow!("Object {} is not a tree", sha))?;

    for item in tree.items.iter() {
        let typ_mode = if item.mode.len() == 5 {
            &item.mode[0..1]
        } else {
            &item.mode[0..2]
        };

        let typ = match typ_mode {
            x if x == b"04" => "tree",
            x if x == b"10" || x == b"12" => "blob",
            x if x == b"16" => "commit",
            _ => bail!(
                "Weird tree leaf mode {:?}",
                String::from_utf8_lossy(&item.mode)
            ),
        };

        if !recursive || typ != "tree" {
            // Pad mode to six characters if needed.
            let mode_str = String::from_utf8(item.mode.clone()).unwrap();
            let padded_mode = if mode_str.len() < 6 {
                format!("{:0>6}", mode_str)
            } else {
                mode_str
            };
            let full_path = if prefix.is_empty() {
                item.path.clone()
            } else {
                format!("{}/{}", prefix, item.path)
            };
            println!("{} {} {}\t{}", padded_mode, typ, item.sha, full_path);
        } else {
            let prefix = if prefix.is_empty() {
                item.path.clone()
            } else {
                format!("{}/{}", prefix, item.path)
            };
            ls_tree_inner(repo, &item.sha, recursive, &prefix)?;
        }
    }
    Ok(())
}
