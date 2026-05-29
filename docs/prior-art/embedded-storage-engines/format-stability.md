**Date:** 2026-05-29
**Status:** active
**Subject:** On-disk format stability — the load-bearing axis; who commits to what, who has broken it

# On-disk format stability

The single heaviest axis for a Myrhiza kernel-storage pick, and the one a
casual benchmark ignores. **A format break forks every peer's local state.** If
the kernel ships a new version whose storage engine cannot read the on-disk
files the previous version wrote, every peer that upgrades either (a) silently
loses its DAG and materialized state, or (b) must run a migration that, if it
fails or differs across peers, produces divergent local state. Either is a
correctness failure for a convergent P2P system, not a mere inconvenience.

This is why the gap analysis flagged the axis explicitly
([`docs/reports/2026-05-29-prior-art-gap-analysis.md`](../../reports/2026-05-29-prior-art-gap-analysis.md)):
"Format-stability is load-bearing — a format break forks every peer's local
state."

## The ladder (best → worst for Myrhiza)

### SQLite — frozen since 2004 (the bar)

Every SQLite 3 release reads/writes files from the first 3.0.0 (2004-06-18).
20+ years, no backward-incompatible format break. Forward compatibility is not
guaranteed (new feature → old reader may refuse), but a file written today is
readable by every future version. **This is the gold standard.** No pure-Rust
candidate matches it. See [sqlite.md](sqlite.md).

### LMDB — very stable, rare documented bumps

A decade-plus as the OpenLDAP backend with a deliberately conservative format.
Bumps are rare and documented. Second only to SQLite. See [lmdb.md](lmdb.md).

### RocksDB — format evolves, library reads old

The SST format gains versions over time, but the library is engineered so the
current version reads older on-disk data (mandatory for fleet rollouts without
downtime). Contract: "current library reads old data," not "format frozen." A
moving target the library manages. See [rocksdb.md](rocksdb.md).

### redb — stable *commitment*, but has broken twice with migration paths

This is the nuance that matters most, because redb is the most likely Myrhiza
candidate. README: *"The file format is stable, and a reasonable effort will be
made to provide an upgrade path if there are any future changes to it."* But the
verified changelog shows **two backward-incompatible format breaks**, each
mitigated by a migration:

- **2.0.0** — new format (constant-time `len()`); *"not backwards compatible
  with 1.x."*
- **3.0.0** (2025-08-09) — *"Removes support for file format v2 … Use
  `Database::upgrade()`, in redb 2.6, to migrate to the v3 file format."* The v3
  format is the current one.
- **4.0.0** (2026-04-02) — **not** a format break (data-loss bug fix).

The critical operational detail: the v2→v3 migration is **staged** — you migrate
*while on redb 2.6*, then bump the library to 3.x. A peer that skipped 2.6
cannot jump an old 2.x file straight to 4.x. For Myrhiza this means a redb
major bump is a **planned kernel-migration event**, not an invisible
`cargo update`.

### fjall — "major-bump + migration path," but young

Same stated posture as redb (*"Future breaking changes will result in a major
version bump and a migration path"*), but fjall first shipped end-2023 and is on
3.x with a much shorter track record. Less battle-tested format. See
[fjall.md](fjall.md).

### sled — no stable format, by admission (worst)

sled's own README: *"the on-disk format is going to change in ways that require
manual migrations before the 1.0.0 release!"* — and 1.0 has never shipped (stuck
at `1.0.0-alpha.124`, 2024-10-11). Disqualifying for a pick-once decision. See
[sled.md](sled.md).

## Implications for Myrhiza — design the format boundary deliberately

The takeaway is not "pick the most-frozen engine and forget it." It is **own the
format boundary in the kernel regardless of engine:**

1. **Pin the engine major in `Cargo.toml`.** Never let a transitive bump change
   the on-disk format silently. (This mirrors the iroh lesson:
   [`iroh/lessons.md`](../iroh/lessons.md) — vendor-pin, bump deliberately.)
2. **Treat an engine-major bump as a kernel-format-version event** with an
   explicit, tested migration — written and owned by Myrhiza, not delegated
   blindly to the engine's `upgrade()` helper. Verify the migration is
   *deterministic and identical across peers* (a migration that produces
   different bytes on different peers is a fork).
3. **Version the kernel's own on-disk layout independently of the engine's.**
   Myrhiza's schema *inside* the key-value store (how events, heads, snapshots,
   `host.kv` entries are keyed and encoded) is a second format that can break
   even if the engine's format does not. That layer is
   [`schema-evolution/`](../schema-evolution/) territory — Myrhiza needs an
   on-disk schema-version byte and a migration plan there too.
4. **Prefer the engine whose format break history + commitment you can live
   with.** SQLite/LMDB (most stable) cost pure-Rust embeddability; redb is the
   pragmatic middle (stable commitment, two historical breaks, both migratable,
   pure Rust, already in the dep tree); fjall is younger; sled is out.

## Sources

- https://www.sqlite.org/onefile.html (SQLite's backward-compatibility promise)
- https://www.sqlite.org/formatchng.html
- https://raw.githubusercontent.com/cberner/redb/master/CHANGELOG.md
- https://raw.githubusercontent.com/cberner/redb/master/README.md
- https://raw.githubusercontent.com/fjall-rs/fjall/main/README.md
- https://raw.githubusercontent.com/spacejam/sled/main/README.md
- http://www.lmdb.tech/doc/
- https://github.com/facebook/rocksdb/wiki/RocksDB-Overview
