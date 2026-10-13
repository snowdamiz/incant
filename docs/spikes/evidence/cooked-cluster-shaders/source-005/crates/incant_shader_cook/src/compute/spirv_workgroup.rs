//! Naga29 decorates every array with ArrayStride, including private workgroup
//! storage. Vulkan forbids that explicit layout without a Block contract.
//! Remove only the fixed membership array's decoration, never shader code.
use crate::{CookError, Result};
const TYPE_INT: u32 = 21;
const TYPE_ARRAY: u32 = 28;
const TYPE_RUNTIME_ARRAY: u32 = 29;
const TYPE_STRUCT: u32 = 30;
const TYPE_POINTER: u32 = 32;
const TYPE_FUNCTION: u32 = 33;
const CONSTANT: u32 = 43;
const VARIABLE: u32 = 59;
const DECORATE: u32 = 71;
const WORKGROUP: u32 = 4;
const ARRAY_STRIDE: u32 = 6;
fn fail() -> CookError {
    CookError::Contract("cluster SPIR-V workgroup layout")
}
fn unique<'a>(mut rows: impl Iterator<Item = &'a [u32]>) -> Result<&'a [u32]> {
    let row = rows.next().ok_or_else(fail)?;
    if rows.next().is_some() {
        return Err(fail());
    }
    Ok(row)
}
fn instructions(words: &[u32]) -> Result<Vec<&[u32]>> {
    if words.len() < 5
        || words[0] != 0x07230203
        || words[1] != 0x00010000
        || words[3] == 0
        || words[4] != 0
    {
        return Err(fail());
    }
    let mut remaining = &words[5..];
    let mut result = Vec::new();
    while !remaining.is_empty() {
        let n = (remaining[0] >> 16) as usize;
        if n == 0 || n > remaining.len() {
            return Err(fail());
        }
        result.push(&remaining[..n]);
        remaining = &remaining[n..];
    }
    Ok(result)
}
pub(super) fn lower(words: &[u32]) -> Result<Vec<u32>> {
    let rows = instructions(words)?;
    let op = |code| rows.iter().copied().filter(move |r| r[0] & 0xffff == code);
    let uint = unique(op(TYPE_INT).filter(|r| r.len() == 4 && r[2..] == [32, 0]))?[1];
    let count = unique(op(CONSTANT).filter(|r| r.len() == 4 && r[1] == uint && r[3] == 128))?[2];
    // This fixed shader has precisely one fixed-size array, used only for membership.
    let array = unique(op(TYPE_ARRAY))?;
    if array.len() != 4 || array[2..] != [uint, count] {
        return Err(fail());
    }
    let array = array[1];
    let pointer = unique(op(TYPE_POINTER).filter(|r| r.len() == 4 && r[3] == array))?;
    if pointer[2] != WORKGROUP {
        return Err(fail());
    }
    let pointer = pointer[1];
    let variable = unique(op(VARIABLE).filter(|r| r.len() >= 2 && r[1] == pointer))?;
    if variable.len() != 4 || variable[3] != WORKGROUP {
        return Err(fail());
    }
    // No aggregate, nested array, function signature or alternate pointer can
    // share this type. These guards apply to generated trusted words only.
    for r in &rows {
        match r[0] & 0xffff {
            TYPE_RUNTIME_ARRAY if r.len() != 3 || r[2] == array => return Err(fail()),
            TYPE_STRUCT if r.len() < 2 || r[2..].contains(&array) || r[2..].contains(&pointer) => {
                return Err(fail());
            }
            TYPE_FUNCTION
                if r.len() < 3 || r[2..].contains(&array) || r[2..].contains(&pointer) =>
            {
                return Err(fail());
            }
            TYPE_POINTER if r.len() != 4 || r[3] == pointer => return Err(fail()),
            _ => {}
        }
    }
    for id in [uint, count, array, pointer, variable[2]] {
        if id == 0 || id >= words[3] {
            return Err(fail());
        }
    }
    let stride = unique(op(DECORATE).filter(|r| r.len() >= 2 && r[1] == array))?;
    if stride.len() != 4 || stride[2..] != [ARRAY_STRIDE, 4] {
        return Err(fail());
    }
    let mut output = Vec::with_capacity(words.len() - stride.len());
    output.extend_from_slice(&words[..5]);
    for row in rows {
        if !std::ptr::eq(row, stride) {
            output.extend_from_slice(row);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u32> {
        vec![
            0x07230203,
            0x00010000,
            0,
            20,
            0,
            4 << 16 | TYPE_INT,
            1,
            32,
            0,
            4 << 16 | CONSTANT,
            1,
            2,
            128,
            4 << 16 | TYPE_ARRAY,
            3,
            1,
            2,
            4 << 16 | TYPE_POINTER,
            4,
            WORKGROUP,
            3,
            4 << 16 | VARIABLE,
            4,
            5,
            WORKGROUP,
            4 << 16 | DECORATE,
            3,
            ARRAY_STRIDE,
            4,
        ]
    }
    #[test]
    fn removes_only_one_stride_row_and_refuses_ambiguous_or_shared_types() {
        let good = fixture();
        let mut expected = good.clone();
        expected.truncate(expected.len() - 4);
        assert_eq!(lower(&good).unwrap(), expected);
        let mut cases = vec![vec![], good[..good.len() - 1].to_vec(), expected];
        for (index, value) in [
            (0, 0),
            (1, 0x00010300),
            (3, 5),
            (4, 1),
            (5, 0),
            (8, 1),
            (12, 129),
            (20, 2),
            (24, 2),
            (28, 8),
        ] {
            let mut bad = good.clone();
            bad[index] = value;
            cases.push(bad);
        }
        for row in [
            vec![4 << 16 | TYPE_ARRAY, 6, 1, 2],
            vec![4 << 16 | TYPE_POINTER, 6, 2, 3],
            vec![4 << 16 | VARIABLE, 4, 6, WORKGROUP],
            vec![4 << 16 | DECORATE, 3, ARRAY_STRIDE, 4],
            vec![3 << 16 | TYPE_STRUCT, 6, 3],
            vec![3 << 16 | TYPE_RUNTIME_ARRAY, 6, 3],
            vec![4 << 16 | TYPE_FUNCTION, 6, 1, 3],
            vec![4 << 16 | TYPE_POINTER, 6, WORKGROUP, 4],
        ] {
            let mut bad = good.clone();
            bad.extend(row);
            cases.push(bad);
        }
        for bad in cases {
            assert!(lower(&bad).is_err(), "accepted {bad:?}");
        }
    }
}
