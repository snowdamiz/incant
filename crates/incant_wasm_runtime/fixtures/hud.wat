(module
  (import "incant_hud_v1" "snapshot_len" (func $len (param i64) (result i32)))
  (import "incant_hud_v1" "read_snapshot" (func $read (param i64 i32 i32) (result i32)))
  (import "incant_hud_v1" "control" (func $control (param i64 i32 i32) (result i32)))
  (memory (export "memory") 1 1)
  (func (export "incant_abi") (result i32) (i32.const 1))
  (func $need (param $v i32) (if (i32.eqz (local.get $v)) (then unreachable)))
  (func (export "update") (param $epoch i64) (param $dt f64)
    (local $mode i32) (local $n i32) (local $i i32)
    (local.set $mode (i32.sub (i32.trunc_f64_u (f64.mul (local.get $dt) (f64.const 64))) (i32.const 1)))
    (local.set $n (call $len (local.get $epoch)))
    (call $need (i32.gt_s (local.get $n) (i32.const 88)))
    ;; Valid retained prefix: widget2 becomes disabled. Any active error below
    ;; must reject it even though this raw guest catches the import return code.
    (memory.fill (i32.const 1024) (i32.const 0) (i32.const 33))
    (i32.store8 (i32.const 1039) (i32.const 2))
    (i32.store (i32.const 1040) (i32.const 1))
    (i32.store (i32.const 1044) (i32.const 1))
    (call $need (i32.eqz (call $control (local.get $epoch) (i32.const 1024) (i32.const 32))))
    (memory.fill (i32.const 8192) (i32.const 165) (i32.const 8192))
    (if (i32.eqz (local.get $mode)) (then
      (call $need (i32.eq (call $read (local.get $epoch) (i32.const 8193) (local.get $n)) (local.get $n)))
      return))
    ;; Real hosts preserve any import error, including a caught stale epoch.
    (if (i32.eq (local.get $mode) (i32.const 10)) (then
      (call $need (i32.eq (call $read (i64.sub (local.get $epoch) (i64.const 1)) (i32.const -1) (i32.const -1)) (i32.const -1)))
      return))
    (if (i32.eq (local.get $mode) (i32.const 1)) (then
      (call $need (i32.eq (call $read (local.get $epoch) (i32.const 8192) (i32.sub (local.get $n) (i32.const 1))) (i32.const -1)))
      (call $need (i32.eq (i32.load (i32.const 8192)) (i32.const -1515870811))) return))
    (if (i32.eq (local.get $mode) (i32.const 2)) (then
      (call $need (i32.eq (call $read (local.get $epoch) (i32.const 65530) (local.get $n)) (i32.const -1))) return))
    (if (i32.eq (local.get $mode) (i32.const 3)) (then
      (call $need (i32.eq (call $control (local.get $epoch) (i32.const -1) (i32.const 32)) (i32.const -1))) return))
    (if (i32.eq (local.get $mode) (i32.const 4)) (then
      (call $need (i32.eq (call $control (local.get $epoch) (i32.const 1024) (i32.const 2147483647)) (i32.const -1))) return))
    (if (i32.eq (local.get $mode) (i32.const 5)) (then (i32.store (i32.const 1040) (i32.const 3))))
    (if (i32.eq (local.get $mode) (i32.const 6)) (then (i32.store (i32.const 1044) (i32.const 2))))
    (if (i32.eq (local.get $mode) (i32.const 7)) (then (i32.store (i32.const 1052) (i32.const 1))))
    (if (i32.eq (local.get $mode) (i32.const 8)) (then
      (i32.store (i32.const 1040) (i32.const 0))
      (i32.store (i32.const 1044) (i32.const 0))
      (i32.store (i32.const 1048) (i32.const 1))
      (i32.store8 (i32.const 1056) (i32.const 255))
      (call $need (i32.eq (call $control (local.get $epoch) (i32.const 1024) (i32.const 33)) (i32.const -1))) return))
    (if (i32.eq (local.get $mode) (i32.const 9)) (then
      (loop $calls
        (drop (call $control (local.get $epoch) (i32.const 1024) (i32.const 32)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br_if $calls (i32.lt_u (local.get $i) (i32.const 256)))) return))
    (call $need (i32.eq (call $control (local.get $epoch) (i32.const 1024) (i32.const 32)) (i32.const -1)))
  )
)
