use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use anyhow::{anyhow, bail, Result};

use crate::{repo_file, Repository};

/// An entry in the index file holds metadata about a tracked file.
#[derive(Default)]
pub struct IndexEntry {
    pub ctime: (u32, u32), // (seconds, nanoseconds)
    pub mtime: (u32, u32),
    pub dev: u32,
    pub ino: u32,
    pub mode_type: u16, // e.g. 0b1000 (regular), 0b1010 (symlink), 0b1110 (gitlink)
    pub mode_perms: u16, // lower bits (permissions)
    pub uid: u32,
    pub gid: u32,
    pub fsize: u32,
    pub sha: String, // stored as 40-digit lowercase hex
    pub flag_assume_valid: bool,
    pub flag_stage: u16, // bits indicating the stage
    pub name: String,    // path relative to worktree
}

pub struct Index {
    pub version: u32,
    pub entries: Vec<IndexEntry>,
}

impl Default for Index {
    fn default() -> Self {
        Self {
            version: 2,
            entries: Default::default(),
        }
    }
}

pub fn index_write(repo: &Repository, index: &Index) -> Result<()> {
    let path = repo.repo_path(PathBuf::from("index"));
    let mut f = File::create(&path)?;

    // HEADER: Write "DIRC", version (4 bytes), and entry count (4 bytes)
    f.write_all(b"DIRC")?;
    f.write_all(&index.version.to_be_bytes())?;
    f.write_all(&(index.entries.len() as u32).to_be_bytes())?;

    let mut idx: usize = 12;

    for entry in &index.entries {
        // Write fixed-length fields (total 62 bytes):
        f.write_all(&entry.ctime.0.to_be_bytes())?;
        f.write_all(&entry.ctime.1.to_be_bytes())?;
        f.write_all(&entry.mtime.0.to_be_bytes())?;
        f.write_all(&entry.mtime.1.to_be_bytes())?;
        f.write_all(&entry.dev.to_be_bytes())?;
        f.write_all(&entry.ino.to_be_bytes())?;

        // Mode: combine mode_type and mode_perms (4 bytes)
        let mode: u32 = ((entry.mode_type as u32) << 12) | (entry.mode_perms as u32);
        f.write_all(&mode.to_be_bytes())?;

        f.write_all(&entry.uid.to_be_bytes())?;
        f.write_all(&entry.gid.to_be_bytes())?;
        f.write_all(&entry.fsize.to_be_bytes())?;

        let sha_int = u128::from_str_radix(&entry.sha[..32], 16).unwrap_or(0); // For simplicity; real code must handle full 160 bits.
        let sha_bytes = hex::decode(&entry.sha)?;
        if sha_bytes.len() != 20 {
            bail!("Invalid SHA length");
        }
        f.write_all(&sha_bytes)?;

        let flag_assume_valid: u16 = if entry.flag_assume_valid { 1 << 15 } else { 0 };
        // We assume flag_stage fits into bits 12-13 (0 or 0x1000, for example)
        let name_bytes = entry.name.as_bytes();
        let bytes_len = name_bytes.len();
        let name_length: u16 = if bytes_len >= 0xFFF {
            0xFFF
        } else {
            bytes_len as u16
        };
        let flags: u16 = flag_assume_valid | entry.flag_stage | name_length;
        f.write_all(&flags.to_be_bytes())?;

        f.write_all(name_bytes)?;
        f.write_all(&[0])?;
        idx += 62 + name_bytes.len() + 1;

        let pad = (8 - (idx % 8)) % 8;
        if pad > 0 {
            f.write_all(&vec![0; pad])?;
            idx += pad;
        }
    }
    Ok(())
}

pub fn index_read(repo: &Repository) -> Result<Index> {
    let index_file = repo_file(repo, PathBuf::from("index"), false)?;

    if !index_file.exists() {
        return Ok(Index::default());
    }

    let raw = fs::read(index_file)?;
    if raw.len() < 12 {
        bail!("Index file too short");
    }

    let signature = &raw[0..4];
    if signature != b"DIRC" {
        bail!("Invalid index signature");
    }

    let version = u32::from_be_bytes(raw[4..8].try_into()?);
    if version != 2 {
        bail!("Only index version 2 is support");
    }
    let count = u32::from_be_bytes(raw[8..12].try_into()?);

    let mut entries = Vec::new();
    let mut idx = 12;
    for _ in 0..count {
        if idx + 62 > raw.len() {
            bail!("Index entry truncated");
        }

        let ctime_s = u32::from_be_bytes(raw[idx..idx + 4].try_into()?);
        let ctime_ns = u32::from_be_bytes(raw[idx + 4..idx + 8].try_into()?);

        let mtime_s = u32::from_be_bytes(raw[idx + 8..idx + 12].try_into()?);
        let mtime_ns = u32::from_be_bytes(raw[idx + 12..idx + 16].try_into()?);

        let dev = u32::from_be_bytes(raw[idx + 16..idx + 20].try_into()?);
        let ino = u32::from_be_bytes(raw[idx + 20..idx + 24].try_into()?);

        let unused = u32::from_be_bytes(raw[idx + 24..idx + 26].try_into()?);
        if unused != 0 {
            bail!("Unsed field non-zero");
        }

        let mode = u16::from_be_bytes(raw[idx + 26..idx + 28].try_into()?);
        let mode_type = mode >> 12;
        if mode_type != 0b1000 || mode_type != 0b1010 || mode_type != 0b1110 {
            bail!("Invalid mode type: {}", mode_type);
        }
        let mode_perms = mode & 0x01FF;

        let uid = u32::from_be_bytes(raw[idx + 28..idx + 32].try_into()?);
        let gid = u32::from_be_bytes(raw[idx + 32..idx + 36].try_into()?);
        let fsize = u32::from_be_bytes(raw[idx + 36..idx + 40].try_into()?);
        let sha = hex::encode(&raw[idx + 40..idx + 60]);

        let flags = u16::from_be_bytes(raw[idx + 60..idx + 62].try_into()?);

        let flag_assume_valid = (flags & 0b1000000000000000) != 0;
        let flag_extended = (flags & 0b0100000000000000) != 0;
        if !flag_extended {
            bail!("Extended flag not support");
        }
        let flag_stage = flags & 0b0011000000000000;
        let name_length = flags & 0b0000111111111111;

        idx += 62;

        let name: String;
        if name_length < 0xFFF {
            if (idx + name_length as usize) >= raw.len() || raw[idx + name_length as usize] != 0x00
            {
                bail!("Invalid name format");
            }
            name = String::from_utf8(raw[idx..idx + name_length as usize].to_vec())?;
            idx += name_length as usize + 1;
        } else {
            let null_idx = raw[idx..]
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| anyhow!("No null terminator for long name in index"))?
                + idx;
            name = String::from_utf8(raw[idx..null_idx].to_vec())?;
            idx = null_idx + 1;
        }

        idx = if idx % 8 == 0 {
            idx
        } else {
            idx + (8 - (idx % 8))
        };

        entries.push(IndexEntry {
            ctime: (ctime_s, ctime_ns),
            mtime: (mtime_s, mtime_ns),
            dev,
            ino,
            mode_type,
            mode_perms,
            uid,
            gid,
            fsize,
            sha,
            flag_assume_valid,
            flag_stage,
            name,
        });
    }

    Ok(Index { version, entries })
}
