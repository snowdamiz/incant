(module
  (import "incant_animation_v2" "snapshot_len" (func $len (param i64) (result i32)))
  (import "incant_animation_v2" "read_snapshot" (func $snapshot (param i64 i32 i32) (result i32)))
  (import "incant_animation_v2" "control" (func $control (param i64 i32 i32)))
  (import "incant_animation_v1" "snapshot_len" (func $old_len (param i64) (result i32)))
  (import "incant_animation_v1" "control" (func $old_control (param i64 i32 i32)))
  (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
  (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
  (memory (export "memory") 2 2)
  (func (export "incant_abi") (result i32) i32.const 1)
  (func (export "update") (param $e i64) (param $dt f64) (local $mode i32) (local $n i32)
    local.get $dt f64.const 60 f64.mul f64.nearest i32.trunc_f64_u i32.const 1 i32.sub local.set $mode
    local.get $e call $len local.set $n
    local.get $e i32.const 0 i32.const 0 call $read
    i32.const 0 f64.const 77 f64.store
    local.get $e i32.const 0 i32.const 0 call $write
    ;; Unaligned copied snapshot/identity/control, no native pointer or loan.
    local.get $e i32.const 1025 local.get $n call $snapshot drop
    i32.const 1033 i32.load i32.const 2 i32.ne if unreachable end
    i32.const 1037 i32.load i32.const 80 i32.ne if unreachable end
    i32.const 2049 i32.const 5 i32.store
    i32.const 2053 i32.const 0 i32.store
    i32.const 2057 f64.const 0.75 f64.store
    i32.const 2065 f64.const 0.5 f64.store
    local.get $e i32.const 1041 i32.const 2049 call $control
    local.get $e i32.const 1025 local.get $n call $snapshot drop
    i32.const 1085 i32.load i32.eqz if else unreachable end
    local.get $mode i32.const 1 i32.eq if local.get $e i32.const 1025 local.get $n i32.const 1 i32.sub call $snapshot drop end
    local.get $mode i32.const 2 i32.eq if local.get $e i32.const 131060 local.get $n call $snapshot drop end
    local.get $mode i32.const 3 i32.eq if i32.const 2049 i32.const 99 i32.store local.get $e i32.const 1041 i32.const 2049 call $control end
    local.get $mode i32.const 4 i32.eq if local.get $e i64.const 1 i64.add i32.const 1041 i32.const 2049 call $control end
    local.get $mode i32.const 5 i32.eq if unreachable end
    local.get $mode i32.const 6 i32.eq if i32.const 2065 f64.const nan f64.store local.get $e i32.const 1041 i32.const 2049 call $control end
    local.get $mode i32.const 7 i32.eq if local.get $e i32.const 1041 i32.const 2049 call $old_control end
    local.get $mode i32.const 8 i32.eq if local.get $e i32.const 1041 i32.const 2049 call $control end
    ;; Normal legacy observations remain faithful until the queued fade commits.
    local.get $e call $old_len i32.const 96 i32.ne if unreachable end))
