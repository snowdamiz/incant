//! Exact engine-owned boundary assembly, shared by cold runtime and host cook.
pub fn source(normalize: bool) -> String {
    [
        include_str!("common.wgsl"),
        include_str!("alpha.wgsl"),
        if normalize {
            include_str!("normalize.wgsl")
        } else {
            include_str!("prepare.wgsl")
        },
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};
    #[test]
    fn boundary_assembly_preserves_exact_reviewed_equations() {
        for (normalize, hash) in [
            (
                false,
                "6ec97c6b24fb89cb93d44969cdeba25c16b22e7bb23574f95c8fbe29746b0efb",
            ),
            (
                true,
                "ed24d096849a33c5d4c03ca2ada6f2b9281144367f6890206ef2bc65d5867b46",
            ),
        ] {
            assert_eq!(
                format!("{:x}", Sha256::digest(super::source(normalize))),
                hash
            );
        }
    }
}
