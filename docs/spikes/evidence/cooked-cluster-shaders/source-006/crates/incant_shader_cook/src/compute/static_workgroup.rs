//! Naga29's two fixed workgroup parameters become kernel-scope static storage.
//! Passthrough has no workgroup-memory reflection. No body statement is changed.
use crate::{CookError, Result};
const PARAMETERS: &str =
    ", threadgroup metal::atomic_uint& hits\n, threadgroup type_7& membership\n";
const HEADER: &str = "  metal::uint3 id [[threadgroup_position_in_grid]]
, uint lane [[thread_index_in_threadgroup]]
, metal::uint3 __local_invocation_id [[thread_position_in_threadgroup]]
, constant Grid& grid [[buffer(0)]]
, device type_3 const& lights [[buffer(1)]]
, device type_5& counts [[buffer(2)]]
, device type_5& masks [[buffer(3)]]
, threadgroup metal::atomic_uint& hits
, threadgroup type_7& membership
, constant _mslBufferSizes& _buffer_sizes [[buffer(4)]]
";
const LOCALS: &str =
    "    threadgroup metal::atomic_uint hits;\n    threadgroup type_7 membership;\n";
pub(super) fn lower(source: &str, entry: &str) -> Result<String> {
    let fail = || CookError::Contract("cluster generated workgroup interface");
    if entry != "build"
        || source.matches("kernel void ").count() != 1
        || source.matches(PARAMETERS).count() != 1
        || source.matches("threadgroup ").count() != 2
        || source.contains("[[threadgroup(")
    {
        return Err(fail());
    }
    let (prefix, kernel) = source.split_once("kernel void build(\n").ok_or_else(fail)?;
    let (params, body) = kernel.split_once(") {\n").ok_or_else(fail)?;
    if params != HEADER || !prefix.contains("metal::atomic_uint inner[128];") {
        return Err(fail());
    }
    let params = params.replacen(PARAMETERS, "", 1);
    Ok(format!(
        "{prefix}kernel void build(\n{params}) {{\n{LOCALS}{body}"
    ))
}

#[cfg(test)]
pub(super) fn restore(source: &str) -> String {
    source
        .replace(LOCALS, "")
        .replace(&HEADER.replace(PARAMETERS, ""), HEADER)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lowering_moves_only_declarations_and_retains_body_byte_for_byte() {
        let source = format!(
            "struct type_7 {{ metal::atomic_uint inner[128]; }};\nkernel void build(\n{HEADER}) {{\nzero_init();\nbarrier();\nbody();\n}}"
        );
        let lowered = lower(&source, "build").unwrap();
        let reconstructed = restore(&lowered);
        assert_eq!(reconstructed, source);
        for changed in [
            source.replace("inner[128]", "inner[129]"),
            source.replace("type_7&", "type_8&"),
            source.replace("uint lane", "threadgroup uint& extra"),
            source.replace("[[buffer(4)]]", "[[buffer(5)]]"),
            source.replace("thread_index_in_threadgroup", "thread_index_in_simdgroup"),
            source.replace("kernel void build", "kernel void other"),
        ] {
            assert!(lower(&changed, "build").is_err());
        }
    }
}
