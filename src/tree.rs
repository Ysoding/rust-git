use anyhow::{anyhow, bail, Result};
use num_bigint::BigUint;

use crate::Object;

#[derive(Clone)]
pub struct TreeLeaf {
    pub mode: Vec<u8>,
    pub path: String,
    pub sha: String,
}

fn tree_parse_one(raw: &[u8], start: usize) -> Result<(usize, TreeLeaf)> {
    let spc = raw[start..]
        .iter()
        .position(|&b| b == b' ')
        .ok_or_else(|| anyhow!("Malformed tree: cannot find space"))?
        + start;
    assert!(spc - start == 5 || spc - start == 6);

    let mode = &raw[start..spc];
    let mode = if mode.len() == 5 {
        // normalize to six bytes.
        [&b"0"[..], mode].concat()
    } else {
        mode.to_vec()
    };

    let null_pos = raw[spc..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("Malformed tree: cannot find null after path"))?
        + spc;
    let path_bytes = &raw[spc + 1..null_pos];
    let path = String::from_utf8(path_bytes.to_vec())
        .map_err(|_| anyhow!("Invalid UTF-8 in tree path"))?;

    let sha_start = null_pos + 1;
    let sha_end = sha_start + 20;
    if sha_end > raw.len() {
        bail!("Malformed tree: SHA truncated");
    }
    let raw_sha = &raw[sha_start..sha_end];
    let sha = format!("{:040x}", BigUint::from_bytes_be(raw_sha));
    Ok((sha_end, TreeLeaf { mode, path, sha }))
}

fn tree_parse(raw: &[u8]) -> Result<Vec<TreeLeaf>> {
    let mut pos = 0;
    let max = raw.len();
    let mut res = Vec::new();
    while pos < max {
        let (new_pos, data) = tree_parse_one(raw, pos)?;
        res.push(data);
        pos = new_pos;
    }

    Ok(res)
}

pub fn tree_serialize(tree: &Tree) -> Vec<u8> {
    let mut items = tree.items.clone();
    items.sort_by_key(|leaf| {
        let mut key = leaf.path.clone();
        if leaf.mode.starts_with(b"10") {
            key.push('/');
        }
        key
    });

    let mut ret = Vec::new();
    for leaf in items.iter() {
        ret.extend_from_slice(&leaf.mode);
        ret.push(b' ');
        ret.extend_from_slice(leaf.path.as_bytes());
        ret.push(0);
        let sha_bytes = hex::decode(&leaf.sha).expect("Invalid SHA in tree leaf");
        ret.extend_from_slice(&sha_bytes);
    }
    ret
}

pub struct Tree {
    pub items: Vec<TreeLeaf>,
}

impl Tree {
    pub fn deserialize(data: &[u8]) -> Self {
        let items = tree_parse(data).unwrap_or_else(|_| Vec::new());
        Self { items }
    }
}

impl Object for Tree {
    fn fmt(&self) -> &'static [u8] {
        b"tree"
    }

    fn serialize(&self) -> Vec<u8> {
        tree_serialize(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
