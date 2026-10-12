(module
  (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
  (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
  (import "incant_log_v1" "write" (func $log (param i64 i32 i32 i32)))
  (import "incant_navigation_v1" "find_path" (func $path (param i64 i32 i32) (result i32)))
  (import "incant_navigation_v1" "find_grid_path" (func $query (param i64 i32 i32) (result i32)))
  (import "incant_navigation_v1" "read_path" (func $result (param i64 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 0) "\49\4e\43\47\52\44\51\31\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\01\00\00\00\00\00\00\00\00\02\00\00\00\00\00\00\00\64\00\00\00\01\00\00\00")
  (data (i32.const 2048) "\49\4e\43\50\54\48\51\31\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\01\00\00\00\00\00\00\20\c0\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\20\40\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\f0\3f\e8\03\00\00\00\00\00\00")
  (func (export "incant_abi") (result i32) i32.const 1)
  (func (export "update") (param $epoch i64) (param $dt f64) (local $mode i32) (local $len i32)
    local.get $dt f64.const 60 f64.mul f64.nearest i32.trunc_f64_u local.set $mode
    local.get $epoch i32.const 0 i32.const 512 call $read
    i32.const 512 f64.const 99 f64.store
    local.get $epoch i32.const 0 i32.const 512 call $write
    local.get $epoch i32.const 1 i32.const 0 i32.const 0 call $log
    local.get $mode i32.const 2 i32.eq if
      local.get $epoch i32.const -1 i32.const 48 call $query drop return end
    local.get $mode i32.const 3 i32.eq if
      local.get $epoch i32.const 0 i32.const -1 call $query drop return end
    local.get $mode i32.const 4 i32.eq if
      local.get $epoch i32.const 0 i32.const 49 call $query drop return end
    local.get $mode i32.const 5 i32.eq if
      local.get $epoch i64.const 1 i64.sub i32.const 0 i32.const 48 call $query drop return end
    local.get $mode i32.const 6 i32.eq if
      i32.const 0 i32.const 0 i32.store8 end
    local.get $mode i32.const 8 i32.eq if
      local.get $epoch i32.const 128 i32.const 128 call $result drop return end
    local.get $mode i32.const 9 i32.eq if
      i32.const 44 i32.const 2 i32.store end
    local.get $mode i32.const 11 i32.eq if
      i32.const 32 i32.const 65536 i32.store end
    local.get $mode i32.const 12 i32.eq if
      i32.const 2072 f64.const 100000.000001 f64.store
      local.get $epoch i32.const 2048 i32.const 88 call $path drop return end
    local.get $mode i32.const 13 i32.eq if
      i32.const 2120 f64.const 0.000999999999 f64.store
      local.get $epoch i32.const 2048 i32.const 88 call $path drop return end
    local.get $mode i32.const 14 i32.eq if
      i32.const 2072 f64.const -100000 f64.store
      i32.const 2096 f64.const 100000 f64.store
      i32.const 2120 f64.const 0.001 f64.store
      local.get $epoch i32.const 2048 i32.const 88 call $path local.set $len
      local.get $epoch i32.const 129 local.get $len call $result drop
      local.get $len i32.const 40 i32.ne if unreachable end return end
    local.get $mode i32.const 15 i32.eq if
      local.get $epoch i32.const 0 i32.const 48 call $query drop
      local.get $epoch i32.const 2048 i32.const 88 call $path local.set $len
      local.get $epoch i32.const 129 local.get $len call $result drop
      i32.const 129 i64.load i64.const 3553982582909783625 i64.ne if unreachable end end
    local.get $epoch i32.const 0 i32.const 48 call $query local.set $len
    local.get $mode i32.const 7 i32.eq if
      local.get $epoch i32.const 128 i32.const 1 call $result drop return end
    local.get $mode i32.const 10 i32.eq if
      local.get $epoch i32.const 65530 local.get $len call $result drop return end
    local.get $epoch i32.const 129 local.get $len call $result drop
    i32.const 129 i64.load i64.const 3553978176122342985 i64.ne if unreachable end))
