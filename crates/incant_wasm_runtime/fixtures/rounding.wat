(module
(import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
(import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
(memory (export "memory") 1)
(func (export "incant_abi") (result i32) i32.const 1)
(func (export "update") (param $e i64) (param f64)
(call $read (local.get $e) (i32.const 0) (i32.const 0))
(f64.store (i32.const 0) (f64.add (f64.mul (f64.load (i32.const 0)) (f64.load (i32.const 8))) (f64.const -1)))
(call $write (local.get $e) (i32.const 0) (i32.const 0))))
