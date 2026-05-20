**Date:** 2026-05-19
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-2 — Persistent identity (bech32m PeerKeypair + AuthorKeypair) and B-1 carryover cleanup

# Plan B-2 design — Persistent identity + B-1 carryover cleanup

## 1. Goal

Replace B-1's in-memory `PeerKeypair` / `AuthorKeypair` stubs (`crates/kernel/src/identity.rs`) with a persistent identity layer:

- Secret keys persisted as **raw 32-byte binary files** (per Willow's "no `wsecret` HRP, ever" commitment — see §4.1).
- Author public keys encoded as bech32m (BIP-350) when they appear in filenames or other paste-buffer-adjacent surfaces.
- An `IdentityStore` trait so production code uses filesystem storage and tests keep using deterministic seeds.
- A `FilesystemIdentityStore` that loads-or-creates and enforces conservative file permissions (0600 + loose-perm refusal).
- `ZeroizeOnDrop` on the keypair structs.
- The same `Runtime::start(...)` signature — the runtime layer stays filesystem-unaware so non-native embeddings (jco backend, B-4 iroh stress harnesses) compose cleanly.

Plus one trivial B-1 review carryover that is a pure-refactor readability fix:

- **N-12 — `handle_heads_summary` function split.** Currently a single `#[allow(clippy::too_many_lines)]` function covering four diff cases (behind / equal / ahead / local). Pure refactor into four sub-fns; no semantic change.

This slice lands **none** of:

- **Q-4 — pending-peer attribution.** Originally planned for B-2 but deferred to **B-4** during spec audit. B-1's `GossipMessage` envelope has no sender identity field, and `Subscription::recv` does not expose the sending peer — so populating `peer_id` on pending entries would require a protocol change. In B-4, iroh's per-connection NodeID authentication naturally provides the sending peer; Q-4 plumbs that through. Bundling the protocol change with persistence is unnecessary risk.
- Q-1 / Q-7 (replay_full O(N) and anchor-digest off-loop) — deferred to **B-2.1** (perf slice). Performance changes to the runtime select loop are orthogonal to persistence and have subtle correctness implications around drop tracking + state-apply purity; bundling them with persistence mixes risk surfaces unnecessarily.
- iroh transport (B-4), module-dep recursion (B-3), revocation topic (B-5), host-call fuel wiring (B-6), persistent DAG (B-7).
- A canonical OS-standard storage path (XDG / `%APPDATA%`). B-2 takes the directory as a parameter; B-7 owns the canonical peer-state-dir layout that subsumes both keys and the persisted DAG.
- Multi-author-per-Runtime semantics — the store API supports listing authors by pubkey for future use, but `Runtime::start` still takes one `Option<AuthorKeypair>`. Multi-identity is deferred per [identity.md](2026-05-09-myrhiza-master-design/identity.md) §6.3.

## 2. Scope decisions (locked during brainstorming + prior-art consultation, 2026-05-19)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **Secret on-disk format** | **Raw 32 bytes (binary)** | bech32m text | `prior-art/willow/identity.md` "no `wsecret` HRP, ever" commitment — secrets in bech32m caused the Nostr nsec/npub visual-similarity disaster. Secrets should never enter paste buffers, log lines, or git diffs. Binary file is a clear discouragement against `cat`-style inspection. |
| **Public-key encoding (filenames)** | bech32m with `wuser` HRP | `mz-author-pk` (initial draft) / `wpub-author` (master-spec publisher HRP) | Aligns with master spec's `w*` HRP convention (`distribution.md §10.2` uses `wpub-author` for *publishers*, `wpub-myrhiza` for official root). `wuser` is a new HRP for *event-author identity* (the IdentityScope.long-term in identity.md §6) — single-token kebab-free style, matching willow's `wpeer/wserver`. The publisher-vs-user-author distinction is preserved: `wpub-*` for app/module signing identities (distribution.md), `wuser` for per-topic event-authoring identities (this spec). |
| Storage layout | `<dir>/peer.key` + `<dir>/authors/<wuser1...>.key`; dir is caller-provided | XDG default baked in | Premature until B-7 defines full peer-state-dir; embedder (CLI, browser, tests) picks; no XDG semantics forced on tests |
| Load layer | New `IdentityStore` trait + `FilesystemIdentityStore` impl | Bake load into `Runtime::start` | Keeps runtime layer pure; jco backend has no filesystem; tests keep `deterministic()`; future stores (HSM, OS keyring, encrypted-at-rest) drop in via same trait |
| Missing key | Generate via `OsRng` + persist on first load | Fail loudly if missing | Generate-on-first-run is expected single-machine UX (mirrors Willow `Identity::load_or_generate`); explicit `open_existing_only` mode is a future extension if anyone needs it |
| Permissions | 0600 on Unix; reject load if looser; Windows best-effort + warning | No enforcement | Kernel custody of secrets implies refusing world-readable files. Direct lift of Willow's `load_or_generate` (`crates/identity/src/lib.rs:196-237`, issue #126 regression tests). Windows ACL story deferred. |
| **Secret zeroization** | `ZeroizeOnDrop` on `PeerKeypair` / `AuthorKeypair` | No explicit zeroization | Willow pattern (`Identity` is `ZeroizeOnDrop + Send + Sync`). Reduces residual-memory risk after key drop. Trivial cost (32-byte memset). |
| Multi-author | Store API lists by `AuthorPubkey`; `Runtime` still takes one author | Multi-author Runtime now | Multi-identity is deferred per identity.md §6.3; don't widen Runtime surface preemptively |
| Test seeds | Retain `PeerKeypair::deterministic(seed)` and `AuthorKeypair::deterministic(seed)` | Replace all sites with filesystem fixtures | 30+ test sites; persistence is a separate concern from event/DAG/runtime tests — persistence has its own dedicated tests |
| Bech32m crate | `bech32 = "=0.11.0"` (BIP-350) | Hand-roll | YAGNI to reimplement checksummed encoding; canonical Rust crate, MIT, widely audited; willow uses it too |
| **Encrypted-at-rest** | **Deferred** — flagged as `IdentityStore` extension point | Ship with B-2 | `prior-art/iroh/identity.md` names this as a "clean win" Myrhiza can offer over iroh, but a passphrase prompt is significant UX surface. Trait shape accepts an `EncryptedFilesystemIdentityStore` future impl without breaking changes. |
| Runtime API change | None — `Runtime::start` signature unchanged | Take store instead of keypairs | Caller drives load → start; runtime stays filesystem-unaware |
| Q-1, Q-7 | Defer to B-2.1 | Bundle into B-2 | Perf optimizations to runtime select loop; orthogonal to persistence; landing them together makes the PR review surface unmanageable |
| Q-4 | **Defer to B-4** | In scope for B-2 | Originally drafted as in-scope. Spec audit revealed B-1's `GossipMessage` carries no sender identity and `Subscription::recv` does not expose the sending peer — implementing Q-4 requires a protocol-level change. iroh transport (B-4) provides per-connection NodeID-authenticated sender identity natively; defer Q-4 there. |
| N-12 | In scope for B-2 | Defer to B-2.1 | Pure refactor (no semantic change); ships cleanly alongside persistence. |

## 3. Crate + module layout

`crates/kernel/src/identity.rs` is promoted to a module directory:

```
crates/kernel/src/identity/
├── mod.rs       — re-exports PeerKeypair, AuthorKeypair, IdentityStore,
│                  FilesystemIdentityStore, IdentityError + module
│                  docs. The keypair structs themselves move here from
│                  the current single-file module. ZeroizeOnDrop is
│                  added per Willow precedent.
├── store.rs     — IdentityStore trait + IdentityError enum
└── fs.rs        — FilesystemIdentityStore impl + bech32m filename
                  helpers (encode_author_pubkey, decode_author_pubkey,
                  perm-mode check)
```

Keypair struct changes:

```rust
// crates/kernel/src/identity/mod.rs (was identity.rs)

use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
pub struct PeerKeypair {
    secret: ed25519_dalek::SigningKey,
    #[zeroize(skip)]
    pub public: PeerPubkey,
}

#[derive(ZeroizeOnDrop)]
pub struct AuthorKeypair {
    secret: ed25519_dalek::SigningKey,
    #[zeroize(skip)]
    pub author: AuthorPubkey,
}
```

`SigningKey` from `ed25519-dalek` 2.x already implements `Zeroize`; deriving `ZeroizeOnDrop` on the wrapper gives us drop-time secret clearing. Public fields skip zeroize because they're not secret. Direct lift from Willow's `ZeroizeOnDrop + Send + Sync` discipline on `willow_identity::Identity`.

No changes to existing public re-export sites: `crates/kernel/src/lib.rs` continues to expose `pub use identity::{AuthorKeypair, PeerKeypair};` and adds `pub use identity::{IdentityStore, FilesystemIdentityStore, IdentityError};`. Downstream test-utils and downstream tests do not need to change their imports.

Kernel modifications outside `identity/`:

```
crates/kernel/src/runtime.rs   — handle_heads_summary split into
                                  four sub-fns (N-12); no other
                                  changes
crates/kernel/src/lib.rs       — re-export new identity types
```

`crates/kernel/src/pending.rs` is **unchanged** — Q-4's pending-entry `peer_id` plumbing is deferred to B-4 (see §1).

Cargo deps:

```toml
# crates/kernel/Cargo.toml
[dependencies]
bech32 = "=0.11.0"            # BIP-350 bech32m (filenames only)
zeroize = { version = "1.7", features = ["derive"] }
async-trait = { workspace = true }   # already in workspace; restated here for clarity

[dev-dependencies]
tempfile = "3"
```

Workspace `[workspace.dependencies]`:

```toml
bech32 = "=0.11.0"
zeroize = { version = "1.7", features = ["derive"] }
tempfile = "3"
# async-trait already present in workspace.dependencies (from B-1)
```

The exact pin on `bech32` matches the project convention from plan A (`bincode = "=1.3.3"`, `wasmtime = "=36.0.9"`); checksum-format crates are stability-sensitive enough to pin tightly. `zeroize` uses caret range — the `derive` macro is stable.

## 4. Encoding

### 4.1 Secret on-disk format — raw 32 bytes (binary)

**Direct lift from `prior-art/willow/identity.md` "No `wsecret` HRP, ever":**

> The spec makes one explicit security commitment: **`wsecret` will never exist**. Private keys do not get a bech32 form. The `nsec` ↔ `npub` visual-similarity disaster in the Nostr ecosystem is treated as a settled negative; secrets stay in the keystore … and never enter paste buffers.

Myrhiza inherits this. A persisted secret key is exactly 32 bytes of binary content. No HRP, no checksum, no text framing. The kernel reads `peer.key` with `std::fs::read`, asserts `len == 32`, and uses the bytes as the Ed25519 seed via `PeerKeypair::from_secret_bytes`. Same for `authors/<pk>.key`.

This file is unfit for `cat`, `git diff`, or copy-paste — by design. Inspection requires a deliberate `xxd` invocation, signalling "you are looking at a private key." That friction is the feature.

### 4.2 Filename-embedded public keys — bech32m

`AuthorPubkey` filenames embed the bech32m-encoded *public* key:

```
authors/wuser1<58 bech32m chars>.key
```

This makes `ls authors/` immediately readable, gives copy-paste users a checksummed identifier (BIP-350 BCH catches single-character typos in CLI flows), and aligns with the master spec's `w*` HRP convention (distribution.md §10.2).

### 4.3 HRP table

| HRP | Encodes | Where it appears | Defined in |
|---|---|---|---|
| `wuser` | 32-byte Ed25519 verifying key (event-author identity / IdentityScope.long-term) | `authors/<pk>.key` filename portion | **B-2 (this spec)** |
| `wpub-author` | Ed25519 verifying key of an app/module *publisher* | manifest fields per distribution.md | distribution.md §10.2 |
| `wpub-myrhiza` | Ed25519 verifying key of official myrhiza-* module signing root | manifest fields per distribution.md | distribution.md §10.2 |

**Publisher vs event-author distinction.** Both kinds are Ed25519 keypairs but they appear in different contexts: `wpub-*` HRPs mark publisher identities (developer signing an app bundle release), `wuser` marks the per-topic event-authoring identity. Same primitive (Ed25519 32-byte pubkey), different display HRP so the role is unambiguous on inspection.

Future HRPs are reserved for follow-up specs (peer-pubkey CLI exports, event-hash URL references, topic-identifier display); B-2 mints only `wuser` to avoid premature HRP allocation. A future "HRP vocabulary" spec should consolidate the table across `distribution.md`, B-2, and any other producers.

HRP charset uses kebab-case hyphens (allowed by BIP-350: HRP is ASCII 33–126 minus `1`).

### 4.4 Encoding helpers

```rust
// crates/kernel/src/identity/fs.rs

pub(super) fn encode_author_pubkey(pk: AuthorPubkey) -> String;
pub(super) fn decode_author_pubkey(s: &str) -> Result<AuthorPubkey, IdentityError>;
```

`decode_author_pubkey` enforces HRP match against `wuser` — any other HRP returns `IdentityError::HrpMismatch { expected: "wuser", actual }`. Used only for filename validation; secret files do not carry an HRP. `encode_author_pubkey` takes `AuthorPubkey` by value (it is `Copy`).

## 5. `IdentityStore` trait

```rust
// crates/kernel/src/identity/store.rs

use async_trait::async_trait;
use myrhiza_types::{AuthorPubkey, PeerPubkey};

/// A pluggable identity backend. Production = filesystem; tests may
/// use the in-memory keypair constructors directly without going
/// through a store.
#[async_trait]
pub trait IdentityStore: Send + Sync {
    /// Load the peer's keypair, generating + persisting a fresh one
    /// if no peer key exists in the store.
    async fn load_or_create_peer(&self) -> Result<PeerKeypair, IdentityError>;

    /// Load an author keypair by its public key.
    async fn load_author(&self, pk: &AuthorPubkey) -> Result<AuthorKeypair, IdentityError>;

    /// Generate + persist a fresh author keypair. Returns the new
    /// keypair (caller decides whether to hand it to Runtime::start).
    async fn create_author(&self) -> Result<AuthorKeypair, IdentityError>;

    /// List all author public keys in the store, sorted.
    async fn list_authors(&self) -> Result<Vec<AuthorPubkey>, IdentityError>;
}
```

The trait is small on purpose: B-2 only needs load/create. Author-key deletion, peer-key rotation, and audit-log surface are intentionally **out of scope** — they belong to future identity-management work (post-B, likely in a `myrhiza-identity-multi-device` module per identity.md §6.3).

## 6. `FilesystemIdentityStore`

### 6.1 Surface

```rust
// crates/kernel/src/identity/fs.rs

pub struct FilesystemIdentityStore {
    dir: PathBuf,
}

impl FilesystemIdentityStore {
    /// Open a filesystem-backed store rooted at `dir`. Creates the
    /// directory (and `authors/` subdirectory) if missing, with mode
    /// 0700 on Unix.
    pub async fn open(dir: impl Into<PathBuf>) -> Result<Self, IdentityError>;
}

#[async_trait]
impl IdentityStore for FilesystemIdentityStore { /* ... */ }
```

### 6.2 On-disk layout

```
<dir>/
├── peer.key                                 (mode 0600)
└── authors/                                 (mode 0700)
    ├── wuser1<...>.key                      (mode 0600)
    └── wuser1<...>.key                      (mode 0600)
```

### 6.3 Read path

1. `open(dir)`:
   - If `<dir>` does not exist, create it with mode 0700.
   - If `<dir>/authors` does not exist, create it with mode 0700.
   - On Unix, verify mode bits on `<dir>` ≤ 0700. If looser, return `IdentityError::InsecurePermissions { path, mode }`.

2. `load_or_create_peer()`:
   - If `<dir>/peer.key` does not exist:
     - Generate a fresh `PeerKeypair` via `PeerKeypair::generate(OsRng)`.
     - Write the **raw 32 secret bytes** to `<dir>/peer.key.tmp` with mode 0600, then atomic-rename to `peer.key`.
     - Return the keypair.
   - Else:
     - On Unix, verify mode bits ≤ 0600. If looser, return `InsecurePermissions`.
     - Read the file; assert `bytes.len() == 32`. If not, return `IdentityError::SeedLengthMismatch { path, actual }`.
     - Return `PeerKeypair::from_secret_bytes(bytes.try_into().unwrap())`.

3. `load_author(pk)`:
   - Read `<dir>/authors/<encode_author_pubkey(pk)>.key` with the same checks as `load_or_create_peer`.
   - Derive the verifying key from the seed and verify it matches the requested `pk`. If not, return `IdentityError::AuthorPubkeyMismatch { requested, actual }` — protects against tampered filenames.

4. `create_author()`:
   - Generate fresh via `AuthorKeypair::generate(OsRng)`.
   - Path: `<dir>/authors/<encode_author_pubkey(&author.author)>.key`.
   - Atomic write through `<path>.tmp` + rename, raw 32 bytes.
   - Return the keypair.

5. `list_authors()`:
   - Read `<dir>/authors/`; collect entries matching `wuser1*.key`; decode the filename's bech32m to recover `AuthorPubkey`; sort.

### 6.4 Atomic write detail

All writes follow the same idiom (`crates/kernel/src/identity/fs.rs::write_secret`):

```rust
// Use a sibling .tmp path explicitly. `with_extension` would replace
// the existing extension (e.g. peer.key → peer.tmp); we want
// peer.key.tmp instead.
let tmp = path.with_file_name({
    let mut s = path.file_name().unwrap().to_os_string();
    s.push(".tmp");
    s
});
let mut f = OpenOptions::new()
    .create_new(true)        // refuse if .tmp exists (concurrent write)
    .write(true)
    .mode(0o600)             // Unix only — cfg-gated
    .open(&tmp)?;
f.write_all(&seed_bytes)?;   // exactly 32 bytes
f.sync_all()?;               // crash-safety
drop(f);
std::fs::rename(&tmp, path)?;
```

`create_new` guards against concurrent stores writing the same key — if another process is mid-write, the second writer errors out cleanly instead of corrupting the file. Direct lift from Willow's `Identity::load_or_generate` (issue #126 regression tests at `willow/crates/identity/src/lib.rs:615-663`).

### 6.5 Async wrapping

Although filesystem IO is the moral equivalent of blocking, we expose the store as `async_trait` because the kernel runtime's startup path is already async (per B-1 spec §11). The implementation wraps the blocking IO in `tokio::task::spawn_blocking` so a slow disk does not block the runtime executor.

Trade-off acknowledged: `spawn_blocking` adds one task-spawn per load. Acceptable — identity load happens once per `Runtime::start`, not per event.

## 7. `IdentityError`

```rust
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("io error at {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },

    #[error("bech32 decode failed for {input}: {source}")]
    Bech32Decode { input: String, source: bech32::DecodeError },

    #[error("HRP mismatch in {input}: expected '{expected}', got '{actual}'")]
    HrpMismatch { input: String, expected: &'static str, actual: String },

    #[error("expected 32-byte seed at {path}, got {actual} bytes")]
    SeedLengthMismatch { path: PathBuf, actual: usize },

    #[error("author pubkey mismatch at {path}: filename {requested}, derived {actual}")]
    AuthorPubkeyMismatch { path: PathBuf, requested: String, actual: String },

    #[error("insecure permissions on {path}: mode 0{mode:o}, expected ≤ 0{expected:o}")]
    InsecurePermissions { path: PathBuf, mode: u32, expected: u32 },

    #[error("invalid filename in authors/: {0}")]
    InvalidAuthorFilename(String),

    #[error("identity dir does not exist and could not be created: {path}: {source}")]
    DirCreate { path: PathBuf, source: std::io::Error },
}
```

`HrpMismatch` / `Bech32Decode` apply to filename parsing only — secret-file content is raw bytes with no HRP. Each variant pins which negative test exercises it (§9 acceptance table).

## 8. Carryover change — N-12: `handle_heads_summary` split

The current `Runtime::handle_heads_summary` covers four diff cases inline:

1. **Behind** — remote has events we don't.
2. **Equal** — same heads, no action.
3. **Ahead** — we have events the remote doesn't; emit a `HeadsRequest` if the remote should backfill.
4. **Local** — remote has events that conflict with ours (equivocation).

Refactor into:

```rust
async fn handle_heads_summary(&mut self, remote: HeadsSummary)
    -> Result<(), RuntimeError>
{
    // Top-level dispatcher: validate envelope, classify diff, dispatch.
    self.validate_heads_summary_envelope(&remote)?;
    let diff = self.diff_heads_against_local(&remote);
    match diff {
        HeadsDiff::Behind { missing }   => self.handle_heads_behind(&remote, &missing).await,
        HeadsDiff::Equal                => Ok(()),
        HeadsDiff::Ahead { extras }     => self.handle_heads_ahead(&remote, &extras).await,
        HeadsDiff::LocalDivergent { .. } => self.handle_heads_local(&remote).await,
    }
}
```

The four `handle_heads_*` sub-fns + the new `diff_heads_against_local` collectively replace the current monolithic function with no semantic change. The current `#[allow(clippy::too_many_lines)]` annotation is removed, not relocated — each sub-fn is well under the limit.

Acceptance: every existing convergence test passes unchanged. No new tests needed for the refactor itself.

## 9. Acceptance tests

New test file `crates/kernel/tests/persistence.rs`:

| # | Test | Covers |
|---|---|---|
| 1 | `peer_key_round_trip_persists_across_store_reopen` | Happy-path: create store, load peer (auto-generate), drop, reopen, load peer again — pubkey + sign output match. |
| 2 | `author_key_round_trip_persists_across_store_reopen` | Same shape for `AuthorKeypair`. |
| 3 | `list_authors_returns_all_created_authors_sorted` | Create 3 authors via `create_author`, reopen, `list_authors` returns 3 pubkeys, sorted lexicographically. |
| 4 | `load_or_create_peer_is_idempotent_within_one_store` | Two sequential `load_or_create_peer` on the same store return identical pubkeys (no second key generated). |
| 5 | `load_rejects_loose_unix_permissions` *(`#[cfg(unix)]`)* | chmod `peer.key` to 0644 → load returns `IdentityError::InsecurePermissions`. |
| 6 | `load_rejects_seed_length_mismatch` | Pre-place a malformed `peer.key` (31 / 33 / 0 bytes) into the store dir before calling `load_or_create_peer` → load returns `IdentityError::SeedLengthMismatch`. The test writes the file directly via `std::fs::write` to bypass the auto-generate path. |
| 7 | `load_rejects_corrupted_filename_bech32m` | Place an `authors/garbage.key` file (no `wuser1` prefix) and call `list_authors` → returns `IdentityError::InvalidAuthorFilename`. |
| 8 | `load_author_rejects_pubkey_filename_mismatch` | Write a valid 32-byte seed into a file whose filename's embedded pubkey doesn't match the derived pubkey → `IdentityError::AuthorPubkeyMismatch`. |
| 9 | `open_creates_directory_with_0700_mode` *(`#[cfg(unix)]`)* | Open on a non-existent dir → dir exists with mode 0700. |
| 10 | `concurrent_store_writes_do_not_corrupt_key` | Two threads call `create_author` concurrently — both succeed, store contains both keys, neither file is partial. |
| 11 | `keypair_types_derive_zeroize_on_drop` | Compile-only check: `fn _assert<T: ZeroizeOnDrop>() {} _assert::<PeerKeypair>(); _assert::<AuthorKeypair>();` — verifies the derive is wired, no runtime cost, no extra dep. The `zeroize` crate's drop-time guarantee is upstream-tested; we test only that we wired the derive correctly. |

Existing B-1 convergence tests (`crates/kernel/tests/convergence.rs`) must continue to pass unchanged — this is the regression guarantee for the N-12 refactor.

Spec-coverage matrix annotations:

- `plan-b-1 §10` (peer identity stub → replaced by persistence) → tests 1, 4, 5, 6, 9, 10, 11.
- `plan-b-1 §11` (kernel runtime — author keypair handling) → tests 2, 3, 8 + convergence regression.
- `identity.md §6` (kernel custody, ZeroizeOnDrop, no-wsecret discipline) → tests 5, 6, 7, 8, 11 (the negative-path + custody proofs).

## 10. Error model and observability

`IdentityError` propagates out of `IdentityStore` methods unchanged. Callers wrap into their own error types — `Runtime::start` does not consume `IdentityStore` directly, so no `RuntimeError::Identity` variant is needed in B-2.

The `peer_warnings` log and `EquivocationFlag::peer` field are unchanged in B-2 (Q-4 deferred to B-4 — see §1).

## 11. Edge cases

- **Dir exists but is a file:** `FilesystemIdentityStore::open` returns `IdentityError::DirCreate` from the `create_dir_all` step.
- **`authors/` exists as a file:** same outcome via the second `create_dir_all`.
- **`peer.key.tmp` already exists from a previous crashed write:** `create_new` rejects; caller must remove stale `.tmp` files manually. Document this as the deliberate trade-off — silently overwriting risks racing a concurrent live write.
- **Empty `peer.key` file:** secret files are raw 32 bytes; an empty (0-byte) or wrong-size file returns `SeedLengthMismatch { actual: 0 }`.
- **Filename in `authors/` is `not-a-bech32m-thing.key`:** `list_authors` returns `InvalidAuthorFilename`. The store does NOT silently skip — surfacing helps detect accidental file placement.
- **Windows mode bits:** mode check is `#[cfg(unix)]`. Windows builds compile out the InsecurePermissions branch; document this gap in the rustdoc on `FilesystemIdentityStore`.

## 12. Non-goals (explicit)

- **No XDG/`%APPDATA%` default.** Dir is always caller-provided. B-7 owns the canonical peer-state-dir.
- **No key rotation.** `peer.key` is generated once and never re-rolled. Rotation lands when multi-device identity ([identity.md](2026-05-09-myrhiza-master-design/identity.md) §6.3) ships.
- **No HSM / OS keyring backend.** Trait is shaped to accept one in the future, not implemented now.
- **No deletion API.** `IdentityStore` has no `delete_author` / `delete_peer`. Audit-log story for key deletion is a future concern.
- **No browser backend.** jco backend's identity story is deferred; B-2 is native-only. The `IdentityStore` trait will be implementable against IndexedDB or `localStorage` in the browser-tier work without breaking changes here.
- **No replay_full O(N) fix (Q-1).** Deferred to **B-2.1**.
- **No anchor-digest off-loop fix (Q-7).** Deferred to **B-2.1**.
- **No pending-peer attribution (Q-4).** Deferred to **B-4** — requires sender identity on `GossipMessage` which iroh transport provides natively.
- **No `PendingBuffer` shape change.** Unchanged from B-1.

## 13. Surface change summary

New public surface in `myrhiza_kernel::identity`:

- `IdentityStore` trait.
- `FilesystemIdentityStore` struct + `open` constructor.
- `IdentityError` enum.

Modified existing types (private field changes only — public API is identical):

- `PeerKeypair` and `AuthorKeypair` gain `#[derive(ZeroizeOnDrop)]` and a `#[zeroize(skip)]` attribute on the public-pubkey field. No method signatures change; existing call sites compile unchanged.

Unchanged public surface:

- `PeerKeypair` constructors (`from_secret_bytes`, `deterministic`, `generate`) and methods (`sign`).
- `AuthorKeypair` constructors and `sign_body_hash`.
- `Runtime::start` signature.
- `PendingBuffer` shape (no Q-4 in this slice).

Refactor-only changes:

- `Runtime::handle_heads_summary` split into four sub-fns per N-12. Module-private; not part of public surface.

## 14. Out-of-scope future work — explicit deferrals

These come up naturally while designing B-2 but do not belong in this slice:

- **B-2.1 (next):** Q-1 (replay_full O(N) → incremental apply) + Q-7 (anchor digest off-loop or memoized by anchor identity). Both are perf changes to the runtime select loop. Plan after B-2 ships.
- **B-4:** Q-4 (pending-peer attribution) — requires sender identity on `GossipMessage` / `Subscription::recv`, which iroh's per-connection NodeID authentication provides natively. Q-4's `peer_id: Option<PeerPubkey>` field on pending entries lands as part of the iroh integration.
- **B-7:** Canonical peer-state-dir layout subsuming both keys and persisted DAG. `FilesystemIdentityStore` is one component of that layout. Likely shape: `<state-dir>/identity/{peer.key, authors/}` + `<state-dir>/dag/...`.
- **Multi-device identity module:** `myrhiza-identity-multi-device` per [identity.md](2026-05-09-myrhiza-master-design/identity.md) §6.3.
- **Browser identity store:** parallel to filesystem; implements the same trait against the browser persistence APIs.
- **HRP vocabulary consolidation spec:** consolidates HRPs across distribution.md (`wpub-author`, `wpub-myrhiza`) and B-2 (`wuser`); ratifies the kebab-vs-single-token style and reserves a forward namespace for `wpeer-*`, `wevent-*`, `wtopic-*` introductions. Low priority — current namespace has no collision.

## 15. Prior-art consultation

Decisions in §2 were grounded in the following prior-art folders (consulted via `using-prior-art` skill, 2026-05-19):

- **`prior-art/willow/identity.md`** §"Ed25519 as identity root" + §"Bech32m-with-HRP user-facing identifiers" + §"No `wsecret` HRP, ever". Willow is Myrhiza's architectural ancestor and already ships the exact pattern B-2 adopts: `Identity::load_or_generate(path)` with 0600 + atomic temp+rename + loose-perm refusal (`willow/crates/identity/src/lib.rs:196-237`, issue #126 tests at lines 615-663), `ZeroizeOnDrop + Send + Sync`, `verify_strict` (RFC 8032), and `pack_profile / unpack_profile` peer_id cross-check (the precedent for `AuthorPubkeyMismatch`). **The "no `wsecret` HRP, ever" commitment is the load-bearing direction shift in B-2**: bech32m for filename pubkeys (CLI paste-buffer surface), raw bytes for the on-disk secret. Direct lift, no novel design.
- **`prior-art/iroh/identity.md`** §"Where private keys live" + §"Rotation and backup". Iroh names "encrypted-at-rest secret-key custody as a kernel capability that apps cannot bypass" as a clean Myrhiza win over iroh's "you figure it out" default — B-2 defers this but shapes the `IdentityStore` trait so an encrypted impl drops in without breaking changes. Iroh's bare-pubkey identity model (NodeID = Ed25519 public key) validates Myrhiza's `PeerPubkey` shape; the recent `NodeId → EndpointId` rename (0.94) is a naming change only, raw bytes unchanged.
- **`prior-art/holochain/identity.md`** §"DPKI / DeepKey: the seven-year saga". Holochain shipped and then removed a planned multi-device/rotation system (DPKI) in 0.6 after seven years of effort. **Direct lesson for B-2 scope discipline: deferring multi-device + rotation is the right call.** Half-shipped identity is worse than missing identity — Holochain's strategy of "remove DPKI, document the gap honestly" is the model. B-2 follows: no rotation, no multi-device, no recovery in this slice; identity.md §6.3 documents the gap.
- **`prior-art/spritely-ocapn/persistence.md`** §"Sturdyrefs as the persistence boundary". Indirectly relevant — Spritely's swiss-num inside sturdyref pattern (durable identity that survives restart, distinct from local state) maps to B-2's `AuthorPubkey`-filename pattern: the pubkey is the durable identity, the secret is the local custody. Validates the split.
- **`prior-art/agoric-endo/persistence.md`** §"The orthogonal-persistence pattern" + §"Three-layer persistence". Not directly applicable to B-2 (which has no transcript / replay / heap-snapshot layering — that's B-7's domain). Noted for completeness: B-2's `FilesystemIdentityStore` is one component of the future canonical peer-state-dir; Agoric's "transcript is canonical, snapshot is cache" pattern will inform B-7's layout.

**Runner-up paradigms rejected:**

- Bech32m for secrets (Willow's pre-2026-04 design considered this). Rejected per the `wsecret`-never commitment. See `prior-art/willow/identity.md` §"Bech32m-with-HRP user-facing identifiers" paragraph 3.
- DPKI-style on-chain key registry. Rejected per Holochain's seven-year removal. See `prior-art/holochain/identity.md` §"DPKI / DeepKey: the seven-year saga".
- Encrypted-at-rest passphrase-prompted store. Deferred — see `prior-art/iroh/identity.md` §"Where private keys live"; trait shape supports it as future extension.
- Out-of-process keystore daemon (Holochain's lair). Out of scope for B-2; possible future `LairIdentityStore` impl, but in-process custody matches Willow's pattern and is simpler for the v1 surface.

**Remaining gaps in the prior-art corpus** (candidate triggers for future `researching-prior-art` spawns):

- No deep-dive on the `zeroize` crate / `ZeroizeOnDrop` derive semantics for Rust ed25519 keys (low priority — well-documented upstream).
- No deep-dive on encrypted-at-rest patterns for Rust filesystem keystores (low priority — relevant when B-2's encrypted-store extension lands).
- No prior-art folder on age / cosign / sigstore identity-file formats (low priority — diverges from Myrhiza's runtime-internal-keypair use case).

## 16. Sources

- [identity.md](2026-05-09-myrhiza-master-design/identity.md) §6 — IdentityScope primitive.
- [identity.md](2026-05-09-myrhiza-master-design/identity.md) §6.3 — Direction for deferred items.
- [crypto.md](2026-05-09-myrhiza-master-design/crypto.md) §9.1 — Kernel custody.
- [2026-05-10-plan-b-1-dag-memnet-design.md](2026-05-10-plan-b-1-dag-memnet-design.md) §10 — Peer identity stub.
- [2026-05-10-plan-b-1-dag-memnet-design.md](2026-05-10-plan-b-1-dag-memnet-design.md) §11 — Kernel Runtime.
- BIP-350 — Bech32m specification (used for filename public-key encoding).
- `crates/kernel/src/runtime.rs` lines 591, 785, 1067, 1272 — B-1 review-finding carryover TODOs (Q-4, N-12, Q-1, Q-7).
- `crates/manifest/src/signature.rs` `verify_signature` function (calls `VerifyingKey::verify_strict` internally, RFC 8032 strict) — existing verify path that B-2 builds on without modification.
