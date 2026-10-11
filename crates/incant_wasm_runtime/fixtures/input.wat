(module
  (import "incant_input_v1" "snapshot_len" (func $len (param i64) (result i32)))
  (import "incant_input_v1" "read_snapshot" (func $input (param i64 i32 i32) (result i32)))
  (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
  (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
  (memory (export "memory") 4 4)
  (func (export "incant_abi") (result i32) i32.const 1)
  (func (export "update") (param $e i64) (param f64) (local $n i32) (local $i i32) (local $hash i32)
    local.get $e call $len local.set $n
    local.get $e i32.const 1024 local.get $n call $input local.get $n i32.ne if unreachable end
    i32.const -2128831035 local.set $hash
    block $done loop $bytes
      local.get $i local.get $n i32.ge_u br_if $done
      local.get $hash local.get $i i32.const 1024 i32.add i32.load8_u i32.xor
      i32.const 16777619 i32.mul local.set $hash
      local.get $i i32.const 1 i32.add local.set $i br $bytes
    end end
    local.get $e i32.const 0 i32.const 0 call $read
    i32.const 0 local.get $hash f64.convert_i32_u f64.store
    i32.const 8 local.get $n f64.convert_i32_u f64.store
    i32.const 16 i32.const 1040 i64.load f64.convert_i64_u f64.store
    local.get $e i32.const 0 i32.const 0 call $write))
