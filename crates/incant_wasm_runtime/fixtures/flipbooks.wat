(module
  (import "incant_flipbook_v1" "snapshot_len" (func $len (param i64) (result i32)))
  (import "incant_flipbook_v1" "read_snapshot" (func $snapshot (param i64 i32 i32) (result i32)))
  (import "incant_flipbook_v1" "control" (func $control (param i64 i32 i32 i32)))
  (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
  (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
  (memory (export "memory") 2 2)
  (global $turn (mut i32) (i32.const 0))
  (func (export "incant_abi") (result i32) i32.const 1)
  
  (func (export "update") (param $e i64) (param f64) (local $n i32) (local $frame i32) (local $flags i32)
    local.get $e call $len local.set $n
    ;; First queue a numeric edit, proving failed imports/commit retain no prefix.
    local.get $e i32.const 0 i32.const 0 call $read
    i32.const 0 f64.const 77 f64.store
    local.get $e i32.const 0 i32.const 0 call $write
    local.get $e i32.const 1024 local.get $n call $snapshot drop
    i32.const 1024 i64.load i64.const 3544385967945371209 i64.ne if unreachable end
    i32.const 1032 i32.load i32.const 2 i32.ne if unreachable end
    i32.const 1036 i32.load i32.const 24 i32.ne if unreachable end
    ;; Both rows are visited in one copy. Capture a pre-command observation.
    i32.const 1056 i32.load local.set $frame
    i32.const 1060 i32.load local.set $flags
    global.get $turn i32.const 0 i32.eq if
      local.get $e i32.const 1040 i32.const 1 i32.const 0 call $control
    end
    global.get $turn i32.const 1 i32.eq if
      local.get $e i32.const 1040 i32.const 3 i32.const 2 call $control
    end
    global.get $turn i32.const 2 i32.eq if
      local.get $e i32.const 1040 i32.const 0 i32.const 0 call $control
    end
    global.get $turn i32.const 3 i32.eq if
      local.get $e i32.const 1040 i32.const 2 i32.const 0 call $control
    end
    ;; Snapshot stays committed even after queued commands.
    local.get $e i32.const 1024 local.get $n call $snapshot drop
    i32.const 1056 i32.load local.get $frame i32.ne if unreachable end
    i32.const 1060 i32.load local.get $flags i32.ne if unreachable end
    global.get $turn i32.const 1 i32.add global.set $turn))
