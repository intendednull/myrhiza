**Date:** 2026-05-29
**Status:** active
**Subject:** Lessons for Myrhiza — the pick-once-commit-hard kernel-storage decision file (validates / avoid / borrow + recommendation matrix)

# Lessons for Myrhiza

This is the consult-this-when-designing file. The other files in this folder are
evidence; this file is the decision. **No engine is committed yet** — what
follows is a starting recommendation to audit, presented neutrally, because a
load-bearing-once-chosen folder has an incentive to soft-pedal whichever engine
it leans toward.

The decision serves four named surfaces:

- **B-9 storage layer** — the kernel's durable storage substrate (the
  [`mvp-gap-analysis`](../../reports/2026-05-21-mvp-gap-analysis.md) build slice
  "Storage layer for runtime restart": persist the event log + state snapshot,
  replay-on-restart).
- **The persistent event DAG** — the per-author Merkle event log
  ([`architecture.md`](../../specs/2026-05-09-myrhiza-master-design/architecture.md)),
  whose on-disk durability is the B-9 storage layer above.
- **`maintenance.md §12.2` Persister module** — durable storage of the event log
  ([`maintenance.md`](../../specs/2026-05-09-myrhiza-master-design/maintenance.md)).
- **`host.kv` per-peer store** — currently unbacked
  (`architecture.md §3.5`; permitted to interaction + behavior profiles).

Decision weighting (heaviest first): **on-disk format stability** → **pure-Rust
zero-external-dep embeddability** → **crash-consistency correctness** →
concurrency fit → raw performance.

## Validates

Myrhiza design choices these engines confirm:

- **Single-writer-many-reader is the right concurrency model.** redb, LMDB, and
  SQLite-WAL all converge on it, and it matches Myrhiza exactly: the kernel is
  the sole writer applying events in order; many readers (interaction/behavior
  components, sync/snapshot providers) read concurrent MVCC snapshots. The
  kernel never needs multi-writer storage — so sled's lock-free multi-writer
  ambition (and its cost) is unneeded complexity. See [lmdb.md](lmdb.md),
  [redb.md](redb.md).

- **"Kernel owns storage, components never touch it" is the mainstream design.**
  Every engine here is a library embedded in one process. The capability model
  (storage reachable only via `host.kv` for non-deterministic profiles, denied
  to `state-apply`/`state-propose`) sits cleanly above any of them. Storage is a
  host concern; the deterministic profiles never see it. Validates the
  capabilities-are-the-only-host-surface axiom for storage.

- **Pure-Rust embeddability is achievable without giving up real ACID.** redb
  and fjall both deliver full ACID, crash-consistent, MVCC storage with zero C
  toolchain. Myrhiza does not have to choose between pure-Rust and a serious
  engine. See [comparison.md](comparison.md).

- **redb already proves the integration.** `iroh-blobs` (a hard-dep subsystem)
  embeds `redb = "4.1.0"` for its file-backed blob store — the exact
  pure-Rust-CoW-B+tree shape Myrhiza needs, already running in the dependency
  tree. See [redb.md](redb.md), [`iroh/`](../iroh/).

## Avoid

Pitfalls, with Myrhiza mitigation:

| Pitfall | Source | Mitigation |
|---|---|---|
| **Treating an engine-major bump as an invisible `cargo update`.** redb broke its format v1→v2 and v2→v3 (staged migration via 2.6). A blind bump can leave a peer unable to read its own state — a self-fork. | [format-stability.md](format-stability.md), [redb.md](redb.md) | Pin the engine major in `Cargo.toml`. Treat any engine-major bump as a planned, tested, **deterministic** kernel-migration event — written and owned by Myrhiza, verified to produce identical bytes on every peer. |
| **Inheriting the cheap default durability.** fjall, RocksDB, and SQLite-`synchronous=NORMAL` default to "OS buffers, not disk" — the last writes can vanish on power loss. For a P2P node that has *already broadcast* a head, that is a self-inflicted author-chain fork. | [crash-consistency.md](crash-consistency.md) | DAG/event store: **fsync-on-commit before broadcasting the new head** — never announce a head you can't prove you durably hold. `host.kv` (local-only, no convergence stake) may use the cheaper mode. Set the knob explicitly; never inherit it. |
| **Picking sled.** Clever Bw-tree design, but no shipped stable release since 0.34.7 (2021), no 1.0, an admitted unstable on-disk format, and its own README routes reliability-first users to SQLite. | [sled.md](sled.md) | Do not adopt sled for the kernel. Study its design (below), don't depend on it. |
| **Two engines without counting the cost.** Splitting DAG (LSM) and kv/snapshots (B+tree) across two engines doubles the format-stability surface (two formats to keep migratable) — the very axis Myrhiza weights most. | [open-problems.md](open-problems.md) | Default to one engine until a bench on the real workload proves the split worth two format-stability stories. |
| **Assuming the engine's frozen format freezes Myrhiza's layout.** Even SQLite's 20-year format doesn't freeze how Myrhiza keys events/heads/snapshots inside it. | [format-stability.md](format-stability.md), [`schema-evolution/`](../schema-evolution/) | Version Myrhiza's own on-disk schema independently (a schema-version byte + migration plan), separate from the engine's format. |
| **Single-maintainer bus factor.** redb (`cberner`) and fjall (`marvin-j97`) are effectively one-person projects; redb still finds corruption bugs in 2026. | [open-problems.md](open-problems.md) | Pin + own-the-migration + keep a documented exit path (the data is bytes in a B-tree/LSM; migrating to another engine is feasible). Treat sled's stall as the proof this risk is real. |

## Borrow

Primitives worth studying / adopting:

- **redb's god-byte two-phase commit** ([crash-consistency.md](crash-consistency.md))
  — a 512-byte super-header, two commit slots, one byte of three flags
  (`primary_bit` + `recovery_required` + `two_phase_commit`) selecting the live
  commit and recording whether it is provably valid. The cleanest small example
  of crash-safe atomic commit without a WAL; worth understanding even if Myrhiza
  just uses redb rather than reimplementing it.
- **LMDB/redb copy-on-write shadow paging** — durability without a log; the
  on-disk structure is always valid. The reference model for "crash-safe by
  construction."
- **RocksDB/fjall column-families / keyspaces with a shared atomic WAL** — the
  pattern for per-topic/per-app partitioning with cross-partition atomic event
  batches. fjall already exposes it as keyspaces + cross-keyspace atomics.
- **WiscKey-style key-value separation** (fjall's `value-log`) — keep large
  payloads out of the tree to cut write amplification. Relevant if events carry
  large inline blobs (though Myrhiza's blob path is content-addressed via iroh —
  see the `content-addressed-blockstore` gap).
- **The sled→komora/marble GC-vs-space redesign** — reference if Myrhiza ever
  needs a multi-writer local store; not a v1 dependency.

## Recommendation matrix (keyed to the four surfaces)

| Surface | Workload | Recommended | Why / runner-up |
|---|---|---|---|
| **Persistent event DAG** (B-9 storage) | append-mostly, sequential writes, range scans by author/seq | **fjall** *or* **redb** — bench first | LSM (fjall) suits sequential append; but redb's CoW B+tree handles append + range fine and keeps one engine. Runner-up RocksDB rejected on pure-Rust embeddability. |
| **`maintenance.md §12.2` Persister** | durable wrapper over the DAG store | **same engine as the DAG store** | The Persister is a module over the DAG; it should not introduce a second format. Durability set to fsync-before-broadcast. |
| **Materialized state + snapshot cache** | read-heavy, point + range lookups, frequent overwrite | **redb** | CoW B+tree is read-optimized; snapshots map onto MVCC read txns. LMDB is the proof-of-design, redb is its pure-Rust form. |
| **`host.kv` per-peer store** | read-heavy point lookups + `list-prefix`, local-only | **redb** | B+tree range/prefix scans fit `list-prefix`. Local-only ⇒ cheaper durability mode acceptable. |

**Single-engine default lean: redb.** It wins three of four surfaces, is the
read-optimized B+tree shape three of them want, is pure Rust, is already in the
dependency tree via iroh-blobs, has a stable-format *commitment* (with the
two-historical-breaks caveat managed by pinning + owned migration), and its
single-writer-many-reader model matches the kernel exactly. The honest
counter-argument: the **persistent event DAG is append-heavy**, where fjall's LSM
economics may win — so the DAG is the one surface to **benchmark redb-vs-fjall on
the real event workload before committing**, rather than assuming. If the bench favors
fjall decisively for the DAG and the rest stays redb, re-open the
one-engine-vs-two question in [open-problems.md](open-problems.md) §2 with the
doubled-format-risk cost explicit.

**What is NOT recommended:** sled (no stable format/release); SQLite/RocksDB/LMDB
as the *adopted* engine (C/C++ deps break pure-Rust embeddability) — they remain
the yardsticks: SQLite/LMDB for format stability, RocksDB for LSM performance.

## Sources

- https://crates.io/api/v1/crates/redb
- https://crates.io/api/v1/crates/fjall
- https://crates.io/api/v1/crates/sled
- https://raw.githubusercontent.com/cberner/redb/master/CHANGELOG.md
- https://raw.githubusercontent.com/spacejam/sled/main/README.md
- https://github.com/cberner/redb/blob/master/docs/design.md
- https://raw.githubusercontent.com/n0-computer/iroh-blobs/main/Cargo.toml
- https://www.sqlite.org/formatchng.html
