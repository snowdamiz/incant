(module
  (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
  (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
  (import "incant_log_v1" "write" (func $log (param i64 i32 i32 i32)))
  (import "incant_navigation_v1" "steer" (func $steer (param i64 i32 i32) (result i32)))
  (import "incant_navigation_v1" "read_steering" (func $result (param i64 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "INCSTR01")
  (func (export "incant_abi") (result i32) i32.const 1)
  (func (export "update") (param $epoch i64) (param $dt f64) (local $mode i32) (local $len i32)
    local.get $dt f64.const 60 f64.mul f64.nearest i32.trunc_f64_u local.set $mode
    local.get $epoch i32.const 0 i32.const 512 call $read
    i32.const 512 f64.const 99 f64.store
    local.get $epoch i32.const 0 i32.const 512 call $write
    local.get $epoch i32.const 1 i32.const 0 i32.const 0 call $log
    i32.const 16 f64.const 1 f64.store
    i32.const 24 f64.const 1 f64.store
    local.get $mode i32.const 2 i32.eq if
      local.get $epoch i32.const -1 i32.const 40 call $steer drop return end
    local.get $mode i32.const 3 i32.eq if
      local.get $epoch i32.const 0 i32.const -1 call $steer drop return end
    local.get $mode i32.const 4 i32.eq if
      local.get $epoch i32.const 0 i32.const 17193 call $steer drop return end
    local.get $mode i32.const 5 i32.eq if
      local.get $epoch i64.const 1 i64.sub i32.const 0 i32.const 40 call $steer drop return end
    local.get $mode i32.const 6 i32.eq if
      i32.const 0 i32.const 0 i32.store8 end
    local.get $mode i32.const 8 i32.eq if
      local.get $epoch i32.const 128 i32.const 16 call $result drop return end
    local.get $mode i32.const 9 i32.eq if
      i32.const 8 i32.const 2 i32.store end
    local.get $mode i32.const 10 i32.eq if
      i32.const 32 f64.const nan f64.store end
    local.get $epoch i32.const 0 i32.const 40 call $steer local.set $len
    local.get $mode i32.const 7 i32.eq if
      local.get $epoch i32.const 128 i32.const 15 call $result drop return end
    local.get $epoch i32.const 129 local.get $len call $result drop))
