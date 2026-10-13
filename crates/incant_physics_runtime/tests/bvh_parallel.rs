#![cfg(not(target_family = "wasm"))]

use rapier3d::{parry, rayon};

include!("../../../tools/probes/parry-parallel-tests.rs");
