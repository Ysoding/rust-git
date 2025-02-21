mod cli;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};
pub use cli::*;
mod repository;
use indexmap::IndexMap;
pub use repository::*;
mod object;
pub use object::*;
mod commit;
pub use commit::*;
mod blob;
pub use blob::*;
mod tree;
pub use tree::*;
mod tag;
pub use tag::*;
mod log;
pub use log::*;

pub fn rev_parse(name: &str, fmt: Option<&[u8]>) -> Result<()> {
    let repo = repo_find(Path::new("."), true).unwrap().unwrap();
    let obj_sha = object_find(&repo, name, fmt, true)?;
    if let Some(sha) = obj_sha {
        println!("{}", sha);
    } else {
        println!("None");
    }
    Ok(())
}

pub fn cat_file(object: &str, fmt: Option<&[u8]>) -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();
    let sha = object_find(&repo, object, fmt, true)?.unwrap();
    let obj = object_read(&repo, &sha)?;
    std::io::stdout().write_all(&obj.serialize())?;
    Ok(())
}

pub fn checkout(commit: &str, target: &PathBuf) -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();

    let mut obj = object_read(&repo, &object_find(&repo, commit, None, true)?.unwrap())?;
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

fn tree_checkout(repo: &Repository, tree_obj: &dyn Object, path: &PathBuf) -> Result<()> {
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

fn ref_resolve(repo: &Repository, refname: &str) -> Result<Option<String>> {
    let path = repo_file(repo, PathBuf::from(refname), false)?;
    if !path.is_file() {
        return Ok(None);
    }

    let data = fs::read_to_string(&path)?;
    let data = data.trim_end();
    if data.starts_with("ref: ") {
        ref_resolve(repo, &data[5..])
    } else {
        Ok(Some(data.to_string()))
    }
}

fn ref_list_flat(
    repo: &Repository,
    path: Option<PathBuf>,
    prefix: Option<&str>,
) -> Result<IndexMap<String, String>> {
    let mut ret = IndexMap::new();
    let path = if path.is_none() {
        repo_dir(&repo, PathBuf::from("refs"), false)?.unwrap()
    } else {
        path.unwrap()
    };
    let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let full_name = match prefix {
            Some(pref) => format!("{}/{}", pref, name),
            None => name,
        };

        if p.is_dir() {
            let sub = ref_list_flat(repo, Some(p), Some(&full_name))?;
            ret.extend(sub);
        } else {
            if let Some(sha) = ref_resolve(repo, &p.to_string_lossy())? {
                ret.insert(full_name, sha);
            }
        }
    }

    Ok(ret)
}

pub fn show_ref() -> Result<()> {
    let repo = repo_find(Path::new("."), true)?.unwrap();
    let refs = ref_list_flat(&repo, None, Some("refs"))?;

    show_ref_print(&refs, true);
    Ok(())
}

fn show_ref_print(refs: &IndexMap<String, String>, with_hash: bool) {
    for (k, _v) in refs {
        if with_hash {
            println!(" {}", k);
        } else {
            println!("{}", k);
        }
    }
}
