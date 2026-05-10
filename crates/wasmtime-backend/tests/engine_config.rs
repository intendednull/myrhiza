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

/// Tail-call `return_call` must be rejected by the engine. Default for
/// `wasm_tail_call` differs across cranelift backends (on for
/// `x86_64` / `aarch64` / `riscv64`, off for s390x / Winch in wasmtime
/// 36) — a silent cross-arch divergence vector. The deterministic
/// config pins it off so every architecture rejects the same set of
/// components.
#[test]
fn engine_config_pins_deterministic_features_tail_call_rejected() {
    let tail_call_wat = r#"
        (component
          (core module
            (func $g (param i32) (result i32) local.get 0)
            (func (export "f") (param i32) (result i32)
              local.get 0
              return_call $g)))
    "#;
    let bytes = wat::parse_str(tail_call_wat).expect("wat parses tail-call component");

    let config = deterministic_config();
    let engine = Engine::new(&config).expect("engine builds from deterministic config");

    let result = Component::new(&engine, &bytes);
    assert!(
        result.is_err(),
        "engine compiled tail-call component despite deterministic config; \
         wasmtime default likely changed (or cross-arch default differs) — \
         re-pin wasm_tail_call(false)"
    );
}

/// Extended-const constant expressions in globals must be rejected.
/// `wasm_extended_const` is on by default in wasmtime 36's WASM2
/// baseline; pinning it off keeps the v1 const-expr surface tight so
/// a future LTS bump cannot quietly admit new global initializers
/// into the deterministic accept set.
#[test]
fn engine_config_pins_deterministic_features_extended_const_rejected() {
    // A core module with a global initialized via `i32.add` of two
    // constants — that's exactly the extended-const proposal: arbitrary
    // constant expressions in initializers, not just a single
    // `*.const`. Under MVP rules the engine rejects this at validation.
    let extended_const_wat = r"
        (component
          (core module
            (global i32 (i32.add (i32.const 1) (i32.const 2)))))
    ";
    let bytes = wat::parse_str(extended_const_wat).expect("wat parses extended-const component");

    let config = deterministic_config();
    let engine = Engine::new(&config).expect("engine builds from deterministic config");

    let result = Component::new(&engine, &bytes);
    assert!(
        result.is_err(),
        "engine compiled extended-const global initializer despite deterministic config; \
         wasmtime default likely changed — re-pin wasm_extended_const(false)"
    );
}
