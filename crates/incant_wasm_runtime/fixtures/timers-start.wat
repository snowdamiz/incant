(module
  (import "incant_timers_v1" "snapshot_len" (func $timers (param i64) (result i32)))
  (memory (export "memory") 1)
  (func $start (drop (call $timers (i64.const 0))))
  (start $start)
  (func (export "incant_abi") (result i32) (i32.const 1))
  (func (export "update") (param i64 f64)))
