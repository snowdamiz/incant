(module
  (import "incant_input_v1" "snapshot_len" (func $len (param i64) (result i32)))
  (memory (export "memory") 1 1)
  (func (export "incant_abi") (result i32) i32.const 1)
  (func $start i64.const 0 call $len drop) (start $start)
  (func (export "update") (param i64 f64)))
