//! WIT/ABI freeze tests per verification.md §22.3.
//!
//! Re-runs `wit-parser` over the canonical kernel WIT package and renders
//! a textual dump of every world's imports/exports with name-resolved
//! types. Asserts byte-equality against per-world snapshot files. Drift
//! fails CI; accepting drift requires updating the snapshot AND a kernel-
//! major version bump per distribution.md §10.2.
//!
//! All four worlds (`state-apply`, `state-propose`, `interaction`,
//! `behavior`) are frozen — adding any of them post-hoc is an ABI change
//! that must be visible in review. Each world has its own `#[test]` so
//! the spec-coverage matrix surfaces one entry per world rather than
//! collapsing all four into a single line; if `state-propose`'s freeze
//! were quietly removed, the matrix would lose its row, which is the
//! observability §22.2 is built for.
//!
//! The state-apply test additionally carries the §22.2 self-coverage
//! annotation: §22.2 is the spec-coverage-matrix convention itself,
//! and the matrix is regenerated from doc-comment annotations on
//! tests. By pinning §22.2 to a load-bearing freeze test, removing
//! the freeze (or its annotation) makes the matrix lose its §22.2
//! row, which is the same fail-closed loop §22.2 prescribes for
//! everything else.
//!
//! Renderer notes:
//! - Functions render as `func name(p: type, ...) -> result-type` using
//!   the type's declared name when available, or a structural form
//!   (`list<u8>`, `tuple<a, b>`, `option<t>`, `result<o, e>`, etc.) for
//!   anonymous types.
//! - Snapshot uses *only* names + structural forms — no `Id { idx: N }`
//!   debug print, so renumbering caused by interface re-ordering does
//!   not produce diffs while still surfacing real ABI changes (added or
//!   removed functions, renamed params, changed types).
//! - Imports/exports are sorted within each world for determinism.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::items_after_statements
)]

use std::fmt::Write as _;

use wit_parser::{
    Function, Handle, Interface, Resolve, Tuple, Type, TypeDefKind, TypeId, World, WorldId,
    WorldItem,
};

/// Covers: architecture.md §3.5, distribution.md §10.2, verification.md §22.2, verification.md §22.3
///
/// Freezes the `state-apply` world's WIT/ABI surface against its
/// snapshot. Carries the §22.2 self-coverage annotation: §22.2 is the
/// spec-coverage-matrix convention, and this test's presence in the
/// generated matrix is itself the §22.2 evidence — drop the test or
/// drop the annotation, and the matrix loses its §22.2 row.
#[test]
fn freeze_state_apply_world() {
    assert_world_snapshot_matches("state-apply");
}

/// Covers: architecture.md §3.5, distribution.md §10.2, verification.md §22.3
#[test]
fn freeze_state_propose_world() {
    assert_world_snapshot_matches("state-propose");
}

/// Covers: architecture.md §3.5, distribution.md §10.2, verification.md §22.3
#[test]
fn freeze_interaction_world() {
    assert_world_snapshot_matches("interaction");
}

/// Covers: architecture.md §3.5, distribution.md §10.2, verification.md §22.3
#[test]
fn freeze_behavior_world() {
    assert_world_snapshot_matches("behavior");
}

/// Render the named world's WIT and compare it byte-for-byte against
/// `tests/snapshots/<world>-world.bindgen.txt`. Set
/// `MYRHIZA_SNAPSHOT_UPDATE=1` to regenerate the snapshot in place
/// after an intentional ABI change (which also requires a
/// kernel-major bump per distribution.md §10.2).
fn assert_world_snapshot_matches(world_name: &str) {
    let mut resolve = Resolve::new();
    let (pkg_id, _src_map) = resolve
        .push_dir(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../wit/myrhiza-kernel/wit"
        )))
        .expect("parse WIT package");

    let snapshot_path = format!(
        "{}/../../tests/snapshots/{}-world.bindgen.txt",
        env!("CARGO_MANIFEST_DIR"),
        world_name
    );
    let world_id = resolve
        .select_world(pkg_id, Some(world_name))
        .unwrap_or_else(|e| panic!("select world {world_name}: {e}"));
    let rendered = render_world(&resolve, world_id);

    if std::env::var("MYRHIZA_SNAPSHOT_UPDATE").is_ok() {
        std::fs::write(&snapshot_path, &rendered).expect("write snapshot");
        return;
    }

    let expected = std::fs::read_to_string(&snapshot_path).unwrap_or_else(|e| {
        panic!(
            "snapshot for world {world_name} missing at {snapshot_path}: {e}\n\
             Run with MYRHIZA_SNAPSHOT_UPDATE=1 to regenerate."
        )
    });
    assert_eq!(
        rendered, expected,
        "WIT/ABI drift for world {world_name}. Either:\n\
         1. Revert the WIT change, or\n\
         2. Bump kernel-major + regenerate {snapshot_path} via\n\
            MYRHIZA_SNAPSHOT_UPDATE=1 cargo test -p myrhiza-wasmtime-backend wit_freeze"
    );
}

fn render_world(resolve: &Resolve, world_id: WorldId) -> String {
    let world: &World = &resolve.worlds[world_id];
    let mut out = String::new();
    writeln!(out, "world {} {{", world.name).unwrap();
    writeln!(out, "  imports:").unwrap();
    render_world_items(resolve, &mut out, world.imports.iter());
    writeln!(out, "  exports:").unwrap();
    render_world_items(resolve, &mut out, world.exports.iter());
    writeln!(out, "}}").unwrap();
    out
}

fn render_world_items<'a>(
    resolve: &Resolve,
    out: &mut String,
    items: impl IntoIterator<Item = (&'a wit_parser::WorldKey, &'a WorldItem)>,
) {
    // Sort by world-key name so re-ordering of imports/exports in WIT
    // sources (or within `Resolve`'s arena) does not produce diffs.
    let mut entries: Vec<(String, &WorldItem)> = items
        .into_iter()
        .map(|(k, v)| (resolve.name_world_key(k), v))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (key_name, item) in entries {
        match item {
            WorldItem::Interface { id, .. } => {
                writeln!(out, "    interface {key_name} {{").unwrap();
                render_interface(resolve, out, &resolve.interfaces[*id]);
                writeln!(out, "    }}").unwrap();
            }
            WorldItem::Function(func) => {
                writeln!(out, "    {}", render_function(resolve, func)).unwrap();
            }
            WorldItem::Type(type_id) => {
                let ty_def = &resolve.types[*type_id];
                let display = ty_def
                    .name
                    .clone()
                    .unwrap_or_else(|| structural_typedef(resolve, *type_id));
                writeln!(out, "    type {key_name} = {display}").unwrap();
            }
        }
    }
}

fn render_interface(resolve: &Resolve, out: &mut String, iface: &Interface) {
    // Sort interface contents (types + functions) by name for stability.
    let mut type_names: Vec<&str> = iface.types.keys().map(String::as_str).collect();
    type_names.sort_unstable();
    for name in type_names {
        let id = iface.types[name];
        let def = type_definition(resolve, id);
        writeln!(out, "      type {name} = {def}").unwrap();
    }

    let mut func_names: Vec<&str> = iface.functions.keys().map(String::as_str).collect();
    func_names.sort_unstable();
    for name in func_names {
        let func = &iface.functions[name];
        writeln!(out, "      {}", render_function(resolve, func)).unwrap();
    }
}

fn render_function(resolve: &Resolve, func: &Function) -> String {
    let kind = match func.kind {
        wit_parser::FunctionKind::Freestanding => "func",
        wit_parser::FunctionKind::AsyncFreestanding => "async func",
        wit_parser::FunctionKind::Method(_) => "method",
        wit_parser::FunctionKind::AsyncMethod(_) => "async method",
        wit_parser::FunctionKind::Static(_) => "static",
        wit_parser::FunctionKind::AsyncStatic(_) => "async static",
        wit_parser::FunctionKind::Constructor(_) => "constructor",
    };
    let params: Vec<String> = func
        .params
        .iter()
        .map(|(n, t)| format!("{n}: {}", type_name(resolve, t)))
        .collect();
    let result = match &func.result {
        Some(t) => format!(" -> {}", type_name(resolve, t)),
        None => String::new(),
    };
    format!("{} {}({}){}", kind, func.name, params.join(", "), result)
}

/// Display name of a `Type`. Named typedefs use the declared name; anonymous
/// typedefs (lists, tuples, options, results, etc.) expand structurally.
fn type_name(resolve: &Resolve, ty: &Type) -> String {
    match ty {
        Type::Bool => "bool".into(),
        Type::U8 => "u8".into(),
        Type::U16 => "u16".into(),
        Type::U32 => "u32".into(),
        Type::U64 => "u64".into(),
        Type::S8 => "s8".into(),
        Type::S16 => "s16".into(),
        Type::S32 => "s32".into(),
        Type::S64 => "s64".into(),
        Type::F32 => "f32".into(),
        Type::F64 => "f64".into(),
        Type::Char => "char".into(),
        Type::String => "string".into(),
        Type::ErrorContext => "error-context".into(),
        Type::Id(id) => typedef_name(resolve, *id),
    }
}

/// Render the *reference* form of a typedef — its declared name when
/// available, structural form otherwise.
fn typedef_name(resolve: &Resolve, id: TypeId) -> String {
    let def = &resolve.types[id];
    if let Some(name) = &def.name {
        return name.clone();
    }
    structural_typedef(resolve, id)
}

/// Structural rendering of an anonymous typedef. Recursively expands
/// inner types via `type_name` (which falls back to declared names when
/// available, keeping output bounded).
fn structural_typedef(resolve: &Resolve, id: TypeId) -> String {
    let def = &resolve.types[id];
    match &def.kind {
        TypeDefKind::List(t) => format!("list<{}>", type_name(resolve, t)),
        TypeDefKind::FixedSizeList(t, n) => format!("list<{}, {n}>", type_name(resolve, t)),
        TypeDefKind::Option(t) => format!("option<{}>", type_name(resolve, t)),
        TypeDefKind::Result(r) => match (&r.ok, &r.err) {
            (Some(o), Some(e)) => {
                format!(
                    "result<{}, {}>",
                    type_name(resolve, o),
                    type_name(resolve, e)
                )
            }
            (Some(o), None) => format!("result<{}>", type_name(resolve, o)),
            (None, Some(e)) => format!("result<_, {}>", type_name(resolve, e)),
            (None, None) => "result".into(),
        },
        TypeDefKind::Tuple(Tuple { types }) => {
            let parts: Vec<String> = types.iter().map(|t| type_name(resolve, t)).collect();
            format!("tuple<{}>", parts.join(", "))
        }
        TypeDefKind::Future(Some(t)) => format!("future<{}>", type_name(resolve, t)),
        TypeDefKind::Future(None) => "future".into(),
        TypeDefKind::Stream(Some(t)) => format!("stream<{}>", type_name(resolve, t)),
        TypeDefKind::Stream(None) => "stream".into(),
        TypeDefKind::Handle(Handle::Own(inner)) => {
            format!("own<{}>", typedef_name(resolve, *inner))
        }
        TypeDefKind::Handle(Handle::Borrow(inner)) => {
            format!("borrow<{}>", typedef_name(resolve, *inner))
        }
        TypeDefKind::Type(t) => type_name(resolve, t),
        // For anonymous record/variant/enum/flags/resource we expand the
        // full definition so the snapshot is lossless.
        TypeDefKind::Record(_)
        | TypeDefKind::Variant(_)
        | TypeDefKind::Enum(_)
        | TypeDefKind::Flags(_)
        | TypeDefKind::Resource
        | TypeDefKind::Unknown => type_definition(resolve, id),
    }
}

/// Full definition of a typedef — the right-hand side of a `type` decl.
/// Used for interface-level type declarations where we want the full
/// shape, not just the reference name.
fn type_definition(resolve: &Resolve, id: TypeId) -> String {
    let def = &resolve.types[id];
    match &def.kind {
        TypeDefKind::Record(r) => {
            let fields: Vec<String> = r
                .fields
                .iter()
                .map(|f| format!("{}: {}", f.name, type_name(resolve, &f.ty)))
                .collect();
            format!("record {{ {} }}", fields.join(", "))
        }
        TypeDefKind::Variant(v) => {
            let cases: Vec<String> = v
                .cases
                .iter()
                .map(|c| match &c.ty {
                    Some(t) => format!("{}({})", c.name, type_name(resolve, t)),
                    None => c.name.clone(),
                })
                .collect();
            format!("variant {{ {} }}", cases.join(", "))
        }
        TypeDefKind::Enum(e) => {
            let cases: Vec<String> = e.cases.iter().map(|c| c.name.clone()).collect();
            format!("enum {{ {} }}", cases.join(", "))
        }
        TypeDefKind::Flags(f) => {
            let names: Vec<String> = f.flags.iter().map(|fl| fl.name.clone()).collect();
            format!("flags {{ {} }}", names.join(", "))
        }
        TypeDefKind::Resource => "resource".into(),
        TypeDefKind::Unknown => "unknown".into(),
        // Aliases/structural — defer to structural form.
        _ => structural_typedef(resolve, id),
    }
}
