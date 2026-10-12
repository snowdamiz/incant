(module
 (import "incant_localization_v1" "format" (func $format (param i64 i32 i32) (result i32)))
 (import "incant_localization_v1" "read_result" (func $read (param i64 i32 i32) (result i32)))
 (import "incant_localization_v1" "set_locale" (func $set (param i64 i32 i32)))
 (import "incant_v1" "chunk_count" (func $count (param i64) (result i32)))
 (import "incant_v1" "read_transforms" (func $rt (param i64 i32 i32)))
 (import "incant_v1" "write_transforms" (func $wt (param i64 i32 i32)))
 (import "incant_v1" "spawn" (func $spawn (param i64 i32 i32 i32 i32)))
 (import "incant_log_v1" "write" (func $log (param i64 i32 i32 i32)))
 (memory (export "memory") 1)
 (data (i32.const 1025) "\49\4e\43\4c\46\4d\30\31\01\00\00\00\00\00\00\00\00\00\00\00\00\4a\93\c0")
 (data (i32.const 1057) "\49\4e\43\4c\46\4d\30\31\01\00\00\00\01\00\00\00\e8\07\00\00\02\1d\02\00")
 (data (i32.const 1536) "\49\4e\43\4c\53\54\30\31\01\00\00\00\02\00\00\00\66\72\00\00\00\00\01\00\00\00")
 (data (i32.const 2000) "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\4d")
 (data (i32.const 2100) "\73\74\61\67\65\64")
 (func (export "incant_abi") (result i32) i32.const 1)
 (func $output (param $e i64) (param $ptr i32) (local $n i32)
  local.get $e local.get $ptr i32.const 24 call $format local.set $n
  local.get $e i32.const 8193 local.get $n call $read drop
  local.get $e i32.const 1 i32.const 8213 i32.const 8209 i32.load call $log)
 (func (export "update") (param $e i64) (param $dt f64) (local $mode i32) (local $n i32) (local $i i32)
  local.get $dt f64.const 60 f64.mul f64.nearest i32.trunc_f64_u local.set $mode
  local.get $e i32.const 0 i32.const 0 call $rt
  i32.const 0 f64.const 42 f64.store
  local.get $e i32.const 0 i32.const 0 call $wt
  local.get $e i32.const 1536 i32.const 26 call $set
  local.get $e i32.const 2000 i32.const -1 i32.const 0 i32.const -1 call $spawn
  local.get $mode i32.const 1 i32.eq (if (then
    local.get $e i32.const 1025 call $output
    local.get $e i32.const 1057 call $output
  ) (else
    local.get $e i32.const 1 i32.const 2100 i32.const 6 call $log
  ))
  local.get $mode i32.const 2 i32.eq (if (then unreachable))
  local.get $mode i32.const 3 i32.eq (if (then i32.const 1037 i32.const 9 i32.store))
  local.get $mode i32.const 4 i32.eq (if (then i32.const 1079 i32.const 3 i32.store8))
  local.get $mode i32.const 5 i32.eq (if (then i32.const 1080 i32.const 1 i32.store8))
  local.get $mode i32.const 6 i32.eq (if (then i32.const 1041 f64.const nan f64.store))
  local.get $mode i32.const 7 i32.eq (if (then i32.const 1041 f64.const inf f64.store))
  local.get $mode i32.const 8 i32.eq (if (then i32.const 1041 f64.const 9007199254740992 f64.store))
  local.get $mode i32.const 9 i32.eq (if (then i32.const 1073 i32.const 1900 i32.store))
  local.get $mode i32.const 10 i32.eq (if (then
   local.get $e i32.const 1025 i32.const 24 call $format local.set $n
   local.get $e i32.const 8193 local.get $n i32.const 1 i32.sub call $read drop))
  local.get $mode i32.const 11 i32.eq (if (then local.get $e i32.const 65520 i32.const 24 call $format drop))
  local.get $mode i32.const 12 i32.eq (if (then
   local.get $e i32.const 1025 i32.const 24 call $format local.set $n
   local.get $e i32.const 65520 local.get $n call $read drop))
  local.get $mode i32.const 13 i32.eq (if (then local.get $e i32.const 1025 i32.const 25 call $format drop))
  local.get $mode i32.const 14 i32.eq (if (then local.get $e i64.const 1 i64.add i32.const 1025 i32.const 24 call $format drop))
  local.get $mode i32.const 15 i32.eq (if (then
   (loop $q local.get $e i32.const 1025 i32.const 24 call $format drop
    local.get $i i32.const 1 i32.add local.tee $i i32.const 65 i32.lt_u br_if $q)))
  local.get $mode i32.const 16 i32.eq (if (then
   (loop $c local.get $e call $count drop
    local.get $i i32.const 1 i32.add local.tee $i i32.const 65531 i32.lt_u br_if $c)))
  local.get $mode i32.const 17 i32.eq (if (then i32.const 1073 i32.const -10000 i32.store))
  local.get $mode i32.const 18 i32.eq (if (then i32.const 1033 i32.const 2 i32.store))
  local.get $mode i32.const 19 i32.eq (if (then local.get $e i32.const 8193 i32.const 1024 call $read drop))
  local.get $mode i32.const 1 i32.ne (if (then
   local.get $e
   local.get $mode i32.const 4 i32.ge_u local.get $mode i32.const 5 i32.le_u i32.and
   local.get $mode i32.const 9 i32.eq i32.or local.get $mode i32.const 17 i32.eq i32.or
   (if (result i32) (then i32.const 1057) (else i32.const 1025))
   i32.const 24 call $format drop))
 ))
