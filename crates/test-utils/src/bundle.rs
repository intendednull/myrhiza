//! Bundle directory builders for tests.

use std::path::PathBuf;

use bincode::Options;
use myrhiza_kernel::BundleAddress;
use myrhiza_manifest::schema::Manifest;
use myrhiza_types::canonical_bincode;
use tempfile::TempDir;

use crate::manifest::{
    deterministic_signing_key, helpers_only_state_apply_manifest,
    helpers_only_state_apply_manifest_with_extra_cap, helpers_only_three_component_manifest,
    sign_manifest, sign_manifest_three_components,
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

/// Resolve a built-fixture path by name. Test-utils sits at
/// `crates/test-utils/`, so walking up two ancestors reaches the
/// workspace root; the fixture name (without `.wasm`) is joined under
/// `tests/fixtures/built/`. Consolidates per-fixture path helpers so
/// each fixture only contributes a `build_signed_*` function.
#[allow(clippy::expect_used)]
fn fixture_path(name: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(2)
        .expect("workspace root is two levels above test-utils crate manifest")
        .join("tests/fixtures/built")
        .join(format!("{name}.wasm"))
}

/// Read a built fixture by name, panicking with a `just build-fixtures`
/// hint if the file is missing. Consolidates the per-`build_signed_*`
/// fixture-reading boilerplate.
#[allow(clippy::panic)]
fn read_fixture(name: &str) -> Vec<u8> {
    let path = fixture_path(name);
    std::fs::read(&path).unwrap_or_else(|e| {
        panic!(
            "{name} fixture missing at {}: {e} — run `just build-fixtures`",
            path.display()
        )
    })
}

/// Build a signed counter-state-apply bundle from the
/// reproducibly-built fixture at `tests/fixtures/built/counter-state-apply.wasm`.
/// Returns the [`TestBundle`] (with on-disk artifacts retained via the
/// inner [`TempDir`]) and its [`BundleAddress`] (suitable for
/// [`myrhiza_kernel::InstallFlow::load`]).
///
/// Requires `just build-fixtures` to have produced the wasm artifact.
/// Used by both plan-A acceptance tests and plan-B-1 convergence tests.
///
/// # Panics
/// Panics if the fixture wasm is missing or unreadable, or if the
/// tempdir bundle write fails. Both indicate a broken test environment
/// (forgot `just build-fixtures`, /tmp unwriteable) rather than a
/// runtime condition the test should recover from. The matching
/// `#[allow]` is the documented escape hatch per workspace
/// `Cargo.toml` — `test-utils` is dev-only (`publish = false`).
#[allow(clippy::expect_used, clippy::panic)]
pub fn build_signed_counter_bundle() -> (TestBundle, BundleAddress) {
    let component_bytes = read_fixture("counter-state-apply");

    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(7);
    sign_manifest(&mut manifest, &component_bytes, &key);

    let test_bundle = write_bundle(&manifest, &component_bytes).expect("write bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
}

/// Build a signed echo-state-apply bundle from the
/// reproducibly-built fixture at `tests/fixtures/built/echo-state-apply.wasm`.
/// Returns the [`TestBundle`] (with on-disk artifacts retained via the
/// inner [`TempDir`]) and its [`BundleAddress`] (suitable for
/// [`myrhiza_kernel::InstallFlow::load`]).
///
/// Requires `just build-fixtures` to have produced the wasm artifact.
/// Used by plan-B-5 coexistence tests (two distinct WASM state-apply
/// components running on the same network).
///
/// # Panics
/// Panics if the fixture wasm is missing or unreadable, or if the
/// tempdir bundle write fails. Both indicate a broken test environment
/// (forgot `just build-fixtures`, /tmp unwriteable) rather than a
/// runtime condition the test should recover from.
#[allow(clippy::expect_used, clippy::panic)]
pub fn build_signed_echo_bundle() -> (TestBundle, BundleAddress) {
    let component_bytes = read_fixture("echo-state-apply");

    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(11);
    sign_manifest(&mut manifest, &component_bytes, &key);

    let test_bundle = write_bundle(&manifest, &component_bytes).expect("write bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
}

/// Write a three-component bundle (state-apply + state-propose + interaction)
/// into a fresh tempdir. Signs with the composite `bundle_content_hash` covering
/// all three component slots.
///
/// Mirrors [`write_bundle`] but handles the three-path manifest layout.
///
/// # Errors
/// Returns any underlying [`std::io::Error`].
///
/// # Panics
/// Panics if any component path in the manifest has no parent directory.
#[allow(clippy::expect_used)]
pub fn write_three_component_bundle(
    m: &Manifest,
    apply_bytes: &[u8],
    propose_bytes: &[u8],
    interaction_bytes: &[u8],
) -> std::io::Result<TestBundle> {
    let dir = TempDir::new()?;
    let bundle_dir = dir.path().to_path_buf();

    // Write state-apply
    let apply_rel = m
        .components
        .state_apply
        .clone()
        .unwrap_or_else(|| "components/state-apply.wasm".into());
    let apply_path = bundle_dir.join(&apply_rel);
    let parent = apply_path
        .parent()
        .expect("state-apply component path has a parent dir");
    std::fs::create_dir_all(parent)?;
    std::fs::write(&apply_path, apply_bytes)?;

    // Write state-propose (same parent dir — components/)
    let propose_rel = m
        .components
        .state_propose
        .clone()
        .unwrap_or_else(|| "components/state-propose.wasm".into());
    std::fs::write(bundle_dir.join(&propose_rel), propose_bytes)?;

    // Write interaction
    let ix_rel = m
        .components
        .interaction
        .clone()
        .unwrap_or_else(|| "components/interaction.wasm".into());
    std::fs::write(bundle_dir.join(&ix_rel), interaction_bytes)?;

    let manifest_rel = PathBuf::from("manifest.bincode");
    let manifest_bytes = canonical_bincode()
        .serialize(m)
        .expect("canonical bincode of Manifest never fails");
    std::fs::write(bundle_dir.join(&manifest_rel), manifest_bytes)?;

    Ok(TestBundle {
        _dir: dir,
        bundle_dir,
        manifest_path: manifest_rel,
        content_bytes: apply_bytes.to_vec(),
    })
}

/// Build a signed three-component counter bundle from the reproducibly-built
/// fixtures at `tests/fixtures/built/`.
///
/// Signs all three components (state-apply, state-propose, interaction) under
/// the composite `bundle_content_hash` with [`deterministic_signing_key`](7).
/// Returns the [`TestBundle`] and its [`BundleAddress`].
///
/// Requires `just build-fixtures` to have produced all three artifacts.
///
/// # Panics
/// Panics if any fixture is missing or unreadable, or if the tempdir write
/// fails. Both indicate a broken test environment.
#[allow(clippy::expect_used, clippy::panic)]
pub fn build_signed_counter_bundle_three_components() -> (TestBundle, BundleAddress) {
    let apply_bytes = read_fixture("counter-state-apply");
    let propose_bytes = read_fixture("counter-state-propose");
    let interaction_bytes = read_fixture("counter-interaction");

    let mut manifest = helpers_only_three_component_manifest();
    let key = deterministic_signing_key(7);
    sign_manifest_three_components(
        &mut manifest,
        &apply_bytes,
        &propose_bytes,
        &interaction_bytes,
        &key,
    );

    let test_bundle =
        write_three_component_bundle(&manifest, &apply_bytes, &propose_bytes, &interaction_bytes)
            .expect("write three-component bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
}

/// Build a signed poll-state-apply bundle from the
/// reproducibly-built fixture at `tests/fixtures/built/poll-state-apply.wasm`.
/// Returns the [`TestBundle`] (with on-disk artifacts retained via the
/// inner [`TempDir`]) and its [`BundleAddress`] (suitable for
/// [`myrhiza_kernel::InstallFlow::load`]).
///
/// Requires `just build-fixtures` to have produced the wasm artifact.
/// Used by plan-B-6 poll-app state-tier + kernel-tier tests.
///
/// Signs with [`deterministic_signing_key`](13) — distinct from the
/// counter helpers' seed `7` so the on-disk signing artifact does not
/// collide when both bundles co-exist in a single test (per plan B-6).
///
/// # Panics
/// Panics if the fixture wasm is missing or unreadable, or if the
/// tempdir bundle write fails. Both indicate a broken test environment
/// (forgot `just build-fixtures`, /tmp unwriteable) rather than a
/// runtime condition the test should recover from. The matching
/// `#[allow]` is the documented escape hatch per workspace
/// `Cargo.toml` — `test-utils` is dev-only (`publish = false`).
#[allow(clippy::expect_used, clippy::panic)]
pub fn build_signed_poll_bundle() -> (TestBundle, BundleAddress) {
    let component_bytes = read_fixture("poll-state-apply");

    let mut manifest = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(13);
    sign_manifest(&mut manifest, &component_bytes, &key);

    let test_bundle = write_bundle(&manifest, &component_bytes).expect("write bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
}

/// Build a signed three-component poll bundle from the reproducibly-built
/// fixtures at `tests/fixtures/built/`.
///
/// Signs all three components (state-apply, state-propose, interaction) under
/// the composite `bundle_content_hash` with [`deterministic_signing_key`](13).
/// Returns the [`TestBundle`] and its [`BundleAddress`].
///
/// Requires `just build-fixtures` to have produced all three artifacts.
///
/// # Panics
/// Panics if any fixture is missing or unreadable, or if the tempdir write
/// fails. Both indicate a broken test environment.
#[allow(clippy::expect_used, clippy::panic)]
pub fn build_signed_poll_bundle_three_components() -> (TestBundle, BundleAddress) {
    let apply_bytes = read_fixture("poll-state-apply");
    let propose_bytes = read_fixture("poll-state-propose");
    let interaction_bytes = read_fixture("poll-interaction");

    let mut manifest = helpers_only_three_component_manifest();
    let key = deterministic_signing_key(13);
    sign_manifest_three_components(
        &mut manifest,
        &apply_bytes,
        &propose_bytes,
        &interaction_bytes,
        &key,
    );

    let test_bundle =
        write_three_component_bundle(&manifest, &apply_bytes, &propose_bytes, &interaction_bytes)
            .expect("write three-component bundle to tempdir");
    let addr = BundleAddress {
        bundle_dir: test_bundle.bundle_dir.clone(),
        manifest_path: test_bundle.manifest_path.clone(),
    };
    (test_bundle, addr)
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
    let mut manifest = helpers_only_state_apply_manifest_with_extra_cap(extra_cap);
    let key = deterministic_signing_key(seed);
    sign_manifest(&mut manifest, component_bytes, &key);
    write_bundle(&manifest, component_bytes)
}
