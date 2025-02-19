use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::{anyhow, bail, Result};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};

use crate::{repo_file, Repository};

pub trait Object {
    /// Returns the object type as bytes (e.g. b"blob").
    fn fmt(&self) -> &'static [u8];
    fn serialize(&self) -> Vec<u8>;
}

pub fn object_find(repo: &Repository, name: &str, fmt: Option<&[u8]>, _follow: bool) -> String {
    name.to_string()
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

pub struct Blob {
    pub blobdata: Vec<u8>,
}

impl Blob {
    pub fn new(data: &[u8]) -> Self {
        Self {
            blobdata: data.to_vec(),
        }
    }

    pub fn deserialize(data: &[u8]) -> Self {
        Self::new(data)
    }
}

impl Object for Blob {
    fn fmt(&self) -> &'static [u8] {
        b"blob"
    }

    fn serialize(&self) -> Vec<u8> {
        self.blobdata.clone()
    }
}
