//! Bit-identity check between `crates/sdk/wit/` and the canonical
//! `wit/myrhiza-kernel/wit/`. Prevents WIT drift between the SDK's
//! distributed copy and the source of truth.
//!
//! Per docs/specs/2026-05-26-b-8-sdk-design.md §2.5.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn sdk_wit_matches_kernel_wit() {
    let sdk_wit = sdk_wit_dir();
    let kernel_wit = kernel_wit_dir();

    let sdk_files = list_wit_files(&sdk_wit);
    let kernel_files = list_wit_files(&kernel_wit);

    let only_in_sdk: BTreeSet<_> = sdk_files.difference(&kernel_files).cloned().collect();
    let only_in_kernel: BTreeSet<_> = kernel_files.difference(&sdk_files).cloned().collect();

    assert!(
        only_in_sdk.is_empty(),
        "WIT drift: files present in crates/sdk/wit/ but missing from wit/myrhiza-kernel/wit/: {only_in_sdk:?}. Run `just sync-wit`."
    );
    assert!(
        only_in_kernel.is_empty(),
        "WIT drift: files present in wit/myrhiza-kernel/wit/ but missing from crates/sdk/wit/: {only_in_kernel:?}. Run `just sync-wit`."
    );

    // Bit-identity check on every file.
    for name in &sdk_files {
        let sdk_bytes = std::fs::read(sdk_wit.join(name)).expect("read sdk wit file");
        let kernel_bytes = std::fs::read(kernel_wit.join(name)).expect("read kernel wit file");
        assert_eq!(
            sdk_bytes,
            kernel_bytes,
            "WIT drift: {} differs between crates/sdk/wit/ and wit/myrhiza-kernel/wit/. Run `just sync-wit`.",
            name.display()
        );
    }
}

fn sdk_wit_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wit")
}

fn kernel_wit_dir() -> PathBuf {
    // ancestors().nth(2) → workspace root:
    //   nth(0) is crates/sdk, nth(1) is crates, nth(2) is workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root is two levels above crates/sdk")
        .join("wit/myrhiza-kernel/wit")
}

fn list_wit_files(dir: &std::path::Path) -> BTreeSet<PathBuf> {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("wit") {
                Some(PathBuf::from(path.file_name()?))
            } else {
                None
            }
        })
        .collect()
}
