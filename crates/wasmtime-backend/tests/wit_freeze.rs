//! WIT/ABI freeze test per verification.md §22.3.
//!
//! /// Covers: architecture.md §3.5, distribution.md §10.2, verification.md §22.3
//!
//! Re-runs `wit-parser` over the canonical kernel WIT package and renders
//! a textual dump of the state-apply world's imports/exports. Asserts
//! byte-equality with the committed snapshot. Drift fails CI; accepting
//! drift requires updating the snapshot AND a kernel-major version bump
//! per distribution.md §10.2.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::items_after_statements
)]

const SNAPSHOT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/snapshots/state-apply-world.bindgen.txt"
);

#[test]
fn state_apply_world_bindings_match_snapshot() {
    let generated = generate_state_apply_bindings();
    let Ok(expected) = std::fs::read_to_string(SNAPSHOT_PATH) else {
        // First run: write the snapshot. Subsequent runs assert.
        std::fs::write(SNAPSHOT_PATH, &generated).expect("write initial snapshot");
        panic!("WIT-freeze snapshot bootstrapped at {SNAPSHOT_PATH}. Re-run the test.");
    };
    assert_eq!(
        generated, expected,
        "WIT/ABI drift detected. Either:\n\
         1. Revert the WIT change, or\n\
         2. Bump kernel-major + update {SNAPSHOT_PATH} by deleting it and re-running this test.",
    );
}

fn generate_state_apply_bindings() -> String {
    // The runtime uses `wasmtime::component::bindgen!()` in
    // `crates/wasmtime-backend/src/engine.rs`. To freeze the generated
    // surface, we invoke `wit-parser` directly to build a textual
    // representation of the resolved world, which is what the WIT
    // package promises to consumers. `wit-parser`'s `Resolve` renders
    // canonically.
    //
    // This is preferable to expanding the `bindgen!` macro (which
    // depends on wasmtime version + macro implementation details
    // that drift across patch releases for reasons orthogonal to
    // ABI semantics).

    let mut resolve = wit_parser::Resolve::new();
    let (pkg_id, _src_map) = resolve
        .push_dir(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../wit/myrhiza-kernel/wit"
        )))
        .expect("parse WIT package");
    let world_id = resolve
        .select_world(pkg_id, Some("state-apply"))
        .expect("select state-apply world");
    let world = &resolve.worlds[world_id];

    let mut out = String::new();
    use std::fmt::Write as _;
    writeln!(out, "world {} {{", world.name).unwrap();
    writeln!(out, "  imports:").unwrap();
    for (name, item) in &world.imports {
        let key = match name {
            wit_parser::WorldKey::Name(n) => n.clone(),
            wit_parser::WorldKey::Interface(id) => resolve
                .id_of(*id)
                .unwrap_or_else(|| format!("interface#{id:?}")),
        };
        writeln!(out, "    {key}: {item:?}").unwrap();
    }
    writeln!(out, "  exports:").unwrap();
    for (name, item) in &world.exports {
        let key = match name {
            wit_parser::WorldKey::Name(n) => n.clone(),
            wit_parser::WorldKey::Interface(id) => resolve
                .id_of(*id)
                .unwrap_or_else(|| format!("interface#{id:?}")),
        };
        writeln!(out, "    {key}: {item:?}").unwrap();
    }
    writeln!(out, "}}").unwrap();
    out
}
