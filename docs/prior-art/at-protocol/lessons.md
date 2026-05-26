**Date:** 2026-05-22
**Status:** active
**Subject:** Lessons for Myrhiza from AT Protocol — validates / avoid / borrow

# Lessons for Myrhiza

The synthesis file. Other files in this corpus are evidence; this is the decision file. Format follows the standard: **validates** (atproto confirms a Myrhiza choice), **avoid** (atproto demonstrates an antipattern), **borrow** (atproto has a primitive Myrhiza should lift).

This is the file to consult when designing — particularly Plan B-2 (persistent identity), any future MLS-integration spec, any future snapshot-portability spec, or any decision about the kernel's identity capability surface.

## Validates

Things Myrhiza is doing that atproto's deployment confirms are right.

### V-1. Long-term identity must be separate from active signing key

Myrhiza Plan B-2 splits `AuthorKeypair` (long-term identity) from `PeerKeypair` (device-scoped). Atproto's `did:plc` makes the same split: a set of **rotation keys** controls identity reconfiguration, and a single **`#atproto` signing key** signs repository commits. The atproto deployment at 42M+ users validates the structural pattern.

The atproto data point: signing keys are compromised every day (PDS infrastructure leaks, individual device theft, etc.); rotation keys are rarely compromised because they're rarely used. Separating the two keeps signing-key compromise from cascading into identity loss.

**Lift**: Plan B-2's split is the right shape. The implementation details differ (atproto's signing-key vs. Myrhiza's per-device PeerKeypair are different things) but the architectural principle holds.

### V-2. Content-addressed deterministic storage for the data substrate

Myrhiza's state-apply requires deterministic state transitions. Atproto's MST gives the same: two PDSes given the same records produce byte-identical MST nodes. This validates the "deterministic-serialization + content-addressing" pattern that Myrhiza's state DAG already uses.

The atproto data point: cross-implementation interop (TypeScript reference vs Go indigo vs Rust atrium) works because the MST is content-addressed and deterministic. Any drift would manifest as CID mismatches and would be immediately visible. The pattern survives multi-implementation rigor.

### V-3. DNS-rooted publisher authority

Myrhiza's `wpub-*` HRP for module publishers + atproto's NSID for schemas both anchor authority in DNS control rather than a central registry. Atproto's deployment confirms the pattern works at scale.

The atproto data point: thousands of NSID namespaces in production (`com.whtwnd.*`, `events.smokesignal.*`, `fyi.unravel.*`, etc.); none required permission from a central authority. DNS control was sufficient.

### V-4. Strict additive schema evolution as the default

Myrhiza needs a snapshot-portability schema. Atproto's Lexicon takes the strict line: the schema language doesn't version; individual schemas evolve by additive rules only; breaking changes mean new NSID. The Bluesky team's bet is that this discipline produces more stable schemas long-term than version-friendly approaches.

The atproto data point: `app.bsky.feed.post` is essentially unchanged since 2023. Individual fields have been added (image embeds, video embeds, reply context) but no breaking changes; old data still validates. Three years of stable schema lineage at multi-million-user scale.

**Caveat**: this works for the "Twitter posts" use case where the data shape is stable. Whether it works for Myrhiza's more open-ended snapshot needs is genuinely uncertain — see [open-problems.md](open-problems.md) §"Lexicon evolution".

### V-5. Layered E2E story (identity from protocol, encryption from MLS)

Germ DM's integration confirms that **MLS layered on top of a separate identity protocol** is a workable deployment shape. Myrhiza's master spec PR #636 envisions the same with `host.mls` as a kernel capability: identity comes from Myrhiza, MLS confidentiality lives inside the capability, the two compose at the identity boundary.

The atproto + Germ data point: deployment in February 2026 to iOS, native integration with `bsky.app` profiles via Germ badges. The pattern composes cleanly.

## Avoid

Things atproto's deployment demonstrates as antipatterns Myrhiza should not repeat.

### A-1. Tiered federation creates structural centralization

Atproto's PDS / Relay / AppView trio looks federated on paper. In deployment ~99% of users are on Bluesky-operated tiers because the Relay's hardware requirements price out small operators. The tiered shape inherently concentrates power at the tier that's expensive to run.

**Lesson for Myrhiza**: if any Myrhiza role requires terabytes of storage or gigabit-class throughput, only large operators will run it and federation diversity will be aspirational. Myrhiza's tiers must all be peer-runnable on commodity hardware. The state-apply tier, the gossip-propagation tier, the snapshot-hosting tier — each must fit a hobbyist's home server. If any one of them doesn't, atproto's centralization pattern will reappear in Myrhiza.

### A-2. Single-operator identity registries are a single point of trust

`plc.directory` is Bluesky-operated; ~99% of atproto users depend on it for DID resolution and rotation operations. The trust model is "transparent server with audit log" — auditable, not cryptographically constrained. This is the load-bearing weakness atproto critics identify (see [critiques.md](critiques.md)).

**Lesson for Myrhiza**: there is no Bluesky-equivalent operator. Myrhiza's identity mechanism cannot assume a central registry. The rotation-key recovery story must work via replicated state-apply or MLS-group-state or some equivalent peer-symmetric mechanism. See [open-problems.md](open-problems.md) §"DID registry is a single-operator service".

### A-3. "Federate eventually" is non-binding

Atproto's federation opened years after launch and remains thin in practice. The stated commitment ("we don't want to be the only major operator") is sincere but produces aspirational diversity rather than structural diversity. Federation that hasn't happened on day one tends not to happen.

**Lesson for Myrhiza**: ship peer-symmetric from day one. Don't promise "P2P eventually" while running a centralized service. The deployment-shape is what matters; future-tense promises are rhetorical and unenforceable.

### A-4. ECDSA-SHA256 is a working defensible choice but not the right one for a P2P runtime

Atproto picked secp256k1 and P-256 for browser, hardware-token, and W3C-DID-method compatibility. The cost is the standard ECDSA footguns (nonce reuse) and curve incompatibility with the rest of the modern P2P ecosystem (Ed25519). Atproto's reasons are sound *for atproto*; they don't transfer to Myrhiza.

**Lesson for Myrhiza**: Ed25519 throughout (Willow precedent) is the right call. Don't pick atproto's curves just because atproto picked them — the design context is different.

### A-5. Server-side-readable DMs as a stopgap

`bsky.app`'s DMs are server-side-readable because shipping native E2E was out of scope. The result: users see "DMs" and reasonably assume privacy that isn't there. This is technically correct (the protocol never promised E2E) and operationally a footgun.

**Lesson for Myrhiza**: don't ship "private" features without E2E. If E2E isn't ready, don't call the feature private. If the feature can't survive without E2E, defer it.

## Borrow

Specific primitives or designs Myrhiza should lift.

### B-1. The rotation-key priority list

`did:plc` allows 1-5 rotation keys in priority order, with higher-priority keys able to override lower-priority ones' recent operations. This gives hot/warm/cold key tiering naturally:

- `K0`: cold-storage paper backup, "break glass in case of emergency"
- `K1`: warm key in a hardware token or secure enclave
- `K2`: hot key on the user's primary device

A compromised hot key is recoverable by a warm or cold key. A compromised warm key is recoverable by the cold key. Loss of all three is terminal (atproto can't help you either).

**Lift**: Plan B-2's `AuthorKeypair` is currently a single key. The runner-up "use a priority-ordered set" gives recoverability. Cost: more complex API, more state to track. Benefit: device-loss recovery without a separate "social recovery" mechanism.

### B-2. The 72-hour recovery window — but redesigned for P2P

The recovery rule itself is the load-bearing idea: a higher-priority key can clobber a lower-priority key's recent operation within a bounded time. The atproto-specific bits (`plc.directory`-enforced, wall-clock 72 hours) don't transfer to Myrhiza.

**Lift the rule, redesign the enforcement**: the recovery rule can be expressed as an event-ordering constraint in Myrhiza's state-apply. *"A rotation-clobber event is valid if signed by a higher-priority rotation key and references the to-be-clobbered event by digest and lands within N events (or T deterministic logical time) of the to-be-clobbered event."* The state-apply runtime enforces this deterministically; no central operator needed.

Design surface: what's N? What does "logical time" mean when peers can deliver events at different rates? The answers go in a future identity-recovery spec.

### B-3. NSID-as-collection-path

Atproto's repository keys are `<NSID>/<record-key>` — the schema namespace IS the storage path. A query for "all `app.bsky.feed.post` records" is a prefix lookup. No separate "type" field; no JSON Schema discriminator; the namespace is structural.

**Lift**: Myrhiza's snapshot schema should similarly use NSID-style namespacing as the storage key prefix. A query for "all events of type X" should be a prefix scan, not a type-tag filter. This composes naturally with content-addressed storage (the prefix is part of the canonical encoding) and with Lexicon-style schema validation (the prefix identifies the schema).

### B-4. CAR-format portability

Atproto's repositories export as CAR (Content Addressable aRchive) — IPFS's standard format for self-contained CBOR DAGs. The CAR contains the full commit, all MST nodes, all records. Migration is "export CAR, import CAR, sign one new operation pointing the DID at the new PDS."

**Lift the format, not the migration story**: Myrhiza's snapshot export should be CAR-compatible. This buys IPFS tooling interop (`ipfs car`, `go-car`, etc.) for free, and the format is already designed to be self-contained and content-addressed. The migration story differs (Myrhiza has no PDS to migrate between) but the snapshot-portability story can borrow the format directly.

### B-5. The Lexicon strict-additive evolution discipline

The discipline itself is worth borrowing: no schema language version, no breaking changes ever within a schema, additive evolution only. This forces design-time discipline and produces more stable schemas long-term.

**Lift**: Myrhiza's snapshot schema should default to the same rules. Where atproto says "publish a new NSID for breaking changes," Myrhiza should say "publish a new module version with a `migrate-from` declaration." That's a richer escape hatch than atproto provides but built on the same discipline.

### B-6. Service proxying via DID document

Atproto clients route XRPC calls to other services via the user's PDS. The user's DID document declares the PDS endpoint; the client uses it as a forwarding proxy. This means the user has one well-known endpoint that authorizes them to talk to many services.

**Lift**: Myrhiza's `Identity` could similarly declare a "kernel home" endpoint (or an analogous concept) that authorizes the user's session with other services. This composes well with the capability model — the kernel home is the capability-broker; other services trust the kernel home's attestations.

## Sources

- `did:plc` spec: <https://github.com/did-method-plc/did-method-plc>
- atproto repository spec: <https://atproto.com/specs/repository>
- atproto Lexicon spec: <https://atproto.com/specs/lexicon>
- atproto federation architecture: <https://docs.bsky.app/docs/advanced-guides/federation-architecture>
- Plan B-2 design: [`docs/specs/2026-05-19-plan-b-2-persistent-identity-design.md`](../../specs/2026-05-19-plan-b-2-persistent-identity-design.md)
- Myrhiza master spec PR #636: see `prior-art/willow/runtime-vision.md`
- Willow open problems: [`prior-art/willow/open-problems.md`](../willow/open-problems.md)
- MLS prior art: [`prior-art/mls/`](../mls/)
- Holochain prior art: [`prior-art/holochain/`](../holochain/)
