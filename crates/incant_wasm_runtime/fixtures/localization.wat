(module
 (import "incant_localization_v1" "locale" (func $locale (param i64) (result i32)))
 (import "incant_localization_v1" "localize" (func $localize (param i64 i32 i32) (result i32)))
 (import "incant_localization_v1" "read_result" (func $read (param i64 i32 i32) (result i32)))
 (import "incant_localization_v1" "set_locale" (func $set (param i64 i32 i32)))
 (import "incant_v1" "read_transforms" (func $rt (param i64 i32 i32)))
 (import "incant_v1" "write_transforms" (func $wt (param i64 i32 i32)))
 (import "incant_v1" "spawn" (func $spawn (param i64 i32 i32 i32 i32)))
 (import "incant_log_v1" "write" (func $log (param i64 i32 i32 i32)))
 (memory (export "memory") 1)
 (data (i32.const 1024) "\49\4e\43\4c\53\54\30\31\01\00\00\00\02\00\00\00\66\72\00\00\00\00\00\00\00\00")
 (data (i32.const 2048) "\49\4e\43\4c\51\52\30\31\01\00\00\00\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\30\32\05\00\00\00\68\65\6c\6c\6f\01\00\00\00\04\00\00\00\6e\61\6d\65\00\00\00\00\07\00\00\00\e9\9b\aa\f0\9f\99\82")
 (data (i32.const 3072) "\49\4e\43\4c\53\54\30\31\01\00\00\00\05\00\00\00\45\4e\5f\75\73\00\00\00\00\00\00\00\00")
 (data (i32.const 4000) "\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\4d")
 (data (i32.const 4100) "staged")
 (func (export "incant_abi") (result i32) i32.const 1)
 (func (export "update") (param $e i64) (param $dt f64) (local $mode i32) (local $len i32) (local $i i32)
  local.get $dt f64.const 60 f64.mul i32.trunc_f64_u local.set $mode
  local.get $e i32.const 0 i32.const 0 call $rt
  i32.const 0 f64.const 42 f64.store
  local.get $e i32.const 0 i32.const 0 call $wt
  local.get $e i32.const 1024 i32.const 26 call $set
  local.get $e i32.const 4000 i32.const -1 i32.const 0 i32.const -1 call $spawn
  local.get $e i32.const 1 i32.const 4100 i32.const 6 call $log
  local.get $mode i32.const 1 i32.eq (if (then
    local.get $e i32.const 2048 i32.const 74 call $localize local.set $len
    local.get $e i32.const 8193 local.get $len call $read drop
  ))
  local.get $mode i32.const 2 i32.eq (if (then unreachable))
  local.get $mode i32.const 3 i32.eq (if (then local.get $e i32.const 2048 i32.const 0 call $localize drop))
  local.get $mode i32.const 4 i32.eq (if (then
    local.get $e call $locale local.set $len
    local.get $e i32.const 8193 local.get $len i32.const 1 i32.sub call $read drop
  ))
  local.get $mode i32.const 5 i32.eq (if (then local.get $e i32.const 3072 i32.const 29 call $set))
  local.get $mode i32.const 6 i32.eq (if (then local.get $e i64.const 1 i64.sub call $locale drop))
  local.get $mode i32.const 7 i32.eq (if (then local.get $e i32.const 65530 i32.const 100 call $localize drop))
  local.get $mode i32.const 8 i32.eq (if (then
    local.get $e call $locale local.set $len
    local.get $e i32.const 2147483647 local.get $len call $read drop
  ))
  local.get $mode i32.const 9 i32.eq (if (then
    (loop $loop local.get $e call $locale drop local.get $i i32.const 1 i32.add local.tee $i i32.const 65 i32.lt_u br_if $loop)
  ))
  local.get $mode i32.const 10 i32.eq (if (then local.get $e i32.const 8193 i32.const 100 call $read drop))
  local.get $mode i32.const 11 i32.eq (if (then
    local.get $e call $locale drop
    local.get $e i32.const 2048 i32.const 75 call $localize drop
  ))
  local.get $mode i32.const 12 i32.eq (if (then
    ;; Superseded invalid settings are rejected before a later valid setting.
    local.get $e i32.const 3072 i32.const 29 call $set
    local.get $e i32.const 1024 i32.const 26 call $set
  ))
 ))
