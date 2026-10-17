(module
  (import "incant_haptics_v1" "snapshot_len" (func $targets (param i64) (result i32)))
  (memory (export "memory") 1)
  (func $start (drop (call $targets (i64.const 0))))
  (start $start)
  (func (export "incant_abi") (result i32) (i32.const 1))
  (func (export "update") (param i64 f64)))
