#![cfg(not(target_family = "wasm"))]

use rapier2d::{parry, rayon};

include!("../../../tools/probes/parry-parallel-tests.rs");
