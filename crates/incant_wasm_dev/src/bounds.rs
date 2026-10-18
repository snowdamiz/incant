//! Fixed first-ABI quotas. Not a total-process/JIT allocator or wall-time limit.
use crate::ModuleError;
pub use incant_wasm_runtime::limits::*;
use wasmparser::{Parser, Payload};

pub fn preflight(
    bytes: &[u8],
) -> Result<Option<incant_wasm_abi::persistent::Declaration>, ModuleError> {
    let declaration =
        incant_wasm_abi::persistent::declaration_in_module(bytes).map_err(ModuleError::Abi)?;
    let coroutines =
        incant_wasm_abi::coroutines::catalog_in_module(bytes).map_err(ModuleError::Abi)?;
    if bytes.len() > 1024 * 1024 || bytes.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err(ModuleError::Abi("expected core Wasm binary, at most 1 MiB"));
    }
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|_| ModuleError::Abi("malformed Wasm binary"))?;
        let allowed = match payload {
            Payload::TypeSection(s) => {
                let mut count = 0;
                for group in s {
                    count += group
                        .map_err(|_| ModuleError::Abi("type group"))?
                        .types()
                        .len();
                    if count > 1024 {
                        return Err(ModuleError::Limit("module types"));
                    }
                }
                true
            }
            Payload::ImportSection(s) => {
                for (count, import) in s.into_imports().enumerate() {
                    let import = import.map_err(|_| ModuleError::Abi("import"))?;
                    if import.module == incant_wasm_abi::coroutines::SECTION && coroutines.is_none()
                    {
                        return Err(ModuleError::Abi("coroutine imports require catalog"));
                    }
                    if import.module == incant_wasm_abi::persistent::SECTION
                        && declaration.is_none()
                    {
                        return Err(ModuleError::Abi("persistent imports require declaration"));
                    }
                    if count >= 32 {
                        return Err(ModuleError::Limit("module imports"));
                    }
                }
                true
            }
            Payload::ExportSection(s) => s.count() <= 128,
            Payload::DataSection(s) => s.count() <= 128,
            Payload::ElementSection(s) => s.count() <= 128,
            Payload::FunctionSection(s) => s.count() <= 1024,
            Payload::GlobalSection(s) => s.count() <= 128,
            Payload::TableSection(s) => s.count() <= 1,
            Payload::MemorySection(s) => s.count() == 1,
            Payload::CodeSectionEntry(body) => {
                let mut locals = 0u64;
                for local in body
                    .get_locals_reader()
                    .map_err(|_| ModuleError::Abi("locals"))?
                {
                    locals += u64::from(local.map_err(|_| ModuleError::Abi("locals"))?.0);
                }
                locals <= 4096 && (body.range().end - body.range().start) <= 256 * 1024
            }
            _ => true,
        };
        if !allowed {
            return Err(ModuleError::Limit("module declarations"));
        }
    }
    Ok(declaration)
}
