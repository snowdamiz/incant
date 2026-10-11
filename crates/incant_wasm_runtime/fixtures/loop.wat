(module
(import "incant_v1" "chunk_count" (func $count (param i64) (result i32)))
(import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
(import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
(import "incant_v1" "spawn" (func $spawn (param i64 i32 i32 i32 i32)))
(import "incant_v1" "set_parent" (func $parent (param i64 i32 i32)))
(import "incant_v1" "set_velocity" (func $velocity (param i64 i32 i32)))
(memory (export "memory") 4)
(func (export "incant_abi") (result i32) (i32.const 1))

(func (export "update") (param $e i64) (param f64) (local $n i32) (loop $loop (br $loop))))
