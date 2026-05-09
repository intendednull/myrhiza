**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — `iroh-docs` multi-author KV with range-based set reconciliation

# iroh-docs

A signed-write, eventually-consistent KV store synchronized between peers via range-based set reconciliation. Built on top of [iroh-blobs](./blobs.md) (for content) and [iroh-gossip](./gossip.md) (for live notifications). The closest thing iroh ships to "a database."

## Versions

[`n0-computer/iroh-docs`](https://github.com/n0-computer/iroh-docs); not archived. Release history is unusual: v0.29 (2024-12-04), then a long gap, then v0.97/0.98/0.99 in March–May 2026 ([releases](https://github.com/n0-computer/iroh-docs/releases)). The version jump tracks the iroh main-line; v0.99 ([release notes](https://github.com/n0-computer/iroh-docs/releases/tag/v0.99.0)) ships against `iroh = "1.0.0-rc.0"`, `iroh-blobs = "0.101"`, `iroh-gossip = "0.99"` — i.e. the ecosystem is converging toward iroh 1.0.

The crate has had ~5 commits between v0.29 and v0.97 that aren't dependency bumps. iroh-docs is **not abandoned, but it is not where active design effort is going either** — it's being kept alive against a rotating dependency set rather than evolved.

## Data model

A **document** (called a "replica" in the API) is a multi-author keyed set. Identity:

- **NamespaceId** — public key of a keypair that controls *write capability* for the document. Whoever holds the namespace secret can authorize writes; whoever knows the public NamespaceId can sync the document.
- **AuthorId** — public key of a per-writer keypair. An entry is signed by an author. Authors are application-defined; one human may have many.

Entries are tuples `(namespace, author, key)` with values `(BLAKE3 hash of content, content_len, timestamp)`. The actual content lives in iroh-blobs; the document only carries the hash. Both the namespace and the author sign each entry — namespace key authorizes "this entry belongs in the document," author key proves "this writer wrote it" ([README](https://github.com/n0-computer/iroh-docs/blob/main/README.md)).

This is **not** Willow's namespace/subspace/path model. iroh-docs is one layer of namespacing (the NamespaceId) plus a per-author signature; there is no hierarchical path, no subspace prefix-deletion. Closer to "a sharded Git repository" than "a filesystem."

## Conflict resolution

The README and proto docs describe iroh-docs as handling "concurrent updates from multiple peers, ensuring eventual consistency without conflicts" ([proto docs](https://docs.iroh.computer/protocols/kv-crdts)) but do not specify the conflict-resolution rule. From reading the source: **last-writer-wins by `(timestamp, author_id)`** for the same `(namespace, author, key)` triple. Different authors writing the same key produce parallel entries — the document is keyed on `(author, key)`, not just `key`, so cross-author writes don't conflict, they coexist.

This is intentionally weaker than a CRDT-with-merge: there is no built-in mechanism for "merging" two authors' writes to the same logical key. Applications either accept the multi-entry-per-key shape or layer their own resolution on top. The "CRDT" in `kv-crdts` is somewhat aspirational — what actually ships is a multi-author timestamped KV with set-reconciliation sync.

## Sync: range-based set reconciliation

The convergence algorithm is **range-based set reconciliation (RBSR)** from [Aljoscha Meyer's 2022 paper](https://arxiv.org/abs/2212.13567) ([README cites it](https://github.com/n0-computer/iroh-docs/blob/main/README.md)). The shape:

1. Both peers sort their entries by some total order (here: `(author, key)`).
2. They exchange a **fingerprint** (hash) of all entries in some range `[lo, hi)`.
3. If fingerprints match, the range is fully reconciled — done.
4. If they differ, the range is recursively split and step 2 repeats on subranges.
5. At small enough ranges, peers exchange the actual entry lists.

The bandwidth cost is `O(d log n)` where `d` is the number of differences and `n` is set size. Two fully-synced peers exchange one fingerprint per session. The technique is what makes iroh-docs scale as set sizes grow — alternative naive sync is `O(n)` per peer pair per round.

Live propagation rides on **iroh-gossip** for the same NamespaceId topic: when a new entry lands, peers in the topic get a notification and pull the deltas immediately rather than waiting for the next periodic sync. Without gossip, sync still works, but only on demand.

## Position relative to iroh-willow

The obvious question: is iroh-docs being deprecated in favor of iroh-willow? **Officially, no.** Neither the iroh-docs README nor any release note mentions deprecation, and the v0.99 release happened today (2026-05-08).

**Practically, partly.** iroh-willow implements the same RBSR primitive (called "3dRBSR" in Willow because it ranges over namespace × subspace × path) plus everything iroh-docs lacks: hierarchical paths, prefix deletion, capability-system-based access control via Meadowcap, owned-vs-communal namespaces. If iroh-willow ever reaches feature-complete production status, there is no obvious reason to keep using iroh-docs except inertia. But iroh-willow has not had a meaningful code change since March 2025 (see [willow.md](./willow.md)), so the "willow eats docs" migration is not happening on any visible timeline.

The honest read: iroh-docs is the *available* answer; iroh-willow is the *intended* answer; the gap between them has been static for over a year.

## Implications for Myrhiza

iroh-docs is a tempting fit for Myrhiza state-replication if you squint, but the shape is wrong for Myrhiza's authority model:

- **Determinism.** Myrhiza state-apply must be a pure function of `(prior state, event)`. iroh-docs gives you eventually-consistent multi-writer KV; the order in which entries arrive at a peer is non-deterministic, so any state derived from "iterate this document" is non-deterministic by construction. A state-apply component cannot read iroh-docs directly. To use it, you'd have to define an event log on top — entries are events, state-apply is a deterministic fold over the log sorted by `(timestamp, author_id)`. Doable, but iroh-docs adds a layer you mostly want to bypass.
- **Authority.** iroh-docs writes are signed by the author key; the namespace key authorizes who *can* be an author. That's a closed-membership model with an admin. Myrhiza apps will frequently want open-membership semantics (anyone with this app can write events) or capability-discriminated semantics (only this delegation chain can write). Neither fits iroh-docs' single-NamespaceId-grants-everything model cleanly.
- **Deletion.** iroh-docs has tombstones for entries you wrote, but no equivalent of Willow's prefix-pruning. Hard to express "delete everything under `/group-a/`" without iterating.
- **Pragmatic value as a building block.** The RBSR sync is genuinely the right algorithm. The lesson worth stealing isn't the iroh-docs API; it's "use range-based set reconciliation, build the data model on top." If Myrhiza writes its own state-sync layer, it should be RBSR, not gossip-flood.

For app-level KV (settings, profiles, contact lists) iroh-docs would be fine. For the deterministic state-apply core, it's the wrong abstraction layer — too much policy baked in, not enough determinism guaranteed.

## Sources

- [iroh-docs repository](https://github.com/n0-computer/iroh-docs)
- [iroh-docs README on main](https://github.com/n0-computer/iroh-docs/blob/main/README.md)
- [iroh-docs releases](https://github.com/n0-computer/iroh-docs/releases)
- [iroh-docs v0.99.0 release notes](https://github.com/n0-computer/iroh-docs/releases/tag/v0.99.0)
- [iroh-docs Cargo.toml @ v0.99.0](https://github.com/n0-computer/iroh-docs/blob/v0.99.0/Cargo.toml)
- [iroh-docs proto docs](https://docs.iroh.computer/protocols/kv-crdts)
- [Range-based set reconciliation paper (Meyer 2022)](https://arxiv.org/abs/2212.13567)
- [iroh-blobs — sibling doc](./blobs.md)
- [iroh-gossip — sibling doc](./gossip.md)
- [iroh-willow — sibling doc](./willow.md)
