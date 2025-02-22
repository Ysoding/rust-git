use std::{
    any::Any,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Result};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use regex::Regex;
use sha1::{Digest, Sha1};

use crate::{ref_resolve, repo_dir, repo_file, repo_find, Blob, Commit, Repository, Tag};

pub trait Object {
    /// Returns the object type as bytes (e.g. b"blob").
    fn fmt(&self) -> &'static [u8];
    fn serialize(&self) -> Vec<u8>;
    fn as_any(&self) -> &dyn Any;
}

pub fn object_find(
    repo: &Repository,
    name: &str,
    fmt: Option<&[u8]>,
    follow: bool,
) -> Result<Option<String>> {
    let mut shas = object_resolve(repo, name)?;

    if shas.is_empty() {
        bail!("No such reference {}", name);
    }
    if shas.len() > 1 {
        bail!("Ambiguous reference {}: Candidates are: {:?}", name, shas);
    }

    let mut sha = shas.pop().unwrap();
    if fmt.is_none() {
        return Ok(Some(sha));
    }
    let fmt = fmt.unwrap();

    loop {
        let obj = object_read(repo, &sha)?;
        if obj.fmt() == fmt {
            return Ok(Some(sha));
        }

        if !follow {
            return Ok(None);
        }

        if obj.fmt() == b"tag" {
            let tag_obj = obj
                .as_any()
                .downcast_ref::<Tag>()
                .ok_or_else(|| anyhow!("Tag object not implemented properly"))?;

            let tag_sha = tag_obj
                .kvlm
                .get(&Some(b"object".to_vec()))
                .and_then(|v| v.first())
                .and_then(|val| String::from_utf8(val.clone()).ok())
                .ok_or_else(|| anyhow!("Tag missing object field"))?;

            sha = tag_sha;
        } else if obj.fmt() == b"commit" && fmt == b"tree" {
            let commit_obj = obj
                .as_any()
                .downcast_ref::<Commit>()
                .ok_or_else(|| anyhow!("Not a commit"))?;

            let tree_sha = commit_obj
                .kvlm
                .get(&Some(b"tree".to_vec()))
                .and_then(|v| v.first())
                .and_then(|val| String::from_utf8(val.clone()).ok())
                .ok_or_else(|| anyhow!("Commit missing tree field"))?;

            sha = tree_sha;
        } else {
            return Ok(None);
        }
    }
}

pub fn object_read(repo: &Repository, sha: &str) -> Result<Box<dyn Object>> {
    // e.g. .git/objects/e6/73d1b7eaa0aa01b5bc2442d570a765bdaae751
    let dir = &sha[0..2];
    let file = &sha[2..];
    let object_path = repo_file(repo, PathBuf::from("objects").join(dir).join(file), false)?;
    if !object_path.is_file() {
        bail!("Object {} does not exist", sha);
    }

    let compressed = fs::read(&object_path)?;
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut raw = Vec::new();
    decoder.read_to_end(&mut raw)?;

    // b"<type> <size>\x00<data>"

    // read object type
    let space_pos = raw
        .iter()
        .position(|&b| b == b' ')
        .ok_or_else(|| anyhow!("Malformed object header"))?;
    let fmt = &raw[0..space_pos];

    // read size
    let null_pos = raw
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("Malformed object header"))?;
    let size_str = std::str::from_utf8(&raw[space_pos + 1..null_pos])?;
    let size: usize = size_str.parse()?;
    if size != raw.len() - null_pos - 1 {
        bail!("Malformed object {}: bad length", sha);
    }

    let data = &raw[null_pos + 1..];

    match fmt {
        b"commit" => bail!("commit type not implemented"),
        b"tree" => bail!("tree type not implemented"),
        b"tag" => bail!("tag type not implemented"),
        b"blob" => Ok(Box::new(Blob::deserialize(data))),
        _ => bail!("Unknown object type: {}", std::str::from_utf8(fmt)?),
    }
}

pub fn object_write(obj: &dyn Object, repo: Option<&Repository>) -> Result<String> {
    let data = obj.serialize();
    let header = format!("{} {}", std::str::from_utf8(obj.fmt())?, data.len());

    let mut store = Vec::new();
    store.extend_from_slice(header.as_bytes());
    store.push(0);
    store.extend_from_slice(&data);

    let mut hasher = Sha1::new();
    hasher.update(&store);
    let sha = hex::encode(hasher.finalize());

    if let Some(repo) = repo {
        let dir = &sha[0..2];
        let file = &sha[2..];
        let object_path = repo_file(repo, PathBuf::from("objects").join(dir).join(file), true)?;
        if !object_path.exists() {
            let f = fs::File::create(object_path)?;
            let mut encoder = ZlibEncoder::new(f, Compression::default());
            encoder.write_all(&store)?;
            encoder.finish()?;
        }
    }

    Ok(sha)
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

pub fn object_resolve(repo: &Repository, name: &str) -> Result<Vec<String>> {
    let mut candidates = Vec::new();

    if name.is_empty() {
        return Ok(candidates);
    }

    if name == "HEAD" {
        if let Some(sha) = ref_resolve(repo, "HEAD")? {
            candidates.push(sha);
        }
        return Ok(candidates);
    }

    let hash_re = Regex::new(r"^[0-9A-Fa-f]{4,40}$").unwrap();
    if hash_re.is_match(name) {
        let lower = name.to_lowercase();
        let prefix = &lower[0..2];
        let objects_dir = repo_dir(repo, PathBuf::from("objects"), false)?
            .unwrap()
            .join(prefix);
        if objects_dir.is_dir() {
            for entry in fs::read_dir(objects_dir)? {
                let entry = entry?;
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with(&lower[2..]) {
                    candidates.push(format!("{}{}", prefix, fname));
                }
            }
        }
    }

    // Try tags.
    if let Some(tag_sha) = ref_resolve(repo, &format!("refs/tags/{}", name))? {
        candidates.push(tag_sha);
    }

    // Try branches.
    if let Some(branch_sha) = ref_resolve(repo, &format!("refs/heads/{}", name))? {
        candidates.push(branch_sha);
    }

    Ok(candidates)
}
