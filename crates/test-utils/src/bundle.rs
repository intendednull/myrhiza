//! Bundle directory builders for tests.

use std::path::PathBuf;

use bincode::Options;
use myrhiza_manifest::schema::Manifest;
use myrhiza_types::{EventHash, canonical_bincode};
use tempfile::TempDir;

use crate::manifest::{
    deterministic_signing_key, helpers_only_state_apply_manifest_with_extra_cap, sign_manifest,
};

/// A built test bundle: tempdir + manifest path + content bytes.
///
/// `_dir` is `pub` so consumers can borrow it for path manipulations
/// while still naming the field `_dir` to flag "this exists for RAII;
/// reads are unusual." Hence the targeted clippy allow.
#[must_use]
#[allow(clippy::pub_underscore_fields)]
pub struct TestBundle {
    /// Owning tempdir handle. Kept alive for the test's duration.
    pub _dir: TempDir,
    /// Absolute path to the bundle root.
    pub bundle_dir: PathBuf,
    /// Path to the canonical-bincode manifest, relative to `bundle_dir`.
    pub manifest_path: PathBuf,
    /// The component bytes that were written into the bundle.
    pub content_bytes: Vec<u8>,
}

/// Write a signed bundle into a fresh tempdir.
///
/// `m` must already be signed via [`crate::manifest::sign_manifest`]
/// against the `content_hash` of `component_bytes`.
///
/// # Errors
/// Returns any underlying [`std::io::Error`] from the tempdir creation,
/// directory creation, or file write calls.
///
/// # Panics
/// Panics if the manifest declares a `state_apply` component path that
/// has no parent directory. The manifest builder normalizes this to
/// `components/state-apply.wasm`, so the panic is structurally
/// unreachable for fixtures from this crate.
#[allow(clippy::expect_used)]
pub fn write_bundle(m: &Manifest, component_bytes: &[u8]) -> std::io::Result<TestBundle> {
    let dir = TempDir::new()?;
    let bundle_dir = dir.path().to_path_buf();

    let comp_rel = m
        .components
        .state_apply
        .clone()
        .unwrap_or_else(|| "components/state-apply.wasm".into());
    let comp_path = bundle_dir.join(&comp_rel);
    let parent = comp_path.parent().expect("component path has a parent dir");
    std::fs::create_dir_all(parent)?;
    std::fs::write(&comp_path, component_bytes)?;

    let manifest_rel = PathBuf::from("manifest.bincode");
    let manifest_bytes = canonical_bincode()
        .serialize(m)
        .expect("canonical bincode of Manifest never fails");
    std::fs::write(bundle_dir.join(&manifest_rel), manifest_bytes)?;

    Ok(TestBundle {
        _dir: dir,
        bundle_dir,
        manifest_path: manifest_rel,
        content_bytes: component_bytes.to_vec(),
    })
}

/// Build a signed [`TestBundle`] around `component_bytes` whose manifest
/// is the helpers-only state-apply manifest *augmented* with one extra
/// entry under `capabilities.host_imports` set to `true`.
///
/// Used to drive the `mvp.md §15.1 #5` manifest-arm acceptance test.
/// The manifest is signed with [`deterministic_signing_key`] keyed on
/// `seed` so the install flow accepts the bundle (signature verifies);
/// the rejection then surfaces from the backend's
/// `validate_state_apply_manifest` step rather than from the install
/// loader. The component itself does not need to import `extra_cap` —
/// the manifest gating fires regardless.
///
/// # Errors
/// Returns any underlying [`std::io::Error`] from the tempdir creation,
/// directory creation, or file write calls.
///
/// # Panics
/// Panics under the same conditions as [`write_bundle`].
#[allow(clippy::expect_used)]
pub fn build_counter_bundle_with_extra_cap(
    component_bytes: &[u8],
    extra_cap: &str,
    seed: u8,
) -> std::io::Result<TestBundle> {
    let content_hash = EventHash::blake3(component_bytes);
    let mut manifest = helpers_only_state_apply_manifest_with_extra_cap(extra_cap);
    let key = deterministic_signing_key(seed);
    sign_manifest(&mut manifest, &content_hash, &key);
    write_bundle(&manifest, component_bytes)
}
