**Date:** 2026-05-09
**Status:** active
**Subject:** Myrhiza design space — identity, crypto, capabilities

Mining of `prior-art/mls/`, `prior-art/spritely-ocapn/`, `prior-art/agoric-endo/`, `prior-art/willow/{identity,crypto,authority,runtime-vision}.md`, `references/local-first.md`, plus the Willow seal-gift-wrap and epoch-key-rotation specs. Output for the brainstorming session that locks the master spec for Myrhiza's identity custody, multi-device + behavior identity unification, capability discipline, crypto host imports, MLS adoption shape, and post-quantum migration path.

## Domain index

1. [Identity custody](#1-identity-custody)
2. [Multi-device + behavior identity unification](#2-multi-device--behavior-identity-unification)
3. [Capability discipline (per-call vs per-import vs full ocap)](#3-capability-discipline-per-call-vs-per-import-vs-full-ocap)
4. [Crypto host imports — shapes, returns, profile gating](#4-crypto-host-imports--shapes-returns-profile-gating)
5. [Permission model — app-defined vs kernel tiers](#5-permission-model--app-defined-vs-kernel-tiers)
6. [MLS adoption shape](#6-mls-adoption-shape)
7. [Post-quantum migration path](#7-post-quantum-migration-path)
8. [Multi-device recovery / device add+revoke](#8-multi-device-recovery--device-addrevoke)
9. [Cross-peer capability protocol — CapTP/OCapN vs re-invent](#9-cross-peer-capability-protocol--captpocapn-vs-re-invent)
10. [Distributed GC for capability handles](#10-distributed-gc-for-capability-handles)
11. [Promise pipelining vs submit-and-poll](#11-promise-pipelining-vs-submit-and-poll)

---

## 1. Identity custody

**Question.** Where do private signing keys live? What does a component see?

### Option A — Kernel-only Ed25519 with opaque-handle access (Willow today, PR #636 commitment)

- **Mechanism.** Kernel owns one or more Ed25519 keypairs. Components describe events; the kernel signs. State-`apply` never sees private bytes. Behavior components and `state-propose` reach signing through `host.identity` (capability-gated). Public keys exposed as opaque handles.
- **Pros.** Maps directly onto WIT `resource` ownership. `ZeroizeOnDrop`, atomic file write, `0o600` permission discipline already shipping in `willow-identity`. Keys never enter linear memory in raw form, so a malicious component cannot exfiltrate via OOB-read or panic-string. Fits Wasmtime's no-`WeakRef` posture: no nondeterministic finalization timing.
- **Cons.** Adds a kernel round-trip per signature. Forces the kernel to own key-rotation lifecycle (when do device keys rotate? per app or per kernel?). All identity domains funnel through one custodian.
- **Sources.** `willow/identity.md` lines 21-46, 109-167; `willow/runtime-vision.md` lines 137-158; PR #636 §"Crypto and key custody" lines 266-310.
- **Precedent.** Willow's `willow-identity` crate. Wasmtime resource-handle discipline. Similar to OS-keystore custodianship (macOS Keychain, Linux kwallet).

### Option B — Per-app keypair, kernel-mediated

- **Mechanism.** Each app receives its own Ed25519 keypair on install; events authored by that app are signed under it. Kernel still custodies the bytes; the *namespace* of signing identities is app-scoped.
- **Pros.** Compromise of one app's signing chain does not implicate the user's primary identity. Behavior identity becomes "app-scoped key" naturally — the kernel doesn't need a separate concept.
- **Cons.** Cross-app authority composition becomes hairy (PR #636 already flags this as deferred). Multiple identities-per-user means downstream apps need an "identity-of-record" resolver. The "user is X" claim becomes ambiguous.
- **Sources.** Holochain's per-cell `(DNA hash, agent pubkey)` (`prior-art/holochain/`); Spritely's per-vat key (`spritely-ocapn/captp-and-ocapn.md` §`op:start-session`).
- **Precedent.** Holochain cells (one identity per integrity-zome instance).

### Option C — Capability-handle-based identity (Spritely sturdyref / Agoric vat-ref shape)

- **Mechanism.** Identity is *implicit in possession of a reference*. There is no global "who is signing" predicate; authority follows from holding a capability handle. Signing happens at the cap-network boundary (handoff certificates) when an opaque handle crosses a session.
- **Pros.** No ambient identity inside a vat — full ocap discipline. Composition is reference-passing. Sealers + `Far()` give attenuation for free.
- **Cons.** Foreign to event-sourced DAG semantics — "who authored this event" needs an answer for replay, and "the holder of capability X" is not portable across peers without persisting the cap-graph itself. Distributed GC of that graph is a hard problem (Agoric SwingSet comms-vat phasing problem).
- **Sources.** `spritely-ocapn/capabilities.md` "no ambient authority" thesis; `agoric-endo/capabilities.md` `Far()` + pass-style discipline; `agoric-endo/captp-and-network.md` comms vat c-list.
- **Precedent.** Spritely Goblins; Agoric SwingSet vats.

### Willow position

Locked: **Option A (kernel-only Ed25519, opaque handles)**. PR #636 §"Crypto and key custody" line 50 makes this non-negotiable: "Private signing keys live only in the kernel. No component sees them." Already code in `willow-identity`. Myrhiza inherits.

### Re-evaluation question

Should Myrhiza accept that the **author identity per event is always the user's primary identity**, even for behavior-authored events, or is the per-(peer, behavior-instance) keypair (PR #636 lines 515-529) a separate identity domain visible at the DAG layer? Today PR #636 says behavior events sign under the behavior identity, *not* the user's. That makes the kernel manage at least two identity domains. Is there a third (multi-device)? Fourth (per-app)?

### Open questions

- Does the kernel expose a single `host.sign(payload-bytes) -> sig` import, or only typed `host.author-event(...)`? The latter prevents apps from misusing the signer for non-event payloads.
- Identity persistence: is the `0o600`-on-disk file the canonical custody, or do we want OS-keystore integration as the v1 default? Browser path forces non-extractable WebCrypto.

---

## 2. Multi-device + behavior identity unification

**Question.** PR #636 names per-device signing keys and per-(peer, behavior-instance) signing keys as **structurally the same problem**. Is this one kernel mechanism, two, or three (also counting MLS leaf-key-vs-credential)?

### Option A — One unified mechanism: long-term identity + short-lived signing key

- **Mechanism.** The kernel owns a single primitive: `(long-term identity, ephemeral signing keypair scoped to (peer, instance))`. Multi-device, behavior identity, and (when adopted) MLS LeafNode signing keys all use this primitive. The "instance" axis differentiates: instance = `device:N` for multi-device, instance = `bot:foo` for behavior, instance = `mls-group:bar` for MLS leaf.
- **Pros.** PR #636 explicitly recommends this: "structurally the same problem … should share a kernel-level mechanism rather than be invented twice." Single capability surface to design, audit, and rotate. MLS LeafNode signature_key fits the same shape — long-term credential + epoch-rotated leaf signing key.
- **Cons.** Conflates three distinct trust domains. Multi-device key advertisement needs to be in some app's authority graph; behavior key advertisement needs the same; MLS key advertisement needs to be inside MLS state. Sharing a mechanism means sharing a single bug class.
- **Sources.** `willow/identity.md` lines 109-140; PR #636 lines 515-529; `mls/group-lifecycle.md` §1 KeyPackage / §4 Update.
- **Precedent.** Sesame (Signal multi-device, cited in Willow seal-gift-wrap deferral spec line 124). MLS LeafNode credential vs signature_key split.

### Option B — Two mechanisms: device identity at kernel, behavior at app

- **Mechanism.** Kernel owns multi-device identity (long-term + per-device); behavior keypairs are minted by an app's `state-apply` as opaque handles, and the kernel only signs *events the app's pre-check authorizes*. The "behavior identity" is app-defined.
- **Pros.** Cleanly separates concerns. Behavior identity becomes an authority pattern in the app's permission model (per Domain 5).
- **Cons.** Behavior key custody still belongs in the kernel (Willow CLAUDE.md and PR #636 are emphatic that components don't touch private bytes). So this collapses to "Option A but with app-defined naming of the instance axis."

### Option C — Three mechanisms: multi-device, behavior, MLS-leaf separate

- **Mechanism.** Each domain has its own kernel surface. Avoids conflation; each can rotate independently.
- **Pros.** Different threat models get different policies. MLS LeafNode key rotation already lives inside MLS protocol; gluing it to multi-device rotation may force rotations the protocol doesn't need.
- **Cons.** Triples the surface to audit. Goes against PR #636's explicit guidance ("rather than be invented twice").

### Willow position

Not yet implemented. PR #636 names the unification as the *target*, not yet shipped. Willow seal-gift-wrap deferral spec (`/mnt/storage/projects/willow/docs/specs/2026-04-24-seal-gift-wrap-dms.md` lines 116-126) makes multi-device "non-negotiable" for the future MLS-over-Willow spec.

### Re-evaluation question

Adopt Option A and design the kernel primitive as `IdentityScope { long_term, instance: (peer, kind, name) }`, with `kind ∈ {Device, BehaviorInstance, MlsLeaf}`? This commits to one mechanism but admits three trust-domain readings via `kind`.

### Open questions

- Where is the **long-term identity → ephemeral signing-key binding** advertised? In-band (event in some DAG announcing the key) or out-of-band (kernel-known)? Willow's bech32-identifier scheme hints at in-band (`wpeer` is the long-term ID).
- Does the binding event itself need to be signed by the long-term key only? If yes, the long-term key must be live at advertisement time — which fights the "long-term identity stays in cold storage" pattern Sesame supports.

---

## 3. Capability discipline (per-call vs per-import vs full ocap)

**Question.** When does the kernel check authority for a host call?

### Option A — Per-import-binding (instantiate-time only) (default WIT)

- **Mechanism.** A component declares `import host.http` in its WIT world. At instantiate time, the kernel resolves the import to a function pointer with a closure carrying the component's manifest. Inside the component, `host.http(req)` is "ambient authority" within the declared scope.
- **Pros.** Fast: zero per-call kernel checks. Maps onto Component Model semantics directly. Manifest is the static declaration of authority.
- **Cons.** No per-call attenuation. A single `host.write-file` import means the component can write any file the manifest allows — no narrowing per-call.
- **Sources.** WASM Component Model resource semantics (`agoric-endo/capabilities.md` "Component Model handles"); WIT default semantics.

### Option B — Per-call gating with calling-component's manifest (PR #636 explicit)

- **Mechanism.** Each host call is checked against the *calling component's* manifest. PR #636 names specific cases: clipboard writes, file pickers, push registration, all gated per-call by the calling component, not the UI app's broad surface. Manifest-declared imports + per-call check.
- **Pros.** Lets a high-authority UI app (clipboard, file system, push) host a low-authority interaction component without leaking its own imports. The interaction component sees only what its manifest declared, regardless of who hosts it. Caller-attribution defense-in-depth.
- **Cons.** Per-call kernel work. Requires the kernel to track calling-component identity through the call stack — non-trivial when interaction components compose.
- **Sources.** PR #636 lines 366-385 (manifest-declared) + the per-call gating examples (clipboard, file picker, push); `willow/runtime-vision.md` §"Inter-component composition".

### Option C — Full ocap discipline ("no ambient authority", every reference is a capability)

- **Mechanism.** Spritely-style: a component sees no ambient host imports. To do anything, it must hold a reference to an actor that can do the thing. References are passed in (root-presence pattern), composed via attenuation, and revoked by dropping. The kernel manifest only declares the *root capabilities* the component starts with.
- **Pros.** Strictly stronger composition. Attenuation is native — wrap the file-actor with a "writes-only-into-this-directory" proxy. Revocation by drop. Sealers for nominal typing.
- **Cons.** Foreign to WIT's static-import model. WASM Component Model `resource` types get part of the way (handles are non-forgeable, scope-bounded by ownership), but full ocap requires reference passing as the *only* authority mechanism — no static imports at all. Steeper authoring discipline.
- **Sources.** `spritely-ocapn/capabilities.md` "if you don't have it, you can't use it"; `agoric-endo/capabilities.md` `Far()` + `E()` + pass-style.
- **Precedent.** Spritely Goblins; Agoric Endo / SES.

### Willow position

PR #636 commits to **Option B** (per-call gating with calling-component's manifest) for specific high-authority operations, and Option A (manifest-declared imports = ambient within scope) for everything else. The mix is described in PR #636 lines 366-385.

### Re-evaluation question

Should Myrhiza adopt **full ocap discipline** (Option C) for the cross-component composition layer (where two components on the same peer call each other) while keeping Option B for the host surface? This matches `agoric-endo/capabilities.md` §"Three reference styles" mapped onto Component Model: data is copied, resource handles are presences, futures are remote promises. Cross-component handles are non-forgeable; cross-peer handles need a CapTP-equivalent (Domain 9).

### Open questions

- Per-call gating cost: at what call rate does the per-call check matter? PR #636 cites clipboard, file picker, push — all rare. Hot-path imports (`host.subscribe` event delivery) likely stay Option A.
- Does the kernel need to track *which UI app* hosts an interaction component for attribution? Or is this a single-tenant invariant (only one UI app per peer)?
- Component-Model resource handles already give us non-forgeable cross-component refs. Do we need *more* than that?

---

## 4. Crypto host imports — shapes, returns, profile gating

PR #636 commits placement: encryption is a kernel capability bound to opaque key handles. Specific WIT signatures are deferred. This is the brainstorming session's job.

### The locked-in slice (PR #636 §"Crypto and key custody")

| Import | Profile | Purpose | Determinism |
|---|---|---|---|
| `host.seal(handle, plaintext) -> ciphertext` | state-`propose`, behavior | Encrypt under named key | Loose (originator-only) |
| `host.open(handle, ciphertext) -> plaintext` | interaction | Decrypt for display | Non-deterministic OK |
| `host.verify-payload-mac(envelope, key-handle) -> bool` | state-`apply` | Prove key-possession (not author identity) | **Must be deterministic** |
| `host.install-key(handle, sealed-distribution-blob) -> ()` | state-`apply` | Record key binding under app namespace | **Must be deterministic; returns `()` deliberately** |
| `host.can-open(handle) -> bool` | interaction (only) | Per-peer "can this peer decrypt?" query | Non-deterministic OK |

The `install-key` returning `()` is **not optional.** A `bool` return would make `state-apply` peer-locally branch on whether the local X25519 key can unwrap the blob — destroying determinism. The kernel records the (handle, blob) pair under the app's namespace on every peer regardless of whether that peer can actually unwrap it. Whether a peer can use the key is queried separately on the interaction side via `host.can-open`. (Willow `crypto.md` lines 138-167; PR #636 lines 220-232.)

### MLS-specific imports — design space for brainstorming

If MLS is adopted (Domain 6), the kernel exposes a typed `host.mls.*` capability. Candidate shapes:

- **Coarse: per-message** — `host.mls.process-message(group-handle, mls-message-bytes) -> Outcome`. App emits `MLSMessage` framing as event payload bytes; kernel-side MLS engine processes. Single import; minimal WIT surface.
- **Medium: per-message-type** — Distinct imports for `welcome`, `commit`, `proposal`, `application`. Each typed against the MLS WIT. App constructs typed events.
- **Fine: per-operation** — `host.mls.add-member(group, key-package)`, `host.mls.update-leaf(group)`, `host.mls.commit(group)`. App orchestrates protocol moves; kernel arbitrates.

| Granularity | Pro | Con |
|---|---|---|
| Coarse | Smallest WIT surface; MLS engine internal | App can't validate semantics in pre-check; pre-check sees opaque bytes |
| Medium | Pre-check can validate by message type; matches MLS framing layer | Forces app to know `WireFormat` selectors |
| Fine | Pre-check sees full operation intent | Largest WIT surface; tightest coupling to RFC 9420 mechanics |

### MLS Welcome / Commit envelope formats `host.verify-payload-mac` must accept

PR #636 defers to crypto child spec. Brainstorming candidates:

- **`MLSMessage` raw (RFC 9420 wire format)** — simplest; `host.verify-payload-mac` parses TLS-codec internally.
- **Wrapped envelope** — `{ payload_kind: enum, mls_bytes: vec<u8>, willow_envelope_sig: Ed25519Sig }`. Adds outer Ed25519 sig binding to author; kernel verifies both.
- **NIP-44 v2 ciphertext at the framing layer** — Willow seal-gift-wrap deferral spec §"NIP-44 v2 payload format is reusable" recommends keeping NIP-44 verbatim as the AEAD framing on top of MLS application payloads. Verify with KAT vectors.

### Sources

- `willow/crypto.md` lines 138-167 (placement commitments).
- `willow/runtime-vision.md` lines 137-158.
- `mls/protocol.md` §8-9 (wire format).
- `mls/group-lifecycle.md` §1, §6, §7 (KeyPackage, Commit, Welcome shapes).
- Willow seal-gift-wrap deferral spec lines 100-114 (NIP-44 v2).

### Open questions

- Does `host.verify-payload-mac` accept *any* MAC type (HMAC-SHA256 + ChaCha20-Poly1305 + AES-GCM-128 + ML-DSA stubs) or pin to a single AEAD? PQ migration says ciphersuite agility.
- Is `host.seal` keyed by AEAD nonce policy (random vs ratchet-derived) or does the kernel always pick? The Willow ratchet (epoch-key-rotation spec) uses HKDF-derived per-message keys with `MAX_RATCHET_LOOKAHEAD = 1024` DoS bound — the kernel should own this, not the app.
- "Author-identity vs key-possession" distinction: PR #636 makes `host.verify-payload-mac` prove possession only. Author identity comes from the outer Ed25519. Brainstorming should confirm: are there cases where the app wants `verify-payload-mac` to also bind author? If yes, return type widens.

---

## 5. Permission model — app-defined vs kernel tiers

**Question.** Do apps express their own authority graphs (PR #636) or does the kernel ship built-in tiers (Willow today)?

### Option A — Per-app authority via state-apply verdict (PR #636 commitment)

- **Mechanism.** Each app exports a single `check_permission(state, author, event_kind) -> Permission?` function. The kernel calls it both before signing (pre-check) and during replay (apply). The kernel knows nothing about `Admin`, `SyncProvider`, `SendMessages` — those are app-internal.
- **Pros.** Apps with no governance pay for nothing. App authors define their own permission shapes. Compositionally clean: the authority predicate is *one* WASM export.
- **Cons.** Each app re-derives common patterns (votes, owner-roots, role-based access). No kernel-level help. Cross-app authority composition (PR #636 §Open Questions) is unsolved.
- **Sources.** `willow/authority.md` lines 126-191 ("What changes under PR #636", "What Myrhiza inherits"); `willow/runtime-vision.md` §"Pre-check equals apply".
- **Precedent.** Willow's `check_permission` + `required_permission()` collapsed into one export. Holochain's per-zome validation callbacks (sibling shape).

### Option B — Kernel tiers (Willow today, chat-tuned)

- **Mechanism.** Kernel provides a fixed permission enum (`Owner`, `Admin`, `SyncProvider`, `ManageChannels`, `ManageRoles`, `SendMessages`, `CreateInvite`); apps consume it. State changes happen through structured `Propose`/`Vote`/`GrantPermission` events.
- **Pros.** Apps share a vocabulary. Cross-app composition has a fixed lingua franca.
- **Cons.** Forces every app into the chat shape. A pure CRDT note app, a kanban board, a 3D voice room — all pay for chat-shaped tiers. Lifts the wrong abstraction: Willow's tiers are chat-tuned, not P2P-runtime-tuned.

### Option C — Hybrid: app-defined predicate, kernel-supplied common helpers

- **Mechanism.** The kernel exposes a single authority-predicate import (Option A) + opt-in helper components (vote-threshold, RBAC, ACL). Apps that want governance pull the helper component; apps that don't, don't.
- **Pros.** Re-use without forcing. Apps compose helpers via the same component-model machinery as everything else.
- **Cons.** Helpers are app-version-coupled — schema migration risks. Each helper is a small auth surface that might leak the wrong way under composition.

### Willow position

Willow today: Option B (chat tiers, owner-rooted-with-governance). PR #636: Option A (apps own their authority). Myrhiza inherits A.

### Re-evaluation question

Should Myrhiza standardize a small set of authority-predicate **patterns** (in `references/authority-patterns.md` or similar) — owner-rooted, flat-membership, ocap-delegated — and let apps mix them? This is Option C-lite: no shared helper components, just shared design vocabulary.

### Cross-app authority composition

Open per PR #636. The brainstorming list:

- **Pure app-internal** (today's commitment): no cross-app composition; each app's authority is its own.
- **Shared identity, separate authority** (likely v2): all apps share user identity; each app gates its own events. UI app coordinates "I'm logged in as Alice across all apps."
- **Capability-handoff** (Spritely shape, future): app A gives app B a sturdyref; B's authority includes "things sturdyrefs from A authorize." Requires Domain 9 cross-peer capability protocol.

### Sources

- `willow/authority.md` lines 1-216 (full file).
- `willow/runtime-vision.md` lines 99-119 ("Pre-check equals apply").
- PR #636 §"What stays the same about Willow" lines 367-385.
- `willow/identity.md` §"Trust model".

### Open questions

- Is `SyncProvider`-equivalent (relay trust grant) an app concern or a kernel-level capability granted at the topic-membership layer? PR #636's relay-is-dumb stance suggests app-level.
- Do we need a kernel-level "critical app" flag (Agoric's `criticalVatKey`)? `agoric-endo/vat-model.md` §"Critical vats" — kernel panics if a critical vat dies. Useful for bootstrap apps; probably not v1.

---

## 6. MLS adoption shape

**Question.** When does Myrhiza adopt MLS? At v1 (kernel imports), v2 (post-MVP), or never?

### Option A — Adopt at v1: `host.mls` is in the kernel from day one

- **Mechanism.** Kernel ships an MLS engine (OpenMLS or mls-rs) behind a typed WIT capability. `host.mls.*` imports for group operations. Group state lives kernel-side; apps reference via opaque handles.
- **Pros.** First-class group-shaped capabilities from MVP. Avoids the "we'll add MLS later" trap that bit Matrix (Megolm = 7 years of UTD bugs). Cremers ETK 2025 says MLS *needs* SUF-CMA signatures — Ed25519 is SUF-CMA; ECDSA is not. Willow already uses Ed25519, so the constraint is satisfied.
- **Cons.** Increases kernel surface. OpenMLS ships with `rayon` parallelism — non-determinism risk if MLS operations enter `state-apply`. Need to disable rayon or route MLS outside `state-apply` (propose-only).
- **Sources.** `mls/lessons.md` "Recommended posture for the runtime spec"; `mls/critiques.md` §2 Cremers ETK 2025; `mls/protocol.md`; Willow seal-gift-wrap deferral spec lines 22-49.

### Option B — Adopt at v2: ship without group caps, add MLS later

- **Mechanism.** MVP ships pairwise channels only (or no encryption beyond `willow-crypto`'s ChaCha20-Poly1305 + epoch ratchet). Group-shaped state uses unencrypted DAG + per-channel symmetric keys (Willow's current approach + the `RotateChannelKeyV2` rotation pattern from `2026-04-24-epoch-key-rotation.md`).
- **Pros.** Smaller MVP. Delays MLS engine integration cost.
- **Cons.** Apps that need groups (chat, kanban with shared edit history) ship without FS/PCS. Rip-and-replace later, with all the migration pain Wire just absorbed (`mls/critiques.md` §3 Wire deployment-experience report).

### Option C — Never: stay with Willow-shape symmetric channel keys + epoch rotation

- **Mechanism.** Continue with `RotateChannelKeyV2` event-driven rotation. Group state encrypted under a single channel key per epoch; rotation triggered by membership change. PCS via rotation, no FS for in-flight messages, no post-quantum confidentiality.
- **Pros.** Already shipping in Willow (`willow/crypto.md` lines 56-91). Simple, audited primitives.
- **Cons.** Threat model gap: not-FS, not-PCS at message granularity, just at epoch granularity. Welcome-equivalent (key distribution to new joiners) is per-recipient X25519 wrap, O(N) per join — not O(log N).

### MLS application messages should NOT flow through DAG

Willow seal-gift-wrap deferral spec lines 90-101 captured this lesson. The future MLS-over-Willow spec must put MLS application messages on a *separate* transport path (inbox topic + worker-bounded retention, or fetch-on-demand store). Reasons: per-author DAG pollution, lack of natural retention signal, MLS's own application/handshake split (`mls/protocol.md` §9). MLS handshake (Welcome/Commit/Proposal) lives in DAG; MLS application messages do not.

### Cremers ETK 2025 constraint (must absorb)

Cremers/Günther/Wallez/Zhao 2025 (`mls/critiques.md` §2): MLS does not realize FCGKA security with EUF-CMA-only signatures. ECDSA is EUF-CMA but not SUF-CMA. **Use Ed25519 (which is SUF-CMA) or ML-DSA (PQ).** Willow already uses Ed25519, so this is fine for v1 — but document the constraint in the master spec so future ciphersuite selection cannot regress.

### Willow position

Deferred but committed. Willow seal-gift-wrap deferral spec is unambiguous: "specify **MLS-over-Willow (RFC 9420)** in a follow-up." Multi-device "non-negotiable." MLS app messages "NOT in per-author DAG." Library candidate: OpenMLS.

### Re-evaluation question

Adopt MLS at v1 (Option A) but **scope it to a single in-tree app** (chat, the candidate first app on Myrhiza)? The kernel `host.mls` capability ships in v1; only the chat app uses it. Other apps adopt later. Avoids "MLS for everything" overreach while validating the kernel surface.

### Open questions

- WIT contract granularity for MLS (Domain 4 above): coarse, medium, or fine?
- OpenMLS vs mls-rs vs mlspp? `mls/lessons.md` recommendation matrix favors OpenMLS for Myrhiza (formal-verification adjacency via libcrux, sync-only API matches `state-apply` purity, MIT license fits).
- How does `state-apply` purity interact with MLS engine state? MLS is itself deterministic in spec, but OpenMLS implementation has non-determinism risks (rayon, RNG). Need feature flags or routing decision.
- Authentication Service (AS) vs Delivery Service (DS) split (`mls/protocol.md` §6): pubkey-as-identity (simplest) vs DID-based vs application-defined?

---

## 7. Post-quantum migration path

**Question.** How does Myrhiza migrate from classical Ed25519 / X25519 / Ed25519-MLS to post-quantum (ML-KEM, ML-DSA) ciphersuites?

### Option A — Ciphersuite agility from day one, migrate via MLS Reinit

- **Mechanism.** WIT contracts for crypto imports parameterize over ciphersuite ID (RFC 9420 `0x0001`–`0x0007` initially; PQ suites `MLS_128_MLKEM768X25519_AES128GCM_SHA256_Ed25519` etc. when ratified). When all members of a group advertise the new suite (LeafNode `capabilities` field), a current member proposes `ReInit` naming the new ciphersuite. Old group terminates; new group bootstraps at epoch 0 with same membership, tied via resumption PSK.
- **Pros.** RFC 9420 anticipates this exactly. The PQ migration is mechanical, not a protocol break. `mls/crypto.md` §7-8.
- **Cons.** Hardcodes ciphersuite-ID switch into every host import. If Myrhiza later wants a non-MLS-shaped crypto (e.g. `nutshell`-style proof system), the ciphersuite axis is wrong.
- **Sources.** `mls/crypto.md` §7-8 ("Quantum-safe migration story"); `mls/lessons.md` §"Plan the post-quantum migration path".

### Option B — Hybrid combiner (`draft-ietf-mls-combiner`)

- **Mechanism.** Layer a PQ key agreement *on top* of an existing classical MLS group, rather than replacing the ciphersuite outright. Targets Dec 2026 IETF milestone.
- **Pros.** No Reinit; the existing group continues. Belt-and-suspenders.
- **Cons.** Draft, not ratified. Two key schedules to manage. Implementation cost.

### Option C — Defer the problem: pin classical ciphersuites for v1, plan migration pre-2030

- **Mechanism.** v1 ships with Ed25519 + X25519 + ChaCha20-Poly1305 (`MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519`, ID `0x0003`). PQ migration tracked as a v3 spec.
- **Pros.** Smallest v1 surface.
- **Cons.** "Harvest now, decrypt later" attacks already underway. PCS is meaningless against a future quantum adversary if v1 traffic is harvested.

### Willow position

Not yet specified. Willow `crypto.md` doesn't surface PQ. Willow seal-gift-wrap deferral spec doesn't mention PQ. Myrhiza must own this.

### Re-evaluation question

Adopt **Option A** but commit to ciphersuite-ID-as-handle from day one — `host.seal(handle, plaintext)` where `handle` carries the ciphersuite ID. This makes the WIT contract PQ-ready without committing to specific suites in v1.

### Open questions

- ML-DSA-65 vs ML-DSA-87 for PQ signatures? The MLS PQ draft registers both. `mls/crypto.md` §7.
- Hybrid (PQ KEM + classical signature) vs pure PQ? Hybrid is the conservative migration step; pure PQ requires ML-DSA which is larger.
- Does Myrhiza commit to following the IETF MLS PQ ciphersuites draft (target Dec 2026) or pick a different PQ track (e.g. application-layer Hybrid Public Key Encryption draft variants)?

---

## 8. Multi-device recovery / device add+revoke

**Question.** Out of scope for v1 in PR #636, but "must be reasoned about" (Willow seal-gift-wrap deferral spec calls multi-device "non-negotiable").

### Option A — Sesame-class: long-term identity, per-device session keys, in-band advertisement

- **Mechanism.** Long-term identity key (cold storage / hardware-backed) signs `DeviceAdd { device-pubkey, capabilities }` events. Other peers see the event, accept the device key as authorized for the user. Device revoke: `DeviceRevoke { device-pubkey }` event signed by long-term key (or by quorum of remaining devices).
- **Pros.** Standard pattern (Signal, Matrix). MLS-LeafNode-ready (each device has its own LeafNode in user's MLS groups).
- **Cons.** Long-term key must be live at advertise time. Hardware-backed cold storage is a UX cliff; "I lost my phone and can't get into my laptop because the long-term key is on the phone" is the reality.

### Option B — Quorum / threshold-signature-based recovery

- **Mechanism.** k-of-n device threshold for adds and revokes. Lose one device, others vote. No single long-term key with absolute authority.
- **Pros.** No single point of failure.
- **Cons.** Bootstrap problem (what's the first device?). Forces on-app authority semantics into kernel. Probably out of scope for v1.

### Option C — Defer to v2; v1 = single-device only

- **Mechanism.** Each Myrhiza user is one device. To "switch devices," migrate the kernel state file. Multi-device added in v2.
- **Pros.** Smallest v1 scope. Willow today is here.
- **Cons.** Doesn't match the "non-negotiable" framing in seal-gift-wrap deferral. New users on phone + laptop expect both to work. UX deal-breaker for serious adoption.

### Willow position

Currently Option C (single device per identity file). Future: Option A is implicit (Sesame-class is what the Willow seal-gift-wrap deferral spec names). PR #636 names device-add/revoke as out of v1 scope but unifies device identity with behavior identity (Domain 2).

### Re-evaluation question

If Myrhiza commits to Option A but ships **only the in-band advertisement event in v1** (no UX, no recovery flow), is that enough to claim multi-device-ready? The on-the-wire shape locks in; the UX comes later.

### Open questions

- Is the `DeviceAdd` event in the user's per-author DAG, or in a separate device-management topic? The former couples user-identity DAG to device list; the latter is cleaner but requires a new topic shape.
- Recovery: does Myrhiza ship social recovery, hardware-backed escrow, both, neither?
- "Stolen device" threat model: a thief with the device-key can sign for some interval before revocation propagates. What's the propagation latency commitment?

---

## 9. Cross-peer capability protocol — CapTP/OCapN vs re-invent

**Question.** When components on different peers reference each other (e.g. interaction-component on peer A invokes a behavior-component on peer B), what protocol carries the reference?

### Option A — Adopt OCapN / CapTP

- **Mechanism.** Implement the OCapN wire protocol (`spritely-ocapn/captp-and-ocapn.md`): `op:start-session`, `op:deliver`, `op:listen`, `op:gc-exports`, `op:gc-answers`. Imports/exports/questions/answers four-table model. Sturdyrefs for persistent caps; swiss-numbers for unguessability. Netlayer abstraction for transport pluggability.
- **Pros.** Standard-track (OCapN is the de facto cross-implementation effort; Spritely + Agoric co-author). Promise pipelining first-class. Distributed acyclic GC with documented limits. Sturdyref-as-URI is portable. Already implemented in Guile Goblins, Racket Goblins, Haskell.
- **Cons.** Pre-1.0 since 2022 (`spritely-ocapn/lessons.md` §"Avoid"). Multi-year drift risk. Tracking an unstable spec into Myrhiza v1 is a commitment to a moving target. No Rust reference impl.
- **Sources.** `spritely-ocapn/captp-and-ocapn.md` (full file); `agoric-endo/captp-and-network.md` "OCapN — the cross-implementation effort"; `spritely-ocapn/lessons.md` "Borrow" §1-4.

### Option B — Re-invent over iroh + gossipsub

- **Mechanism.** Build a Myrhiza-native cross-peer reference protocol on top of iroh transport + Willow's gossipsub topics. References scoped to (peer, topic, opaque-handle). Lifecycle managed via topic membership.
- **Pros.** No external-spec dependency. Aligns with Willow's existing transport.
- **Cons.** Re-derivation of distributed GC, pipelining, sturdyref-equivalent. Agoric SwingSet "informed vs ignorant" comms-vat phasing problem suggests this is real engineering, not a weekend's work (`agoric-endo/captp-and-network.md` "Honest unflattering bits"). Cap'n Proto + Spritely + Agoric all converged on the same shape — re-inventing means re-discovering.

### Option C — Hybrid: OCapN-shaped wire, Myrhiza-native transport (iroh netlayer)

- **Mechanism.** Adopt the OCapN four-table model and message types as the cap protocol. Implement an iroh-based netlayer alongside Tor / TCP+TLS / libp2p. Sturdyrefs use iroh-EndpointId as the designator.
- **Pros.** Best of both: standardized semantics, native transport. iroh becomes one of N netlayers. Path forward to interop with Spritely/Agoric.
- **Cons.** Still tracking pre-1.0 OCapN. Wire-format churn risk.

### Willow position

Not yet specified. Willow today has no cross-peer capability protocol; cross-peer *events* flow over gossipsub, but there's no notion of "send a cap to a peer." PR #636 leaves this open.

### Re-evaluation question

Does Myrhiza need cross-peer capabilities at all in v1, or is "events over DAG topics" sufficient? If apps don't compose across peers (each app is single-tenant per peer, with cross-peer state handled by the DAG), Option B/C might be deferred to when a use case forces it.

### Open questions

- If Option A or C: which OCapN version do we lock to? Spec is at draft, has churned.
- If Option B: do we re-derive promise pipelining, or just submit-and-poll across peers?
- Does the `host.broadcast` import already provide enough cross-peer composition that we don't need cap refs? Probably yes for state-apply use cases; probably no for interaction-component-to-behavior-component direct invocation.

---

## 10. Distributed GC for capability handles

**Question.** When components hold capability handles to objects on other peers, what releases them?

### Option A — Acyclic distributed refcount (Spritely + Agoric shape)

- **Mechanism.** Each peer tracks reachable / recognizable counts for each export. Drop messages (`op:gc-exports` in OCapN; `syscall.dropImports` in SwingSet) flow when local refcount hits zero. Cycles spanning peers leak; documented limit.
- **Pros.** Working in Spritely Goblins (acyclic) and Agoric SwingSet (with comms-vat phasing). Honest scope reduction.
- **Cons.** Cycles leak. The "informed vs. ignorant" message phasing in SwingSet's comms layer is real engineering (`agoric-endo/captp-and-network.md` line 232).

### Option B — Per-app problem: kernel doesn't custody cross-peer cap-graph

- **Mechanism.** Apps using cross-peer caps own their refcounting. Kernel provides handle-creation primitives; lifecycle is app's job.
- **Pros.** No kernel complexity. Apps that don't use cross-peer caps pay nothing.
- **Cons.** Every app re-derives the same hard problem. Bug class will repeat.

### Option C — Punt entirely: no cross-peer cap GC in v1

- **Mechanism.** Cross-peer caps are sturdyrefs (persistent, manually revoked). Live refs are session-scoped; die with the session. No automatic GC of either.
- **Pros.** Smallest scope.
- **Cons.** Applications that need live caps with auto-cleanup must build it themselves.

### Willow position

Not addressed. Willow has no cross-peer cap concept.

### Re-evaluation question

If Myrhiza adopts OCapN (Domain 9 Option A/C), Option A here is implied. If it doesn't, Option C is fine for v1 — apps that need it build it.

### Open questions

- Is `WeakRef` / `FinalizationRegistry` denied to all components (per Agoric's determinism rule) or just `state-apply`? Determinism only requires denial in `state-apply`, but cross-peer GC machinery often relies on weak references in the kernel.

---

## 11. Promise pipelining vs submit-and-poll

**Question.** PR #636's submit-and-poll pattern (component calls `host.foo(args) -> request-token`, kernel later calls `on-completion(token, result)`) is the v1 ABI. Does this preclude promise pipelining?

### Option A — Submit-and-poll only (PR #636)

- **Mechanism.** Sync host imports return tokens; async completion is delivered via re-entry. No pipelining; chained calls are RTT-bound.
- **Pros.** Browser-jco-compatible (no async on browser side). Simpler v1 ABI.
- **Cons.** Cross-peer chained calls (`E(server).getA().getB().compute(42)`) are 3 RTTs, not 1. CapTP-style apps would underperform.
- **Sources.** `willow/runtime-vision.md` §"Submit-and-poll for async"; PR #636 line 195+.

### Option B — Promise pipelining at the cross-peer boundary (CapTP)

- **Mechanism.** Cross-peer cap calls allocate `answer-pos` slots; subsequent calls can target unresolved-promise-of-call-N before resolution. Pipelined batch ships in one packet. `op:deliver` with `desc:answer N` pattern.
- **Pros.** Kills RTT-bound chains. Production-validated (Cap'n Proto, CapTP).
- **Cons.** Adds a question/answer table to the kernel's per-peer state. Submit-and-poll within the peer + pipelining across peers is two ABIs.
- **Sources.** `spritely-ocapn/captp-and-ocapn.md` §"Promise pipelining at the wire level"; `agoric-endo/capabilities.md` §"Promise pipelining".

### Option C — Pipelining at the SDK macro layer; submit-and-poll at the kernel

- **Mechanism.** Kernel ABI is submit-and-poll (Option A). SDK macros for common patterns hide the token juggling and (eventually) the pipelining for cross-peer chains.
- **Pros.** Single kernel ABI. Pipelining is opt-in via SDK.
- **Cons.** SDK becomes load-bearing for performance — if it lags, apps suffer. Kernel can't enforce pipelining safety properties.

### Willow position

PR #636 commits to submit-and-poll for v1 (Option A), explicitly because of jco-async-limitations. Pipelining not addressed.

### Re-evaluation question

If Myrhiza adopts CapTP/OCapN for cross-peer caps (Domain 9), pipelining is implied. The question is whether the **kernel-to-component** ABI also supports something more than submit-and-poll once browser async stabilizes.

### Open questions

- `spritely-ocapn/lessons.md` §"Avoid": "Don't sell pipelining as a throughput win. Sell it as the latency primitive it is." Document accordingly.
- Does Myrhiza commit to a kernel-side answer-table size limit to prevent pipeline-bomb DoS?

---

## Cross-domain interactions

Decisions in these domains are not independent. Key couplings:

- **Domain 1 (kernel-only Ed25519) + Domain 2 (one unified IdentityScope) + Domain 6 (MLS at v1)** — if all three: the kernel's identity surface custodies (long-term, device-instance, MLS-leaf) keys via one primitive. Domain 4 `host.install-key` semantics carry over to MLS LeafNode publishing via in-band events.
- **Domain 3 (per-call gating) + Domain 4 (crypto host imports)** — `host.seal` / `host.open` profile gating is itself an instance of per-call discipline. The kernel checks "is the calling component a state-`propose` or behavior?" per-call.
- **Domain 5 (app-defined permissions) + Domain 6 (MLS)** — MLS membership ops (Add/Remove/Update) need authority. PR #636 sends authority to app-defined `check_permission`. The MLS Welcome/Commit events are *both* MLS protocol moves and authority events; the app's predicate decides who may emit them.
- **Domain 7 (PQ migration) + Domain 6 (MLS)** — MLS Reinit is the migration vehicle. PQ migration logic lives in MLS-layer ciphersuite advertisement, not in custom kernel code.
- **Domain 9 (CapTP/OCapN) + Domain 10 (distributed GC) + Domain 11 (pipelining)** — adopting OCapN brings all three together; rejecting OCapN forces re-deriving each.
- **Domain 8 (multi-device recovery) + Domain 6 (MLS)** — MLS handles per-device LeafNodes natively; multi-device add = MLS Add for new device. Multi-device revoke = MLS Remove + key rotation.
- **Domain 4 (`install-key` returns `()`) + Convergence (deterministic state-apply)** — non-negotiable. The bool-return alternative breaks bit-identical convergence across peers.

---

## Brainstorming question list

Concrete questions for the session:

**Identity custody.**

1. Confirm Option A (kernel-only Ed25519, opaque handles)? Any reason to revisit?
2. Single `host.author-event(...)` or general `host.sign(payload-bytes)`? The latter is more flexible but loses kernel-enforced "only sign events" invariant.

**Multi-device + behavior identity.**

3. One unified `IdentityScope { long_term, instance: (peer, kind, name) }` or three separate kernel mechanisms?
4. Is `kind` the source of truth for trust-domain separation, or do we need per-domain capability gating?

**Capability discipline.**

5. PR #636's per-call gating + manifest-scope (Option B) is locked. Do we extend to full ocap (Option C) for cross-component composition, while keeping Option B for host imports?
6. WIT resource handles already give us non-forgeable cross-component refs. Sealers as a kernel primitive — yes/no?

**Crypto host imports.**

7. WIT signature shapes for `host.seal` / `host.open` / `host.verify-payload-mac` / `host.install-key` / `host.can-open` — confirm or revise the PR #636 placement.
8. MLS imports: coarse / medium / fine granularity?
9. MLS Welcome/Commit envelope format candidates — vote among `MLSMessage` raw, wrapped envelope, NIP-44-v2-framed.
10. Does `host.verify-payload-mac` accept multiple AEADs, or pin to one?

**Permission model.**

11. Confirm app-defined authority predicate (Option A) for v1.
12. Cross-app authority composition: defer to v2 (status quo) or sketch a shape now?

**MLS adoption shape.**

13. Adopt at v1, v2, or never?
14. If v1: scope to in-tree chat app, or open the `host.mls` capability to all apps?
15. Library: OpenMLS, mls-rs, or hold off?
16. WIT contract granularity (echoes Q8).

**Post-quantum.**

17. Ciphersuite-ID-as-handle from day one (Option A) — confirm?
18. Track the IETF MLS PQ ciphersuites draft (Dec 2026 milestone) or pick a different track?

**Multi-device recovery.**

19. Sesame-class long-term + per-device, in-band advertised — confirm shape?
20. Recovery flow: social recovery, hardware-backed escrow, both, neither?
21. Defer to v2 entirely, or ship the in-band advertisement event (only) in v1?

**Cross-peer cap protocol.**

22. Adopt OCapN (Option A/C) or re-invent (Option B) or punt (no cross-peer caps in v1)?
23. If OCapN: track pre-1.0 spec or wait for 1.0?

**Distributed GC.**

24. Acyclic refcount (Spritely shape) or per-app or punt?

**Pipelining.**

25. Submit-and-poll only (Option A) or admit CapTP-style pipelining at the cross-peer boundary?

---

## Sources

- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/identity.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/crypto.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/authority.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/runtime-vision.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/spritely-ocapn/capabilities.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/spritely-ocapn/captp-and-ocapn.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/spritely-ocapn/persistence.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/spritely-ocapn/lessons.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/agoric-endo/capabilities.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/agoric-endo/captp-and-network.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/agoric-endo/vat-model.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/mls/protocol.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/mls/crypto.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/mls/group-lifecycle.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/mls/critiques.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/mls/lessons.md`
- `/mnt/storage/projects/myrhiza/docs/references/local-first.md`
- `/mnt/storage/projects/willow/docs/specs/2026-04-24-seal-gift-wrap-dms.md`
- `/mnt/storage/projects/willow/docs/specs/2026-04-24-epoch-key-rotation.md`
- `/mnt/storage/projects/myrhiza/CLAUDE.md`
- Willow PR #636 (`docs/specs/2026-04-27-willow-runtime/README.md`) — referenced via Myrhiza's `prior-art/willow/runtime-vision.md`
- Cremers, Günther, Wallez, Zhao — *ETK: External-Operations TreeKEM and the Security of MLS in RFC 9420* — IACR ePrint 2025/229
- RFC 9420 — *The Messaging Layer Security (MLS) Protocol*
- RFC 9750 — *The MLS Architecture*
- `draft-ietf-mls-pq-ciphersuites-04`
- `draft-ietf-mls-combiner`
- OCapN draft specifications — github.com/ocapn/ocapn
