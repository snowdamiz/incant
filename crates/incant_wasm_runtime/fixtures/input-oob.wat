(module
  (import "incant_input_v1" "snapshot_len" (func $len (param i64) (result i32)))
  (import "incant_input_v1" "read_snapshot" (func $input (param i64 i32 i32) (result i32)))
  (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
  (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
  (memory (export "memory") 4 4)
  (func (export "incant_abi") (result i32) i32.const 1)
  (func (export "update") (param $e i64) (param f64)
    local.get $e i32.const 0 i32.const 0 call $read
    i32.const 0 f64.const 99 f64.store
    local.get $e i32.const 0 i32.const 0 call $write
    local.get $e i32.const 262140 i32.const 131072 call $input drop))
