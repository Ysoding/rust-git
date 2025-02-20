mod cli;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};
pub use cli::*;
mod repository;
pub use repository::*;
mod object;
pub use object::*;
mod commit;
pub use commit::*;
mod blob;
pub use blob::*;
mod tree;
pub use tree::*;

pub fn cat_file(object: &str, fmt: Option<&[u8]>) -> Result<()> {
    let repo = repo_find(Path::new("."), true).unwrap().unwrap();
    let sha = object_find(&repo, object, fmt, true)?;
    let obj = object_read(&repo, &sha)?;
    std::io::stdout().write_all(&obj.serialize())?;
    Ok(())
}

pub fn hash_object(path: &PathBuf, fmt: &[u8], write: bool) -> Result<String> {
    let repo = if write {
        repo_find(Path::new("."), true)?
    } else {
        None
    };

    let file = File::open(path)?;
    object_hash(file, fmt, repo.as_ref())
}

pub fn object_hash<R: Read>(
    mut reader: R,
    fmt: &[u8],
    repo: Option<&Repository>,
) -> Result<String> {
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;

    let obj: Box<dyn Object> = match fmt {
        b"blob" => Box::new(Blob::deserialize(&data)),
        b"commit" => bail!("commit type not implemented"),
        b"tree" => bail!("tree type not implemented"),
        b"tag" => bail!("tag type not implemented"),
        _ => bail!("Unknown object type: {}", std::str::from_utf8(fmt)?),
    };

    object_write(obj.as_ref(), repo)
}

pub fn log(commit: &str) -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();
    println!("digraph wyaglog{{");
    println!("  node[shape=rect]");
    let mut seen = HashSet::new();
    let sha = &object_find(&repo, commit, None, false)?;
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

    let mut message = String::from_utf8_lossy(&msg_bytes).to_string();
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
    let sha = object_find(repo, tree_ref, Some(b"tree"), true)?;
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

pub fn checkout(commit: &str, target: &PathBuf) -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();

    let mut obj = object_read(&repo, &object_find(&repo, commit, None, true)?)?;
    if obj.fmt() == b"commit" {
        let commit_obj = obj
            .as_any()
            .downcast_ref::<Commit>()
            .ok_or_else(|| anyhow!("Not a commit object"))?;
        let tree_sha = commit_obj
            .kvlm
            .get(&Some(b"tree".to_vec()))
            .and_then(|v| v.get(0))
            .and_then(|val| String::from_utf8(val.clone()).ok())
            .ok_or_else(|| anyhow!("Commit missing tree field"))?;
        obj = object_read(&repo, &tree_sha)?;
    }

    if target.exists() {
        if !target.is_dir() {
            bail!("Not a directory: {:?}", target);
        }
        if fs::read_dir(target)?.next().is_some() {
            bail!("Directory not empty: {:?}", target);
        }
    } else {
        fs::create_dir_all(target)?;
    }
    tree_checkout(&repo, obj.as_ref(), target)
}

fn tree_checkout(repo: &Repository, tree_obj: &dyn Object, path: &Path) -> Result<()> {
    let tree = tree_obj
        .as_any()
        .downcast_ref::<Tree>()
        .ok_or_else(|| anyhow!("Object is not a tree"))?;

    for item in tree.items.iter() {
        let obj = object_read(repo, &item.sha)?;
        let dest = path.join(&item.path);
        if obj.fmt() == b"tree" {
            fs::create_dir(&dest)?;
            tree_checkout(repo, obj.as_ref(), &dest)?;
        } else if obj.fmt() == b"blob" {
            let blob = obj
                .as_any()
                .downcast_ref::<Blob>()
                .ok_or_else(|| anyhow!("Object {} is not a blob", item.sha))?;

            let mut f = File::create(&dest)?;
            f.write_all(&blob.blobdata)?;
        } else {
            bail!(
                "Unsupported object type in checkout: {}",
                std::str::from_utf8(obj.fmt())?
            );
        }
    }
    Ok(())
}
