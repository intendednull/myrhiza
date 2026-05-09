# Identity + crypto

## Per-app keypairs

Each app install generates a new ed25519 keypair; the public key is the agent's address in that app's DHT and signs every source chain action and every network message ([Agent-centric Digital Identity](https://medium.com/h-o-l-o/agent-centric-digital-identity-5314d507f0ab)).

This is simple and works, but it's strictly per-app: the same human user has different identities in different DNAs. Cross-app identity, multi-device coordination, and key rotation are not solved by this primitive.

## DeepKey / DPKI: the seven-year saga

[**DeepKey / DPKI**](https://github.com/holochain/deepkey) was the planned solution for cross-app identity, multi-device key management, and key rotation: a system happ where users register all their per-app keys under a single keyset with M-of-N change rules. DeepKey was repeatedly attempted and shipped in 0.4 behind a feature flag.

**It was removed in 0.6.** The 0.5→0.6 upgrade explicitly removes the entire `dpki` config block and the `device_seed_lair_tag` field ([upgrade-holochain-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)). After roughly seven years of effort, the maintainers concluded the DPKI design as built was wrong and pulled it.

**There is currently no canonical multi-device or key-rotation story.**

This is the most important cautionary tale in the Holochain corpus for any new P2P runtime: shipping identity is harder than shipping the rest of the runtime. Apps were built on DPKI and are now stranded.

## No built-in MLS / group key management

Group chats today (e.g. Relay) handle their own group key derivation per-app. There is no shared cryptographic primitive for forward-secret group messaging — each happ rolls its own.

This is a missed opportunity: every messaging-shaped app reinvents the same wheel, and most do it badly. A runtime-level MLS primitive (or equivalent) would have prevented years of duplicated effort.

## Warrants

[**Warrants**](https://developer.holochain.org/concepts/7_validation/) are the Sybil/malicious-author response: when a validation authority detects an invalid op, it publishes a signed warrant; warrants gossip to agent-activity authorities and then network-wide; receiving peers block the warranted author.

This works against integrity violators but not against general Sybil flooding — there's no proof-of-personhood or stake gate. A single attacker with infinite identities can still:

- Generate fresh keys faster than warrants can propagate
- Free-ride on the DHT without contributing storage (zero-arc participation)
- Replay or pre-validate to avoid emitting any "invalid op" event a warrant could attach to

Warrants are an important primitive (cryptographic proof of misbehavior, gossip-distributed) but they are not a Sybil-resistance answer. They're a "if we caught one, we can banish them" answer.

## Implications for Myrhiza

- **Decide identity scope before v1.** Per-app pubkey OR cross-app+multi-device identity, not "we'll figure it out." Half-shipped identity is worse than missing identity.
- **Group cryptography is a runtime opportunity.** MLS-as-a-host-import lets every app reuse one well-audited primitive. Holochain's per-app reinvention is a clear miss.
- **Warrants are worth borrowing for the misbehavior-response story.** Cleaner than reputation, harder to game. See [`lessons.md`](lessons.md).
- **Don't claim to solve Sybil via consensus-avoidance.** "Agent-centric" doesn't make Sybil go away; it relocates the problem. Be explicit that gating who can join a network is the app author's responsibility (membrane proofs are Holochain's pluggable answer to this; worth borrowing).

## Genesis sequence

When a cell is first installed, Holochain writes four "genesis records" to the source chain before any user code runs ([concepts/3_source_chain](https://developer.holochain.org/concepts/3_source_chain/)):

1. **`Dna`** action — anchors the source chain to a specific DNA hash. First action in every source chain; binds authoring history to one specific DNA bundle.
2. **`AgentValidationPkg`** action — carries the membrane proof the user supplied at install time. Inspectable by integrity zome's validation callback when other agents validate the new agent's chain.
3. **`Create`** action with the agent's `AgentPubKey` as the entry — on-chain registration of the agent's public key as a first-class entry, addressed by the pubkey itself ([genesis-self-check-callback](https://developer.holochain.org/build/genesis-self-check-callback)).
4. **`InitZomesComplete`** action — written after the conductor runs each coordinator zome's optional `init` callback. After this, the cell is "live" and the first user-authored action has predecessor `InitZomesComplete`.

In validation callbacks the rule is: the previous action of a `Create(AgentPubKey)` must be `AgentValidationPkg`, and the previous action of `AgentValidationPkg` must be `Dna`. This invariant lets validators detect chains that don't begin with the right preamble.

## Membrane proofs in detail

Membrane proofs are the per-DNA join gate. Arbitrary bytes (`SerializedBytes`) supplied at install time and stored inside the `AgentValidationPkg` action. Validated in two places:

- **`genesis_self_check`** runs locally before the cell announces itself to peers. Receives `GenesisSelfCheckData { membrane_proof, agent_key }` and runs in the integrity zome with **no DHT access**. Use restricted to format and signature checks, including against authorities specified in the DNA's properties. Exists to "guard against user entry error and help prevent them from being accidentally marked as a bad actor" ([Glossary](https://developer.holochain.org/resources/glossary/)).
- **`validate`** runs on every peer that receives a DHT op for the new agent's chain. This callback can do full DHT lookups (e.g. check whether the proof's signer is registered as an authority). If the proof fails here the agent is warranted and blocked.

Known race: "membrane proof checking is currently only enforced via normal validation, not during handshaking, so unauthorised agents are able to join a network and access it for a short time before being warranted and blocked" ([Dev Pulse 153](https://blog.holochain.org/dev-pulse-153-holochain-0-6-released-with-immune-system/)). Plus the chicken-and-egg problem: validating a proof may require DHT data only available after joining ([Dev Pulse 94](https://blog.holochain.org/dev-pulse-94-when-signup-goes-wrong/)).

## Per-app key custody (lair internals)

Agent private keys live in **lair** — out-of-process keystore exposing a libsodium-backed signing/encryption API to the conductor over IPC. Lair stores 32-byte seeds; each seed deterministically derives an Ed25519 signing keypair and an X25519 encryption keypair ([lair README](https://github.com/holochain/lair); [Part 1](https://blog.holochain.org/part-1--holochain--holo-accounts--and-cryptographic-key-management/)).

- **Independent vs. derived.** Default behavior in 0.6 is one fresh random seed per app install. Hierarchical derivation (libsodium `crypto_kdf`) was the path DPKI walked: a root seed derives revocation seeds, those derive device seeds, those derive per-app seeds. Without DPKI, that hierarchy is no longer wired up by default. Lair still supports it via `lair-keystore-import-seed` and `hc_seed_bundle`, but no system app uses it.
- **Backup.** Lair is encrypted at rest under a passphrase. Seed export through `hc_seed_bundle` produces an encrypted blob; mnemonic export was envisioned for the root/revocation seeds but the user-facing flow was never shipped at the conductor level — it lived in DeepKey ([Part 2](https://blog.holochain.org/part-2--holochain--holo-accounts--cryptographic-key-management--and-deepkey/)).
- **Cryptographic primitives exposed to zomes.** `sign`, `verify`, `x_25519_x_salsa20_poly1305_encrypt/decrypt` (libsodium `crypto_box`), and `ed_25519_x_salsa20_poly1305_*` for sender-anonymous boxes ([Cryptography functions](https://developer.holochain.org/build/cryptography-functions/)).

## DPKI / DeepKey: deeper post-mortem

**What it offered.** DeepKey was a system hApp installed first on every conductor; every other hApp's install would register its agent key inside DeepKey. Provided:

- A **keyset root** anchored to a *root seed* held by the user. Multiple devices could be admitted into one keyset via `DeviceInvite` / `DeviceInviteAcceptance` pairs, so the same human's per-app keys across all devices were known to be the same human ([deepkey 2023 docs](https://github.com/holochain/deepkey/blob/main/docs/2023/README.md)).
- **`KeyRegistration`** entries with `Create`, `Update` (replacement), and `Delete` (revocation) variants — key rotation as a first-class on-chain action.
- **`ChangeRule`** governing *who* can authorize key changes, parameterized by `AuthoritySpec { authorized_signers: Vec<AgentPubKey>, sigs_required: u8 }`. Default was 1-of-1 with a revocation key, but the structure allowed arbitrary M-of-N.
- A **`Generator`** key required to register new app keys, locked behind a passphrase prompt to prevent silent registration of attacker keys.
- A **query API** every other hApp could call: "is this `AgentPubKey` still the active key for its keyset, or has it been rotated/revoked?"

**Timeline.**

- 2018 — original `holochain/dpki` repo opened ([Initial Idea](https://github.com/holochain/dpki/issues/1)).
- 2019–2022 — multiple rewrites; original repo deprecated in favor of `holochain/deepkey`.
- December 2024 — Holochain 0.4 ships DeepKey behind the `unstable-dpki` compile-time feature flag ([Upgrade 0.3 → 0.4](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4); [Testing Methodologies](https://blog.holochain.org/testing-methodologies-and-core-happs-api-dpki-aware/)).
- 0.5 — DPKI remains gated behind the unstable flag.
- 0.6 (2025) — every DPKI knob removed from `conductor-config.yaml`: `dpki:` block, `device_seed_lair_tag`, `danger_generate_throwaway_device_seed`, `dna_path`, `network_seed`, `allow_throwaway_random_dpki_agent_key`, `no_dpki` ([Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)).

**Why it was pulled.** No single canonical post-mortem document — neither Dev Pulse 153 nor the upgrade guide explains the rationale, only the fact of removal. Reasoning has to be assembled from adjacent sources:

- The 0.4 announcement framed DPKI as part of a deliberate "shrink to a stable core" effort: features that needed more time were pushed behind `unstable-*` flags so the core could stabilize independently.
- The 0.6 release was branded as shipping the "immune system" (warrants, blocking) and the iroh transport. DPKI was removed alongside other config simplifications.
- DeepKey was structurally a system hApp every conductor had to install — every Holochain upgrade had to migrate not just the conductor but a deployed DeepKey DNA, including any signed registrations from prior versions. Cross-cutting coupling that makes shipping breaking-change releases vastly harder.

The honest reading: DeepKey was a hard problem that competed for engineering attention with networking (Kitsune → Kitsune2, tx5 → iroh) and reliability work, and the team chose reliability. Canonical citation for "DPKI is not shipping" is the 0.5 → 0.6 upgrade guide; no blog post titled "we are removing DPKI."

## Multi-device and key rotation today

DPKI was the *only* designed answer to multi-device. Without it:

- **Each device is a separate agent on the network.** Installing the same hApp on a phone and a laptop produces two distinct `AgentPubKey`s, two source chains, two sets of authored entries. From the DHT's perspective these are two unrelated agents — i.e. **one user with two devices is currently indistinguishable from a Sybil pair**. Apps that want "this is the same user" semantics must build linking logic themselves (typically by having one device sign a statement claiming the other's pubkey).
- **No canonical key rotation.** If a private key is compromised, the standard answer post-0.6 is: install the hApp fresh on a new device, get a new key, somehow communicate the new key to peers, abandon the old source chain. No on-chain "this key is revoked, here's the successor" primitive in core. Issue [holochain#4138](https://github.com/holochain/holochain/issues/4138) implemented a key-update API at the conductor level, but it depended on DPKI and is effectively orphaned in 0.6.
- **App-level workarounds.** Apps that need device continuity (Volla Messages, Moss) implement linking with their own zome logic — a chat group might accept a "new device for existing member" message signed by both keys.

## Warrants in detail

A warrant is a signed structured payload emitted by a validation authority when it detects that an op fails validation:

- **Who signs.** The validator who detected the bad op. Signature is over the offending op plus the failure reason.
- **Gossip path.** As of 0.6, warrants are delivered to **agent-activity authorities** for the bad agent's pubkey (peers whose arc covers that key's address), and additionally to "anyone who queries a bad agent's public key" so they can refuse to interact ([Dev Pulse 153](https://blog.holochain.org/dev-pulse-153-holochain-0-6-released-with-immune-system/)). To check before opening a connection, an agent calls `get_agent_activity` and inspects the warrants list.
- **Effect on the warranted agent.** Receiving peers block all network communication with that pubkey and may delete data they had stored from it.
- **Permanence.** A 0.6 warrant against an integrity violation is permanent for that pubkey — no rebuttal mechanism in core. The agent cannot un-warrant themselves; their only recourse is to generate a new key and start a new chain (then face the multi-device/Sybil problem above). Roadmap mentions "revocable app-level warrants… to non-definitively block an agent" as future work.
- **Can warrants be wrong?** In principle no — a warrant is cryptographic proof of a *deterministic* validation failure on a signed op. If validation logic is non-deterministic across versions or across nodes with stale DNA, an agent can be warranted for an op that *would* validate elsewhere. The determinism problem ([determinism.md](determinism.md)).
- **Pre-0.6 limitation, still partially live.** Warrants are not enforced during the initial gossip handshake; an unauthorized agent can connect briefly before being warranted and blocked.

## Encrypted (private) entries

Every entry type in an integrity zome can be marked `#[entry_def(visibility = "private")]`:

- **What goes where.** The entry's *content* stays on the author's source chain only, in lair's encrypted database. The *action* (`Create`/`Update`/`Delete` with the entry hash) is still gossiped to the DHT — peers know an entry of that type was authored, just not what it contains ([working-with-data](https://developer.holochain.org/build/working-with-data/)).
- **Encryption scheme.** At-rest encryption uses the conductor's passphrase-derived key over the lair store; libsodium's `crypto_secretstream` / `crypto_secretbox` family. No per-entry asymmetric encryption — the author's lair holds the only copy.
- **No selective sharing primitive.** If Alice wants to share a private entry with Bob, she must explicitly encrypt the payload to Bob's X25519 pubkey and send it via `call_remote` or write a *public* entry containing the ciphertext. No built-in "entry encrypted to a group" primitive.

## No MLS — the workaround

Holochain has no Messaging Layer Security implementation, no group-key ratchet, no built-in forward-secret group-messaging primitive. Apps that need group cryptography roll their own. Two examples:

- **Relay** (the Volla messenger) uses what its blog post calls "a separate, 256-bit encrypted P2P network for every conversation" — each chat is a separate DNA hash, isolating membership at the network layer rather than via per-message ratcheting ([Relay spotlight](https://blog.holochain.org/happs-spotlight-relay/); [Volla partnership](https://blog.holochain.org/volla-partnership-announcement/)). No public documentation of forward secrecy, post-compromise security, or group-key rotation. A compromised device's key compromises every prior message in every group it was in.
- **Moss / The Weave** ([Moss](https://github.com/lightningrodlabs/moss); [The Weave](https://theweave.social/)) uses the same per-group-DNA pattern: each group is its own DHT. Members are the agents who installed that DNA. Membership change = new agent installs the DNA; revocation has no clean primitive.

The honest assessment: every Holochain group-messaging app has the security profile of "group chat where the group is the network, members are network members, and forward secrecy is whatever Signal's threat model would call 'none.'" An MLS-equivalent runtime primitive is not on the public Holochain roadmap.

## Sources

- [Agent-centric Digital Identity (Friedman)](https://medium.com/h-o-l-o/agent-centric-digital-identity-5314d507f0ab)
- [Concepts — Source Chain](https://developer.holochain.org/concepts/3_source_chain/)
- [Concepts — Validation (warrants)](https://developer.holochain.org/concepts/7_validation/)
- [Build Guide — Genesis Self-Check Callback](https://developer.holochain.org/build/genesis-self-check-callback)
- [Build Guide — Cryptography functions](https://developer.holochain.org/build/cryptography-functions/)
- [Build Guide — Capabilities](https://developer.holochain.org/build/capabilities/)
- [Build Guide — Working with data](https://developer.holochain.org/build/working-with-data/)
- [Glossary](https://developer.holochain.org/resources/glossary/)
- [Lair keystore](https://github.com/holochain/lair)
- [DeepKey repo](https://github.com/holochain/deepkey)
- [DeepKey 2023 design doc](https://github.com/holochain/deepkey/blob/main/docs/2023/README.md)
- [Initial DPKI idea (2018)](https://github.com/holochain/dpki/issues/1)
- [Issue #4138: DPKI key update API](https://github.com/holochain/holochain/issues/4138)
- [Upgrade 0.3 → 0.4](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4)
- [Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
- [Dev Pulse 153: Holochain 0.6 with Immune System](https://blog.holochain.org/dev-pulse-153-holochain-0-6-released-with-immune-system/)
- [Dev Pulse 94: When Signup Goes Wrong](https://blog.holochain.org/dev-pulse-94-when-signup-goes-wrong/)
- [Part 1: Cryptographic Key Management](https://blog.holochain.org/part-1--holochain--holo-accounts--and-cryptographic-key-management/)
- [Part 2: DeepKey](https://blog.holochain.org/part-2--holochain--holo-accounts--cryptographic-key-management--and-deepkey/)
- [Testing Methodologies (DPKI-aware)](https://blog.holochain.org/testing-methodologies-and-core-happs-api-dpki-aware/)
- [Relay hApps spotlight](https://blog.holochain.org/happs-spotlight-relay/)
- [Volla Partnership Announcement](https://blog.holochain.org/volla-partnership-announcement/)
- [Moss](https://github.com/lightningrodlabs/moss) / [The Weave](https://theweave.social/)
- [Sybil attack vulnerability trilemma (Tandfonline)](https://www.tandfonline.com/doi/full/10.1080/17445760.2024.2352740)
