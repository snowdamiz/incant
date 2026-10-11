(module
  (import "incant_log_v1" "write" (func $log (param i64 i32 i32 i32)))
  (memory (export "memory") 1)
  (func (export "incant_abi") (result i32) (i32.const 1))
  (func $start (call $log (i64.const 0) (i32.const 1) (i32.const 0) (i32.const 0)))
  (start $start)
  (func (export "update") (param i64 f64)))
