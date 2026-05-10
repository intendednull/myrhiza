//! Wasmtime `Config` determinism freeze per determinism.md §5.2 + §5.3.
//!
//! /// Covers: determinism.md §5.2, determinism.md §5.3
//!
//! Asserts the engine built from `deterministic_config()` refuses to compile
//! a component using `v128.const` (SIMD). Wasmtime's defaults shift across
//! LTS bumps; pinning each feature flag is the only way to keep replay
//! determinism stable. If a future wasmtime release flips a default and we
//! forget to pin it, this test catches it.
//!
//! The test deliberately constructs an `Engine` directly from
//! `deterministic_config()` rather than going through `WasmtimeBackend::new`,
//! so it can call `Component::new` without dragging in manifest gating /
//! float-ban / linker wiring. Float-ban would also reject the SIMD bytes,
//! so a backend-level probe could not isolate the engine config.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_wasmtime_backend::deterministic_config;
use wasmtime::{Engine, component::Component};

#[test]
fn engine_config_pins_deterministic_features_simd_rejected() {
    // A component whose core module exports a function returning a v128.
    // Compiles cleanly under wasmtime defaults (SIMD on by default in 36.x);
    // must fail under our pinned config.
    let simd_wat = r#"
        (component
          (core module
            (func (export "f") (result v128)
              v128.const i32x4 0 0 0 0)))
    "#;
    let bytes = wat::parse_str(simd_wat).expect("wat parses SIMD component");

    let config = deterministic_config();
    let engine = Engine::new(&config).expect("engine builds from deterministic config");

    let result = Component::new(&engine, &bytes);
    assert!(
        result.is_err(),
        "engine compiled SIMD component despite deterministic config; \
         wasmtime default likely changed — re-pin wasm_simd(false)"
    );
}
