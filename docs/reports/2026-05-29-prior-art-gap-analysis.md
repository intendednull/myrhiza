**Date:** 2026-05-29
**Status:** active

# Prior-art gap analysis — what's worth filling out next

A one-shot audit of where Myrhiza's `docs/prior-art/` corpus (29 folders as of
this date) lacks an external reference, found by mapping the master spec's
decision surfaces and named-but-deferred open problems against existing
coverage.

**Bottom line:** the corpus is broadly complete on the *paradigm* axes —
convergence (the four-paradigm survey: agoric-endo / crdts / croquet /
holochain), capability *semantics*, crypto, identity, transport, RPC, and
verification all have homes. The gaps cluster in three places: one true
structural hole (on-disk persistence has no section), several
named-but-deferred spec decisions being reinvented uncited, and a couple of
timing-gated maybes. Every recommendation below is anchored to a specific spec
section.

## Method & provenance

Produced by two multi-agent passes (a `prior-art-gap-scan` workflow + a
`prior-art-gap-recover` follow-up that re-ran four lenses whose structured
output was dropped in the first pass). Candidates were generated across eight
lenses (storage, set-reconciliation, capability-OS, local-first products,
decentralized messaging, deterministic-execution, supply-chain trust, and a
wildcard), then adversarially vetted — each vetter read the most-related
existing folder's `README`/`lessons`/`open-problems` and the cited spec
sections before assigning a verdict. Vetting defaulted to SKIP on redundancy or
speculation. Relevance/novelty scores and quoted line numbers live in the
workflow transcripts.

## Tier 1 — clear adds (strongest evidence)

### New section: `Storage & persistence` (the one real structural hole)

No prior-art section covers the storage substrate, yet `CLAUDE.md` states "the
kernel owns storage." `agoric-endo/persistence.md` explicitly tells the reader
to "verify the ACID story before committing… pick once, commit hard" — and
never does that verification.

- **`embedded-storage-engines`** — redb / fjall / sled comparative survey
  (SQLite, RocksDB, LMDB as references) on the axes a pick-once-commit-hard
  kernel-storage decision needs: ACID across heterogeneous state,
  crash-consistency (WAL / fsync / tail-corruption recovery), single-writer
  many-reader concurrency, **on-disk format stability across versions**, and
  zero-external-dep embeddability. Format-stability is load-bearing — a format
  break forks every peer's local state.
  *Consult before any spec on the B-9 storage layer, B-7 persistent DAG, the
  `maintenance.md §12.2` Persister module, or the unbacked `host.kv` per-peer
  store.*

- **`content-addressed-blockstore`** — git packfile+GC / restic prune /
  IPFS-`boxo` (GCBlockstore + GCLocker): how local-first systems persist a
  content-addressed blob graph *on disk* — object packing, dedup / refcount /
  pin retention, mark-and-sweep GC against live roots, concurrent-GC-vs-serve
  safety. The on-disk counterpart to iroh's wire-side blob transfer. Cite
  `boxo/blockstore`, not the older `go-ipfs-blockstore` path.
  *Consult before any spec on the kernel-owned local blob/event store (`FsStore`,
  deferred from B-10), the snapshot-cache retention model (`convergence.md
  §4.2`, `risks.md §17`), or log-truncation/GC-against-live-roots
  (`convergence.md §200`).*

### `update-framework-trust-models` (TUF + Uptane + threshold signing)

A corpus-wide grep returns **zero** occurrences of "TUF" / "Uptane" / "update
framework" in `docs/specs/`, yet `distribution.md §10.9–10.10` (built-in
Ed25519 pubkey allowlist + three offline backup keys + kernel-signing-root
distinct from module-signing allowlist) and `B-10 §3.3` (monotonic
`revocation-seq` with `MAX_REVOCATION_JUMP`) are **hand-rolling TUF's
root/targets/snapshot/timestamp role separation and version-number rollback
resistance uncited**. `app-distribution/signing.md` (the most-related existing
folder) is strictly tool-mechanics (Cosign / Fulcio / Rekor / Notation) and
lists "compromised signer at time of signing" and "root-of-trust compromise"
under *"Doesn't defend"* — exactly TUF's design center — then points elsewhere
without grounding it. TUF is CNCF Graduated (2019), deployed in PyPI, Sigstore's
own root, RustSec, Bottlerocket; Uptane ships in production vehicles.
*Consult before any spec on module/kernel signing-root design, key rotation,
threshold signing, or revocation freshness (`distribution.md §10.7–10.10`).*
Fold binary-transparency + reproducible-builds material in here as files rather
than spinning separate folders.

### `matrix-state-resolution` (Convergence / authority-DAG)

Matrix State Resolution v2 — auth events, auth chains, power levels, the
**state-reset hazard**, room versions, and the Aug-2025 v2.1 "Project Hydra"
fix. Zero corpus coverage; the only deployed-at-scale, still-churning reference
on the exact pitfall Myrhiza's uniform lexicographic `EventHash` tie-break
courts: authority-*changing* events may need power-topological, not plain
lexicographic, ordering — or you get state resets.
*Consult before any spec on concurrent-event tie-breaking (`convergence.md
§4.1`), the deps-monotonicity invariant (`§4.4`), or the deferred RBAC / warrant
modules (`§4.4.1`, `§4.5`).*

### `capability-os-lineage` (KeyKOS → EROS → Coyotos → seL4)

The kernel/OS-side object-capability tradition that is the direct intellectual
root of "capabilities are the only host surface" — the kernel-design companion
the language-side folders (spritely-ocapn, agoric-endo) and WASM-platform
folders only cite as ancestry. seL4 is the live anchor (ongoing verification
under the Linux Foundation); KeyKOS / EROS / Coyotos are
historical-but-load-bearing. **Consolidates** two otherwise-separate candidates:
Genode / Sculpt OS and the deployed isolation substrates (FreeBSD Capsicum,
Fuchsia/Zircon handle-objects, gVisor) belong *inside* this folder as exemplars.
*Consult before any spec on the four-layer capability gating model
(`capabilities.md §7`, `M_eff = A_ambient ∩ M_required`), the kernel TCB
boundary (`abi.md §8`, kernel-is-the-call-broker), or any revisiting of the
capabilities-are-the-only-host-surface axiom.*

### `blockchain-wasm-metering` (CosmWasm / NEAR / Soroban / PolkaVM-JAM)

Highest-scored candidate of the run. The only production systems that have
shipped, debugged, *and version-migrated* native-WASM instruction-count gas as a
hard cross-validator consensus invariant: how four deterministic-WASM-shaped
VMs make instruction-count gas byte-identical across x86_64 / aarch64 /
riscv64, treat recalibration as a protocol-version bump, and the
metering-bug incident corpus (CosmWasm fixed two Wasmer Singlepass metering bugs
upstream — the point being metering is hard). The external-experience companion
to `wasm-component-model`'s fuel-vs-epoch section.
*Consult before the deferred fuel-cost-table child spec (`determinism.md §5.3`),
the DoS-asymmetry risk (`risks.md §19`), or any Wasmtime-LTS-bump migration plan
(recalibration-as-kernel-major rule).*

## Tier 2 — solid, tied to more-deferred surfaces

- **`range-based-set-reconciliation`** — Negentropy / Nostr NIP-77 (strfry,
  rust-nostr) as the cleanest production embodiment of Aljoscha Meyer's
  Range-Based Set Reconciliation, plus Merkle Search Trees (atproto repos) and
  Prolly Trees (Dolt/Noms). Subsumes the already-noted "Willow-protocol
  deep-dive" future candidate (same RBSR lineage). *Serves the verbatim-deferred
  range reconciliation in `networking.md §11.3` / `convergence.md §4.5` —
  replacing the O(authors) HeadsSummary scan at wiki scale.*
- **`append-only-log-forks`** — Secure Scuttlebutt → 2P-BFT-Log. SSB's signed
  append-only feeds are the direct ancestor of Myrhiza's per-author chain;
  2P-BFT-Log supplies the concrete *irrefutable-fork-proof* construction (two
  messages sharing one predecessor) that the spec hand-waves to a "future
  warrant pattern." *Serves author equivocation / single-author-chain fork
  resolution (`convergence.md §4.4.1`).*
- **`local-first-sync-permissions`** — Jazz/cojson's per-transaction
  peer-verified Ed25519 group/role model (the shape Myrhiza adopts) vs.
  Zero / Triplit / PowerSync server-authoritative JWT-claim + query-filter
  enforcement (the trusted-middlebox shape Myrhiza rejects). Fills what
  `crdts/open-problems.md` names but leaves unsolved at the library layer
  ("CRDTs converge, then violate"). Engine/product-level, distinct from the
  library-level `crdts/` folder. *Serves the `myrhiza-permission-*` module ABI
  and the `state-apply` authority verdict.*

## Maybes — track, don't write yet

- **`failure-detectors`** (SWIM / phi-accrual / Lifeguard) — genuinely uncovered
  (the corpus has *trusted-cluster* failure detection via `erlang-otp` but not
  the open-internet randomized-probing/suspicion regime), real and deployed
  (memberlist/Serf, Cassandra, Pekko). But its strongest live anchor is **B-4.6's
  peer-authority-index eviction policy, which is explicitly post-launch** (B-4.3
  halt-detection is *self*-halt, not membership convergence). Write it the moment
  the eviction-policy spec opens.
- **`partial-replication-shapes`** (ElectricSQL Shapes / PowerSync Sync Rules /
  Zero queries) — declarative per-client subset + optimistic-apply→server-
  resolve→rebase. Lands on v2+-deferred surfaces (`convergence.md §4.5` commits
  v1 to "every peer holds everything"), and the partial-replication warning is
  already in `holochain/open-problems.md`. Fold its one durable insight
  ("partial replication breaks deterministic full-log replay from genesis") into
  `holochain/open-problems.md` or a forward-note in `convergence.md §4.5`;
  promote to a folder only at the §4.5 scaling ceiling.

## Skip — already covered or speculative

- **causal-clocks** (HLC / vector / interval-tree) — `willow/determinism.md`
  documents hash-tiebreak ordering as *decided*, HLC as materialization-only,
  with the one open question (HLC-as-`state-apply`-helper per PR #636) logged. A
  folder would revive a rejected runner-up.
- **set-difference-sketches** (IBLT / minisketch) — the runner-up to RBSR, which
  the spec already declines. High novelty here is the warning sign, not the
  virtue.
- **nostr-relay-and-event-validation** — transport / flat-event / identity
  dimensions Myrhiza explicitly rejected (recorded in `willow/networking.md:107`
  and `willow/crypto.md`); already covered by `at-protocol/` + `willow/`.
  (Negentropy/NIP-77 is the *one* Nostr-adjacent piece recommended — see Tier 2.)
- **binary-transparency** as a standalone folder — in tension with the
  `distribution.md §10.8` no-central-service non-negotiable; house it inside
  `update-framework-trust-models` until a v2 transparency-log child spec is
  scheduled.
- **InstantDB / Fireproof / Evolu / Ditto** as standalone folders — exemplars
  inside `local-first-sync-permissions` / `crdts`, not folders.
- **orthogonal persistence / TEE / zkVM / mobile push-relay** — already covered
  where load-bearing (`agoric-endo/persistence.md`, `signal` + `at-protocol`,
  `pears`); zkVM correctly speculative (no live decision surface demands proving
  remote-peer honest execution).

## Spec-hygiene findings (incidental to the audit)

1. **`docs/README.md` merge conflict** — commit `28a7d22` left an unresolved
   `<<<<<<< / ======= / >>>>>>>` block in the Runtime core section (botched
   B-6/B-10 merge). Fixed in the commit preceding this report.
2. **`distribution.md:461` is stale** — it says "FROST-Ed25519… not yet
   RFC-stable," but FROST shipped as **RFC 9591 (2024, Informational)**. The
   §10.9 threshold-vs-policy rotation decision should be re-evaluated on current
   facts (noting 9591 is Informational, not Standards-Track). Not corrected here
   — flagged for a spec edit.

## Outcome

All nine Tier-1 + Tier-2 folders were authored on 2026-05-29 via the
`researching-prior-art` workflow (author → review → polish → second-review per
folder) and are indexed in the catalog as `[active]`:
`embedded-storage-engines`, `content-addressed-blockstore`,
`update-framework-trust-models`, `matrix-state-resolution`,
`capability-os-lineage`, `blockchain-wasm-metering`,
`range-based-set-reconciliation`, `append-only-log-forks`,
`local-first-sync-permissions` (the new **Storage & persistence** section plus
additions under Sync protocols, Determinism, Capability tokens, App
distribution).

The two **maybes** were deliberately held: `failure-detectors` (SWIM) is gated
on the unwritten B-4.6 peer-eviction spec, and `partial-replication-shapes`
should land as a forward-note in `convergence.md §4.5` rather than a folder
until the scaling ceiling is hit. The corpus has no remaining structural gap
beyond those.
