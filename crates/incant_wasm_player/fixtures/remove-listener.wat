(module
  (import "incant_v1" "despawn" (func $despawn (param i64 i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\02")
  (func (export "incant_abi") (result i32) (i32.const 1))
  (func (export "update") (param $epoch i64) (param f64)
    (call $despawn (local.get $epoch) (i32.const 0))))
