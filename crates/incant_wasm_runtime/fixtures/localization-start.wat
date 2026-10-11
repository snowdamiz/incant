(module
 (import "incant_localization_v1" "locale" (func $locale (param i64) (result i32)))
 (memory (export "memory") 1)
 (func $start i64.const 1 call $locale drop)
 (start $start)
 (func (export "incant_abi") (result i32) i32.const 1)
 (func (export "update") (param i64 f64)))
