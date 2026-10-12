(module
 (import "incant_localization_v1" "format" (func $format (param i64 i32 i32) (result i32)))
 (memory (export "memory") 1)
 (func $start i64.const 1 i32.const 0 i32.const 24 call $format drop)
 (start $start)
 (func (export "incant_abi") (result i32) i32.const 1)
 (func (export "update") (param i64 f64)))
