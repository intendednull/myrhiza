**Date:** 2026-05-22
**Status:** active
**Subject:** Rotation mechanisms across DID methods — the load-bearing-for-Myrhiza file. Plan B-2 (persistent identity) wants long-term-identity-vs-active-signing-key separation; this file audits how each method does (or doesn't) provide it.

# Rotation — how DID methods handle long-term identity vs active signing key

Myrhiza Plan B-2 ([`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`](../../specs/2026-05-19-plan-b-2-persistent-identity-design.md)) needs a way to express: **"this is the same author across time, even though the key that signs has changed."** [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md) is the existing in-tree deep-dive on `did:plc`'s answer; this file generalizes the comparison across all eight methods.

The asymmetry every well-designed identity system seems to converge on is **two key tiers**:

- **High-authority, infrequently-used keys** that can rewrite identity state. Kept offline / cold. Multiple, with priority ordering for recovery.
- **Low-authority, frequently-used keys** that sign normal operations. Kept warm / on-device. Rotated frequently.

Some methods bake this in (`did:ion`, `did:plc`); some don't (`did:key`, `did:web`); some bake it differently (`did:webvh`).

## Comparison table

| Method | Long-term identity | Recovery key tier | Update/rotation key tier | Active signing key tier | Recovery from total active-key loss? |
|---|---|---|---|---|---|
| `did:web` | The domain | Domain registrar | (Same as recovery — domain control) | Whichever keys the DID doc lists | Yes, via DNS recovery |
| `did:key` | The key | None — key *is* identity | None | (Same as long-term) | **No** |
| `did:ion` | The DID suffix (commit hash) | Recovery key (offline) | Update key | Signing keys in DID doc | Yes — recovery key rewrites |
| `did:peer` | Per-pairwise — no aggregate | Pairwise key exchange | Out-of-band sync | Whichever in DID doc | No — pairwise reset |
| `did:plc` | The DID suffix | Rotation keys (1–5, priority-ordered, offline) | (Same as recovery — rotation keys) | `#atproto` signing key | Yes — higher-priority rotation key, 72h window |
| `did:webvh` | The DID suffix (scid) | Authorized update keys (in current log entry) | (Same as recovery, with pre-rotation commits) | Verification methods in DID doc | Yes, if the update key is preserved + log is intact |
| `did:ethr` | The Ethereum address | EOA private key (or smart-wallet quorum) | (Same as recovery) | Delegated keys via setAttribute | Only if smart-contract wallet allows |
| `did:cheqd` | The DID UUID | Controller keys | (Same as recovery) | Verification methods in DID doc | Only if multi-controller |

**Four distinct architectural patterns emerge:**

1. **One-tier (no rotation):** `did:key`. Key = identity. Lose key, lose identity.
2. **Substrate-derived recovery (no protocol mechanism):** `did:web` (DNS), `did:ethr` (EOA), `did:cheqd` (Cosmos transaction signer). Recovery is whatever the underlying substrate provides.
3. **Protocol-level recovery key, single tier:** `did:webvh`. Authorized-key concept; recovery and update are the same tier.
4. **Protocol-level recovery key, two tiers:** `did:ion`, `did:plc`. Distinct recovery key + signing key. **This is the model Myrhiza Plan B-2 should adopt.**

The two-tier model is the prior art Myrhiza needs. Both at-scale productions of it (`did:plc`'s 12M+ identities, ION's pre-2023 Microsoft deployment) confirm the model is operationally tractable. Let's look at the two tier-bearing methods in depth.

## `did:ion`'s three-key model

ION (via Sidetree) distinguishes **three** key tiers, not two:

| Tier | Purpose | Operations | Storage |
|---|---|---|---|
| **Recovery key** | Reset the active key set entirely | Sign `recover` operations | Cold storage; rarely used |
| **Update key** | Modify the DID document (keys, services) | Sign `update` operations | Warm — used on every doc change |
| **Signing keys** | What's in the resolved DID document | Sign credentials, authenticate, etc. | Hot — on-device |

Each operation references the prior operation by hash, signs with the appropriate tier's key, and commits to the *next* tier's key (forward-secure commitment). The Sidetree node batches operations and anchors a commit hash on Bitcoin.

**Why three tiers, not two?** ION explicitly separates "I need to recover from a compromised hot key" (use update key) from "I need to recover from a compromised everything-but-recovery-key" (use recovery key). The model assumes the update key is more vulnerable than the recovery key because it gets used more often.

**Trade-off:** Three keys to manage. The pre-rotation commitment scheme (each operation commits to the hash of the *next* update key, not the key itself) gives quantum-resistance for rotation but adds complexity.

**Operational fate:** Despite the architectural elegance, ION's operator ecosystem collapsed — Microsoft Entra Verified ID removed `did:ion` in December 2023, and no large operator replaced them. See [`abandoned.md`](abandoned.md). **The architecture is right; the deployment context was wrong.**

## `did:plc`'s rotation-key set

`did:plc` (covered in detail at [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md)) takes the two-tier model and folds the "update" + "recovery" tiers into a single concept — **rotation keys**:

| Tier | Purpose | Operations | Storage |
|---|---|---|---|
| **Rotation keys** (1–5, priority-ordered) | Sign PLC operations; rewrite history | Each PLC op signed by a rotation key | Priority 0 (highest) typically offline; lower-priority keys on operator-controlled servers |
| **Signing key** (one, `#atproto`) | Sign repo commits | In the resolved DID doc | Hot — on PDS |

**Key observation:** rotation keys are NOT in the resolved DID document. They live only in the operation log. A resolver fetching the DID doc to verify a repo commit sees only the `#atproto` signing key.

**Recovery mechanism:** Any rotation key can submit an op signed by itself. **A higher-priority rotation key can override (rewrite history within) operations submitted by a lower-priority key, within a 72-hour window.** This is the recovery story: keep your priority-0 key offline; if your PDS is compromised and the attacker submits a malicious op using a lower-priority rotation key, you have 72 hours to roll it back with your priority-0 key.

**Trade-off vs ION:** Fewer key tiers (rotation vs signing, not recovery/update/signing). The 72-hour window is a hard tradeoff: too short → false-positive recoveries from legitimate operators; too long → attacker has more time to exfiltrate keys before lock-in. Bluesky picked 72 hours; the spec acknowledges it's a tuneable.

**Operator centralization:** `plc.directory` is the canonical log operator. This is the [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md) headline criticism — "decentralized identifier" is misleading when one company runs the directory. Bluesky has published "future federation" intent but not delivered.

## `did:webvh`'s log-based rotation

`did:webvh` is the architectural middle ground:

- **No central operator** (it's HTTPS + DNS, like `did:web`).
- **No blockchain anchor** (like `did:web`, unlike `did:ion`).
- **Has verifiable history** (unlike `did:web`).

The mechanism: every DID document version is appended to a JSONL log alongside the document. Each entry contains:

- `versionId`: a sequence number + hash.
- `prevHash`: hash of prior entry (Merkle chain).
- `did`: the full DID document for this version.
- `proof`: signature by an *authorized update key* from the prior version.
- `nextKeyHashes`: hashes of the *future* authorized keys (pre-rotation commitment, like ION).

Resolution: fetch `did.jsonl`, verify each entry's signature + prevHash + prior-entry-authorized-key, return the latest document.

**Recovery:** Any key authorized by the prior version can sign the next entry. Multiple authorized keys give a recovery-like effect — lose one, sign with another. Pre-rotation commitments (`nextKeyHashes`) hedge against future quantum break.

**Trade-off vs `did:plc`:** No rewrite-window concept. Once an entry is in the log it's anchored by the chain — there's no "the next entry can erase the prior." A compromise that signs a malicious entry can be overridden only by *additional* entries from authorized keys. This means recovery is *append-only*, not *rewrite-the-past* like `did:plc`. Whether this is better or worse depends on threat model: append-only is auditable but doesn't let you "undo" a compromise; rewrite-window lets you undo but creates a narrow attack window.

**Witness mechanism:** Optional — a configured set of external "witness" servers can attest to log state. A resolver can require that the resolved log was attested by at least N of M witnesses. Adds an external-attestation layer Myrhiza could imitate via gossip.

## Implications for Myrhiza Plan B-2

The Plan B-2 design ([`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`](../../specs/2026-05-19-plan-b-2-persistent-identity-design.md)) currently has one identity tier (`AuthorKeypair` per user, single Ed25519 key, persisted as raw bytes on disk). This corpus's contribution: **at-scale productions of identity systems all converge on the two-tier (or three-tier) model.** Plan B-2's single tier is structurally insufficient for a multi-year identity that survives device loss.

The shape Plan B-2 should adopt, blending the lessons:

| Tier | Lift from | What Myrhiza does |
|---|---|---|
| **Rotation key set (offline-capable, priority-ordered)** | `did:plc` | A set of N rotation keys (Ed25519 — see [`crypto.md`](crypto.md)); priority-0 generated at first-run and recommended-offline; lower-priority keys for delegated authority |
| **Signing key (active, on-device, frequently-rotated)** | `did:plc`'s `#atproto` | Per-device Ed25519 keys, registered/revoked via signed identity events |
| **No central directory** | `did:webvh` | Identity events on Myrhiza's per-author DAG, not a central log; gossip-propagated, witnessable by peers |
| **Pre-rotation commitments** | `did:ion`, `did:webvh` | Each identity event commits to the hash of the *next* rotation key, giving quantum-hedge |
| **Rewrite window (or not?)** | `did:plc` (72h) | **OPEN.** Append-only (webvh-style) is simpler and matches Myrhiza's DAG semantics; rewrite-window (plc-style) needs a global clock and conflict resolution. **Probable answer: append-only.** |

**Don't lift:**

- The DID-document JSON format itself. Myrhiza already has a per-author event DAG; serializing the same state as a DID document for external interop is a *bridge* concern, not a primary-storage concern.
- Bitcoin / Ethereum / Cosmos anchoring. Myrhiza's per-author DAG already provides causal ordering.
- A central registry/directory operator. Myrhiza is P2P; the gossip layer (`prior-art/iroh/`) is the registry.

**Open questions for Plan B-2:**

1. How many rotation keys? `did:plc` blesses 1–5; `did:ion` blesses one recovery + one update at a time. Single-rotation-key is simpler but loses the priority-ordering recovery story.
2. Append-only log vs rewrite-window? Probable append-only, but the threat model needs the audit.
3. Cross-device key registration ceremony — how does Device B learn about (and trust) Device A's signing key? See [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Multi-device identity" and [`prior-art/signal/identity.md`](../signal/identity.md) for the PNI/ACI model.
4. Key types: Ed25519 across the board (Myrhiza already uses), but Cremers ETK 2025 ([`prior-art/mls/`](../mls/)) flags ECDSA risks — if rotation keys ever need EUF-CMA in a stronger model, Ed25519's malleability story needs auditing.

## Cross-method lessons (synthesis)

Five rotation-design lessons worth carrying:

1. **Two tiers minimum.** Any method with one key tier (`did:key`, `did:peer` per-pair) has no recovery story. Disqualified for multi-year identity.
2. **Recovery keys must be storable offline.** `did:plc`'s priority-0, `did:ion`'s recovery, `did:webvh`'s authorized-key — all converge on "this key is rarely used, generate it once, write it down on metal."
3. **Pre-rotation commitments are cheap quantum hedging.** Both `did:ion` and `did:webvh` use them; `did:plc` doesn't. Myrhiza should.
4. **History matters.** `did:web`'s "no history" model is the weakness BC Government built `did:webvh` to fix. Myrhiza's DAG provides this for free.
5. **Don't pick a central operator unless you have to.** `did:plc`'s centralization is its biggest critique. The Trust-over-IP push for `did:webvh` is explicitly motivated by avoiding it.

## Sources

- `did:plc` deep-dive in-tree — [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md).
- Plan B-2 design — [`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`](../../specs/2026-05-19-plan-b-2-persistent-identity-design.md).
- Willow multi-device identity open problem — [`prior-art/willow/open-problems.md`](../willow/open-problems.md).
- Signal PNI/ACI comparator — [`prior-art/signal/identity.md`](../signal/identity.md).
- MLS Cremers ETK 2025 ECDSA finding — [`prior-art/mls/`](../mls/).
- Sidetree v1.0.0 spec — <https://identity.foundation/sidetree/spec/>.
- `did:webvh` v1.0 spec — <https://identity.foundation/didwebvh/>.
- `did:plc` spec — <https://github.com/did-method-plc/did-method-plc>.
- ION repository — <https://github.com/decentralized-identity/ion>.
