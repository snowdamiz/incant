//! Build-time validation and private metering for the static wasm2c path.
//! No native executable/cache input is accepted. No shipped host depends on this.
mod captured;
pub use captured::{CapturedModule, MAX_CAPTURE_BYTES, MAX_MODULE_BYTES, project_cache};
use thiserror::Error;
use wasm_encoder::{
    reencode::{self, Reencode},
    *,
};
use wasmparser::{ExternalKind, Operator, Parser, Payload, TypeRef, ValType as V, WasmFeatures};
#[derive(Debug, Error)]
#[error("static Wasm preparation: {0}")]
pub struct CookError(String);
fn error(message: impl ToString) -> CookError {
    CookError(message.to_string())
}
pub const FUEL_EXPORT: &str = "incant_private_fuel";
pub const START_FUEL: i64 = 1_000_000;
const PAGE_LIMIT: u64 = 512;

/// Validate the original module before adding any trusted metering instructions.
/// Shipping v1 deliberately refuses memory/table growth; storage is fixed at
/// instantiation. This is narrower than the development host's bounded growth.
pub fn prepare(bytes: &[u8]) -> Result<Vec<u8>, CookError> {
    if bytes.len() > 1024 * 1024 || bytes.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err(error("expected core Wasm <= 1 MiB"));
    }
    let features = WasmFeatures::default()
        & !(WasmFeatures::THREADS
            | WasmFeatures::SHARED_EVERYTHING_THREADS
            | WasmFeatures::MEMORY64
            | WasmFeatures::MULTI_MEMORY
            | WasmFeatures::SIMD
            | WasmFeatures::RELAXED_SIMD
            | WasmFeatures::FUNCTION_REFERENCES
            | WasmFeatures::GC
            | WasmFeatures::TAIL_CALL
            | WasmFeatures::STACK_SWITCHING
            | WasmFeatures::EXCEPTIONS
            | WasmFeatures::LEGACY_EXCEPTIONS
            | WasmFeatures::COMPACT_IMPORTS
            | WasmFeatures::CUSTOM_PAGE_SIZES);
    wasmparser::Validator::new_with_features(features)
        .validate_all(bytes)
        .map_err(error)?;
    let mut types = Vec::new();
    let mut funcs = Vec::new();
    let mut exports = Vec::new();
    let mut globals = 0;
    let mut memories = 0;
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(error)? {
            Payload::TypeSection(s) => {
                for ty in s.into_iter_err_on_gc_types() {
                    let ty = ty.map_err(error)?;
                    if ty.params().len() > 128 || ty.results().len() > 16 {
                        return Err(error("function signature quota"));
                    }
                    types.push(ty);
                    if types.len() > 1024 {
                        return Err(error("type quota"));
                    }
                }
            }
            Payload::ImportSection(s) => {
                for (n, i) in s.into_imports().enumerate() {
                    if n >= 32 {
                        return Err(error("import quota"));
                    }
                    let i = i.map_err(error)?;
                    let TypeRef::Func(index) = i.ty else {
                        return Err(error("only function imports supported"));
                    };
                    let ty = &types[index as usize];
                    let contract = incant_wasm_abi::import(i.module, i.name)
                        .ok_or_else(|| error("unsupported import capability"))?;
                    let value = |v: &incant_wasm_abi::ValueType| match v {
                        incant_wasm_abi::ValueType::I32 => V::I32,
                        incant_wasm_abi::ValueType::I64 => V::I64,
                    };
                    if !ty
                        .params()
                        .iter()
                        .copied()
                        .eq(contract.params.iter().map(value))
                        || !ty
                            .results()
                            .iter()
                            .copied()
                            .eq(contract.results.iter().map(value))
                    {
                        return Err(error("import signature"));
                    }
                    funcs.push(index);
                }
            }
            Payload::FunctionSection(s) => {
                if s.count() > 1024 {
                    return Err(error("function quota"));
                }
                for f in s {
                    funcs.push(f.map_err(error)?);
                }
            }
            Payload::GlobalSection(s) => {
                globals = s.count();
                if globals > 128 {
                    return Err(error("global quota"));
                }
            }
            Payload::MemorySection(s) => {
                memories = s.count();
                if memories != 1 {
                    return Err(error("one memory required"));
                }
                for m in s {
                    let m = m.map_err(error)?;
                    if m.initial > PAGE_LIMIT {
                        return Err(error("memory quota"));
                    }
                }
            }
            Payload::TableSection(s) => {
                if s.count() > 1 {
                    return Err(error("table quota"));
                }
                for t in s {
                    let t = t.map_err(error)?;
                    if t.ty.initial > 4096
                        || t.ty.table64
                        || t.ty.element_type != wasmparser::RefType::FUNCREF
                    {
                        return Err(error("table quota/type"));
                    }
                }
            }
            Payload::ExportSection(s) => {
                if s.count() > 128 {
                    return Err(error("export quota"));
                }
                for e in s {
                    let e = e.map_err(error)?;
                    if e.name.starts_with("incant_private_") {
                        return Err(error("reserved export"));
                    }
                    exports.push(e);
                }
            }
            Payload::DataSection(s) if s.count() > 128 => return Err(error("data quota")),
            Payload::ElementSection(s) if s.count() > 128 => return Err(error("element quota")),
            Payload::CodeSectionEntry(body) => {
                if (body.range().end - body.range().start) > 256 * 1024 {
                    return Err(error("function body quota"));
                }
                let mut locals = 0u64;
                for l in body.get_locals_reader().map_err(error)? {
                    locals += u64::from(l.map_err(error)?.0);
                }
                if locals > 128 {
                    return Err(error("local quota"));
                }
                for op in body.get_operators_reader().map_err(error)? {
                    if matches!(
                        op.map_err(error)?,
                        Operator::MemoryGrow { .. } | Operator::TableGrow { .. }
                    ) {
                        return Err(error("static ABI v1 refuses memory/table growth"));
                    }
                }
            }
            _ => {}
        }
    }
    if memories != 1 {
        return Err(error("one defined memory required"));
    }
    for (name, params, results) in [
        ("incant_abi", &[][..], &[V::I32][..]),
        ("update", &[V::I64, V::F64][..], &[][..]),
    ] {
        let e = exports
            .iter()
            .find(|e| e.name == name && e.kind == ExternalKind::Func)
            .ok_or_else(|| error("missing function export"))?;
        let ty = &types[funcs[e.index as usize] as usize];
        if ty.params() != params || ty.results() != results {
            return Err(error("export signature"));
        }
    }
    if !exports
        .iter()
        .any(|e| e.name == "memory" && e.kind == ExternalKind::Memory && e.index == 0)
    {
        return Err(error("missing memory export"));
    }
    // Bound the operand stack as well as locals before emitting native C frames.
    let mut validator = wasmparser::Validator::new_with_features(features);
    for payload in Parser::new(0).parse_all(bytes) {
        if let wasmparser::ValidPayload::Func(f, body) =
            validator.payload(&payload.map_err(error)?).map_err(error)?
        {
            let mut f = f.into_validator(Default::default());
            let mut reader = body.get_binary_reader();
            f.read_locals(&mut reader).map_err(error)?;
            let mut ops = body.get_operators_reader().map_err(error)?;
            while !ops.eof() {
                let offset = ops.original_position();
                let op = ops.read().map_err(error)?;
                f.op(offset, &op).map_err(error)?;
                if f.operand_stack_height() > 128 {
                    return Err(error("operand stack quota"));
                }
            }
        }
    }
    let mut meter = Meter {
        global: globals,
        added: false,
    };
    let mut module = Module::new();
    meter
        .parse_core_module(&mut module, Parser::new(0), bytes)
        .map_err(error)?;
    let output = module.finish();
    wasmparser::Validator::new_with_features(features)
        .validate_all(&output)
        .map_err(error)?;
    Ok(output)
}
struct Meter {
    global: u32,
    added: bool,
}
impl Meter {
    fn add(&mut self, g: &mut GlobalSection) {
        g.global(
            GlobalType {
                val_type: ValType::I64,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i64_const(START_FUEL),
        );
        self.added = true;
    }
}
impl Reencode for Meter {
    type Error = std::convert::Infallible;
    fn parse_global_section(
        &mut self,
        g: &mut GlobalSection,
        s: wasmparser::GlobalSectionReader<'_>,
    ) -> Result<(), reencode::Error<Self::Error>> {
        reencode::utils::parse_global_section(self, g, s)?;
        self.add(g);
        Ok(())
    }
    fn intersperse_section_hook(
        &mut self,
        m: &mut Module,
        _after: Option<SectionId>,
        before: Option<SectionId>,
    ) -> Result<(), reencode::Error<Self::Error>> {
        if !self.added && before.is_none_or(|b| b as u8 > SectionId::Global as u8) {
            let mut g = GlobalSection::new();
            self.add(&mut g);
            m.section(&g);
        }
        Ok(())
    }
    fn parse_export_section(
        &mut self,
        e: &mut ExportSection,
        s: wasmparser::ExportSectionReader<'_>,
    ) -> Result<(), reencode::Error<Self::Error>> {
        reencode::utils::parse_export_section(self, e, s)?;
        e.export(FUEL_EXPORT, ExportKind::Global, self.global);
        Ok(())
    }
    fn memory_type(
        &mut self,
        ty: wasmparser::MemoryType,
    ) -> Result<MemoryType, reencode::Error<Self::Error>> {
        let mut ty = reencode::utils::memory_type(self, ty);
        ty.maximum = Some(ty.minimum);
        Ok(ty)
    }
    fn table_type(
        &mut self,
        ty: wasmparser::TableType,
    ) -> Result<TableType, reencode::Error<Self::Error>> {
        let mut ty = reencode::utils::table_type(self, ty)?;
        ty.maximum = Some(ty.minimum);
        Ok(ty)
    }
    fn parse_function_body(
        &mut self,
        code: &mut CodeSection,
        body: wasmparser::FunctionBody<'_>,
    ) -> Result<(), reencode::Error<Self::Error>> {
        let mut f = self.new_function_with_parsed_locals(&body)?;
        let mut ops = body.get_operators_reader()?;
        while !ops.eof() {
            // Stack-neutral charge before every original instruction, including
            // loop bodies and indirect calls. Original validation prevents access
            // to this appended private global; no function indices are shifted.
            for op in [
                Instruction::GlobalGet(self.global),
                Instruction::I64Eqz,
                Instruction::If(BlockType::Empty),
                Instruction::Unreachable,
                Instruction::End,
                Instruction::GlobalGet(self.global),
                Instruction::I64Const(1),
                Instruction::I64Sub,
                Instruction::GlobalSet(self.global),
            ] {
                f.instruction(&op);
            }
            f.instruction(&self.parse_instruction(&mut ops)?);
        }
        code.function(&f);
        Ok(())
    }
}
