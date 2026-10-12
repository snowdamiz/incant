(module
  (import "incant_animation_v1" "snapshot_len" (func $len1 (param i64) (result i32)))
  (import "incant_animation_v1" "read_snapshot" (func $read1 (param i64 i32 i32) (result i32)))
  (import "incant_animation_v2" "snapshot_len" (func $len2 (param i64) (result i32)))
  (import "incant_animation_v2" "read_snapshot" (func $read2 (param i64 i32 i32) (result i32)))
  (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
  (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
  (memory (export "memory") 1 1)
  (func (export "incant_abi") (result i32) i32.const 1)
  (func (export "update") (param $e i64) (param $dt f64) (local $mode i32)
    local.get $dt f64.const 60 f64.mul f64.nearest i32.trunc_f64_u local.set $mode
    local.get $e i32.const 0 i32.const 0 call $read
    i32.const 0 f64.const 77 f64.store
    local.get $e i32.const 0 i32.const 0 call $write
    ;; The four old imports must trap for a graph's actual two-pose blend.
    ;; Endpoint graphs remain representable, and mode 5 never asks for a snapshot.
    local.get $mode i32.const 1 i32.eq if local.get $e call $len1 drop end
    local.get $mode i32.const 2 i32.eq if local.get $e i32.const 1025 i32.const 500 call $read1 drop end
    local.get $mode i32.const 3 i32.eq if local.get $e call $len2 drop end
    local.get $mode i32.const 4 i32.eq if local.get $e i32.const 1025 i32.const 500 call $read2 drop end))
