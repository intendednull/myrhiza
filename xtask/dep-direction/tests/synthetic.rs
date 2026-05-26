//! Unit tests for [`dep_direction_check::check_dep_direction`].
//!
//! Per `docs/specs/2026-05-26-b-8-sdk-design.md` §3.4 + §5.3: these
//! tests construct synthetic [`Metadata`] from hand-rolled JSON
//! (matching `cargo metadata --format-version=1`'s schema) so the
//! check function can be exercised without mutating real `Cargo.toml`
//! files. This is the testing-anti-patterns-safe shape: we test the
//! check's contract, not by polluting the live workspace.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use cargo_metadata::Metadata;
use dep_direction_check::check_dep_direction;
use serde_json::{Value, json};

/// Build one `Package` JSON object with the minimum fields the
/// `cargo_metadata` deserializer demands (no `#[serde(default)]`).
///
/// `manifest_path` decides whether the package is recognised as an
/// example (the check matches `components().any(== "examples")`).
fn package(name: &str, id: &str, manifest_path: &str) -> Value {
    json!({
        "name": name,
        "version": "0.1.0",
        "id": id,
        "source": null,
        "description": null,
        "dependencies": [],
        "license": null,
        "license_file": null,
        "targets": [],
        "features": {},
        "manifest_path": manifest_path,
        "readme": null,
        "repository": null,
        "homepage": null,
        "documentation": null,
        "links": null,
        "publish": null,
        "default_run": null,
    })
}

/// Build a `Node` JSON object. `deps` is a list of
/// `(dep_name, dep_id, kind)` tuples where `kind` is one of
/// `"normal"`, `"dev"`, `"build"`.
fn node(id: &str, deps: &[(&str, &str, &str)]) -> Value {
    let deps_json: Vec<Value> = deps
        .iter()
        .map(|(name, pkg, kind)| {
            json!({
                "name": name,
                "pkg": pkg,
                "dep_kinds": [{ "kind": kind, "target": null }],
            })
        })
        .collect();
    let dep_ids: Vec<&str> = deps.iter().map(|(_, id, _)| *id).collect();
    json!({
        "id": id,
        "deps": deps_json,
        "dependencies": dep_ids,
        "features": [],
    })
}

/// Assemble the full `Metadata` JSON envelope around the given
/// packages and resolve nodes.
fn metadata(packages: &[Value], nodes: &[Value], root: &str) -> Metadata {
    let workspace_members: Vec<String> = packages
        .iter()
        .map(|p| {
            p.get("id")
                .and_then(Value::as_str)
                .expect("package missing id")
                .to_string()
        })
        .collect();
    let envelope = json!({
        "packages": packages,
        "workspace_members": workspace_members,
        "workspace_default_members": [],
        "resolve": {
            "nodes": nodes,
            "root": root,
        },
        "target_directory": "/tmp/target",
        "workspace_root": "/tmp",
        "metadata": null,
        "version": 1,
    });
    serde_json::from_value(envelope).expect("synthetic metadata must deserialize")
}

const COUNTER_ID: &str = "path+file:///tmp#counter-example@0.1.0";
const COUNTER_MANIFEST: &str = "/tmp/examples/counter/Cargo.toml";
const SDK_ID: &str = "path+file:///tmp#myrhiza-sdk@0.1.0";
const SDK_MANIFEST: &str = "/tmp/crates/sdk/Cargo.toml";
const TYPES_ID: &str = "path+file:///tmp#myrhiza-types@0.1.0";
const TYPES_MANIFEST: &str = "/tmp/crates/types/Cargo.toml";
const KERNEL_ID: &str = "path+file:///tmp#myrhiza-kernel@0.1.0";
const KERNEL_MANIFEST: &str = "/tmp/crates/kernel/Cargo.toml";

#[test]
fn clean_graph_returns_no_violations() {
    // examples/counter -> myrhiza-sdk -> myrhiza-types
    // No FORBIDDEN edges anywhere.
    let m = metadata(
        &[
            package("counter-example", COUNTER_ID, COUNTER_MANIFEST),
            package("myrhiza-sdk", SDK_ID, SDK_MANIFEST),
            package("myrhiza-types", TYPES_ID, TYPES_MANIFEST),
        ],
        &[
            node(COUNTER_ID, &[("myrhiza_sdk", SDK_ID, "normal")]),
            node(SDK_ID, &[("myrhiza_types", TYPES_ID, "normal")]),
            node(TYPES_ID, &[]),
        ],
        COUNTER_ID,
    );

    let violations = check_dep_direction(&m);
    assert!(
        violations.is_empty(),
        "expected no violations, got: {violations:?}"
    );
}

#[test]
fn forbidden_edge_returns_violation() {
    // examples/counter -> myrhiza-kernel (direct violation).
    let m = metadata(
        &[
            package("counter-example", COUNTER_ID, COUNTER_MANIFEST),
            package("myrhiza-kernel", KERNEL_ID, KERNEL_MANIFEST),
        ],
        &[
            node(COUNTER_ID, &[("myrhiza_kernel", KERNEL_ID, "normal")]),
            node(KERNEL_ID, &[]),
        ],
        COUNTER_ID,
    );

    let violations = check_dep_direction(&m);
    assert_eq!(
        violations.len(),
        1,
        "expected 1 violation, got {violations:?}"
    );
    assert!(
        violations[0].contains("myrhiza-kernel"),
        "diagnostic should name the forbidden crate, got: {}",
        violations[0]
    );
    assert!(
        violations[0].contains("counter-example"),
        "diagnostic should name the offending example, got: {}",
        violations[0]
    );
}

#[test]
fn transitive_forbidden_edge_returns_violation() {
    // examples/counter -> myrhiza-sdk -> myrhiza-kernel
    // Counter doesn't name kernel directly, but reaches it via sdk.
    let m = metadata(
        &[
            package("counter-example", COUNTER_ID, COUNTER_MANIFEST),
            package("myrhiza-sdk", SDK_ID, SDK_MANIFEST),
            package("myrhiza-kernel", KERNEL_ID, KERNEL_MANIFEST),
        ],
        &[
            node(COUNTER_ID, &[("myrhiza_sdk", SDK_ID, "normal")]),
            node(SDK_ID, &[("myrhiza_kernel", KERNEL_ID, "normal")]),
            node(KERNEL_ID, &[]),
        ],
        COUNTER_ID,
    );

    let violations = check_dep_direction(&m);
    assert_eq!(
        violations.len(),
        1,
        "expected 1 violation, got {violations:?}"
    );
    assert!(
        violations[0].contains("myrhiza-kernel"),
        "diagnostic should name the forbidden crate, got: {}",
        violations[0]
    );
}
