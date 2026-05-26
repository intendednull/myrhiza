//! `BundleAddress`: locator for an installable bundle.
//!
//! Two variants per B-10 spec §3.5:
//!
//! - `Disk` — bundle is already materialized in a local directory.
//!   Used by tests, the CLI dev harness, and (post-fetch) the
//!   production path after `BundleDistribution::fetch` extracts an
//!   iroh-blob bundle into a tempdir.
//! - `IrohBlob` — production fetch path. Carries the iroh-blobs
//!   hash of the canonical-bincode-encoded manifest. The embedder
//!   must materialize this into a `Disk` variant via
//!   `BundleDistribution::fetch` (in `crates/distribution`) before
//!   calling `myrhiza_kernel::install::load`.
//!
//! Lives in `crates/types/` (a leaf crate) so both `crates/kernel/`
//! (which consumes `BundleAddress` in `install::load`) and
//! `crates/distribution/` (which constructs `BundleAddress::Disk` as
//! the output of `BundleDistribution::fetch`) can reach it without
//! inducing a circular dep. The dep direction is
//! `kernel -> distribution -> types <- kernel` (a diamond, not a cycle).
//!
//! Per B-10 spec §4.6 declared dep direction.
//!
//! `myrhiza_kernel::BundleAddress` is preserved as a backwards-compat
//! re-export of `myrhiza_types::BundleAddress`.

use std::path::PathBuf;

use crate::BlobHash;

/// Locator for an installable bundle.
///
/// See module-level docs for the two-variant rationale.
#[derive(Debug, Clone)]
pub enum BundleAddress {
    /// On-disk bundle. The `bundle_dir` contains `manifest.bincode`
    /// and the `components/` artifact tree; `manifest_path` is the
    /// path of the manifest file (canonical-bincode-encoded) relative
    /// to `bundle_dir`. v1 file naming is `manifest.bincode`. The TOML
    /// human-readable form is canonicalized at publish time; the
    /// kernel only consumes the canonical bytes.
    Disk {
        /// Root of the bundle directory.
        bundle_dir: PathBuf,
        /// Manifest path relative to `bundle_dir`.
        manifest_path: PathBuf,
    },
    /// Iroh-blob bundle identified by the BLAKE3 hash of its
    /// canonical-bincode-encoded manifest. The kernel does not fetch
    /// directly — the embedder calls `BundleDistribution::fetch`
    /// (which lives in `crates/distribution`) to materialize the blob
    /// tree into a tempdir and produce a `Disk` variant that
    /// `myrhiza_kernel::install::load` consumes.
    IrohBlob {
        /// Manifest hash — the identifier the author shares out of band.
        manifest_hash: BlobHash,
    },
}
