use indexmap::IndexMap;

/// We use an IndexMap to preserve insertion order. Keys are of type Option<Vec<u8>>:
/// - Some(key) holds header fields (like "tree", "parent", "author", etc.).
/// - None is reserved for the commit message.
pub type Kvlm = IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>;

pub fn kvlm_parse(raw: &[u8]) -> Kvlm {
    let mut dict: Kvlm = IndexMap::new();
    let mut pos = 0;

    while pos < raw.len() {
        // If space appears before newline, we have a keyword.  Otherwise,
        // it's the final message, which we just read to the end of the file.
        let spc_rel = raw[pos..].iter().position(|&b| b == b' ');
        let nl_rel = raw[pos..].iter().position(|&b| b == b'\n');
        if spc_rel.is_none() || (nl_rel.is_some() && nl_rel.unwrap() < spc_rel.unwrap()) {
            assert_eq!(spc_rel, nl_rel);
            let msg = raw[pos + 1..].to_vec();
            dict.insert(None, vec![msg]);
        }

        let spc = pos + spc_rel.unwrap();
        let nl = pos + nl_rel.unwrap();

        let key = raw[pos..spc].to_vec();

        //  Find the end of the value.  Continuation lines begin with a
        //  space, so we loop until we find a "\n" not followed by a space.
        let mut end = nl;
        loop {
            if end + 1 >= raw.len() {
                break;
            }
            if raw[end + 1] != b' ' {
                break;
            }
            // find the next newline.
            let next_nl_rel = raw[end + 1..].iter().position(|&b| b == b'\n');
            if let Some(nl_offset) = next_nl_rel {
                end = end + 1 + nl_offset;
            } else {
                end = raw.len();
                break;
            }
        }
        let mut value = raw[spc + 1..end].to_vec();

        value = continuation_line_optmize(&value);

        dict.entry(Some(key))
            .and_modify(|v| v.push(value.clone()))
            .or_insert(vec![value]);
        pos = end + 1;
    }

    dict
}

/// For continuation lines, remove the leading space after newlines.
fn continuation_line_optmize(value: &[u8]) -> Vec<u8> {
    let mut optmize_value = Vec::with_capacity(value.len());
    let mut i = 0;
    while i < value.len() {
        if value[i] == b'\n' {
            optmize_value.push(b'\n');
            if i + 1 < value.len() && value[i + 1] == b' ' {
                i += 2;
                continue;
            }
        } else {
            optmize_value.push(value[i]);
        }
        i += 1;
    }
    optmize_value
}

#[cfg(test)]
mod tests {
    use crate::commit::continuation_line_optmize;

    #[test]
    fn test_continuation_line_optmize() {
        let raw_data = b"value\n value continued\n more value";

        let optimized = continuation_line_optmize(raw_data);
        let expected = b"value\nvalue continued\nmore value";

        let optimized_str = String::from_utf8_lossy(&optimized);
        let expected_str = String::from_utf8_lossy(expected);

        assert_eq!(optimized_str.trim(), expected_str.trim());
    }
}
