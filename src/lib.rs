mod cli;
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
pub use cli::*;
mod repository;
pub use repository::*;
mod object;
pub use object::*;
mod commit;
pub use commit::*;

pub fn cat_file(repo: &Repository, object: &str, fmt: Option<&[u8]>) -> Result<()> {
    let sha = object_find(repo, object, fmt, true);
    let obj = object_read(repo, &sha)?;
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
