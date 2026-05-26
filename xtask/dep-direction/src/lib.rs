//! Dependency-direction check.
//!
//! Asserts that no workspace member under `examples/*` transitively
//! depends on any kernel-internal crate.
//!
//! Per `docs/specs/2026-05-26-b-8-sdk-design.md` §2.4: example crates
//! may depend on `myrhiza-sdk` (and third-party crates) but MUST NOT
//! name any kernel-internal crate in their dependency graph — direct
//! or transitive. The SDK transitively pulls in `myrhiza-types` and
//! `myrhiza-manifest` (which it re-exports); that is permitted.
//!
//! ## Dev-dep filtering
//!
//! `cargo metadata`'s resolve graph includes edges of every kind
//! (normal / build / dev). A dev-dep of a *transitive* dependency
//! does NOT propagate into the consumer's link graph — e.g. when
//! `examples/counter` depends on `myrhiza-sdk → myrhiza-types`, the
//! fact that `myrhiza-types` has a dev-dep on `myrhiza-network` is
//! irrelevant; `cargo build -p counter-example` never links it. We
//! therefore walk only **normal** and **build** edges. (Build deps
//! are included because a forbidden crate appearing in a build
//! script's host-side graph is still a leak of host concerns into
//! an example.)
//!
//! Dev-deps on the example crate *itself* would normally be a
//! relevant case to flag (since `cargo test -p counter-example`
//! compiles them), but no example crate currently declares dev-deps
//! and the cleanest invariant is "the link graph of the published
//! example binary must not reach kernel-internals" — keep the rule
//! uniform across all hops.

#![deny(missing_docs)]

use std::collections::{HashMap, HashSet, VecDeque};

use cargo_metadata::{DependencyKind, Metadata, PackageId};

/// Crates `examples/*` MUST NOT depend on, directly or transitively.
///
/// Per spec §2.4. `myrhiza-distribution` is listed here ahead of the
/// B-10 (iroh-blobs distribution) landing so the gate is closed when
/// that crate appears in the workspace.
pub const FORBIDDEN: &[&str] = &[
    "myrhiza-kernel",
    "myrhiza-backend",
    "myrhiza-wasmtime-backend",
    "myrhiza-network",
    "myrhiza-test-utils",
    "myrhiza-cli",
    "myrhiza-distribution",
];

/// Walk the package graph; return one diagnostic per violating edge.
///
/// For each workspace member whose manifest lives under an `examples/`
/// segment, performs a BFS over the resolved dependency graph
/// (filtered to non-dev edges — see module docs) and emits a string
/// for every node whose package name matches [`FORBIDDEN`]. The
/// returned vector is sorted + deduped so the output is stable across
/// runs (the cargo-metadata graph is `HashMap`-backed internally, so
/// iteration order is otherwise unstable).
#[must_use]
pub fn check_dep_direction(metadata: &Metadata) -> Vec<String> {
    let mut violations = Vec::new();
    let id_to_name: HashMap<&PackageId, &str> = metadata
        .packages
        .iter()
        .map(|p| (&p.id, p.name.as_str()))
        .collect();
    let Some(resolve) = metadata.resolve.as_ref() else {
        return vec!["error: cargo_metadata returned no resolve graph".into()];
    };
    let node_by_id: HashMap<&PackageId, &cargo_metadata::Node> =
        resolve.nodes.iter().map(|n| (&n.id, n)).collect();

    // For each examples/* member, BFS its non-dev transitive deps
    // and check if any are in the FORBIDDEN list.
    for package in &metadata.packages {
        let manifest_path = package.manifest_path.as_std_path();
        let in_examples = manifest_path
            .components()
            .any(|c| c.as_os_str() == "examples");
        if !in_examples {
            continue;
        }
        let example_name = package.name.as_str();
        let mut visited: HashSet<&PackageId> = HashSet::new();
        let mut queue: VecDeque<&PackageId> = VecDeque::new();
        queue.push_back(&package.id);
        visited.insert(&package.id);
        while let Some(id) = queue.pop_front() {
            let Some(node) = node_by_id.get(id) else {
                continue;
            };
            for dep in &node.deps {
                // Filter: skip pure dev-dep edges. An edge with
                // multiple kinds (e.g. "dep is both normal and dev")
                // counts if any non-dev kind is present.
                let is_link_edge = dep.dep_kinds.is_empty()
                    || dep
                        .dep_kinds
                        .iter()
                        .any(|k| k.kind != DependencyKind::Development);
                if !is_link_edge {
                    continue;
                }
                if visited.insert(&dep.pkg) {
                    queue.push_back(&dep.pkg);
                    if let Some(&name) = id_to_name.get(&dep.pkg)
                        && FORBIDDEN.contains(&name)
                    {
                        violations.push(format!(
                            "{example_name}: transitive dep on forbidden crate {name}"
                        ));
                    }
                }
            }
        }
    }
    violations.sort();
    violations.dedup();
    violations
}
