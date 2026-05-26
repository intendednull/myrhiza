**Date:** 2026-05-22
**Status:** active
**Subject:** Per-method survey — eight DID methods compared on architecture, registry, recovery, and production status. The catalog file.

# DID methods — comparative survey

Eight DID methods matter for Myrhiza's identity decision. They split into four architectural groups:

| Group | Methods | Verifiable Data Registry |
|---|---|---|
| **Self-resolving** | `did:key`, `did:peer` | None — the DID encodes its own key |
| **Web-hosted** | `did:web`, `did:webvh` | HTTPS server controlled by holder |
| **Blockchain-anchored** | `did:ion`, `did:ethr`, `did:cheqd` | Bitcoin / Ethereum / Cosmos |
| **Operator-centralized** | `did:plc` | `plc.directory` (Bluesky PBC) |

See [`rotation.md`](rotation.md) for how each handles long-term-identity-vs-active-signing-key separation — that's the file Plan B-2 readers should consult.

---

## `did:web`

**Spec:** [w3c-ccg/did-method-web](https://w3c-ccg.github.io/did-method-web/), current draft. CCG (Credentials Community Group) rather than a W3C WG track; no REC.

**Identifier shape:** `did:web:example.com` → resolves to `https://example.com/.well-known/did.json`. Path syntax (`did:web:example.com:users:alice` → `https://example.com/users/alice/did.json`) is also supported.

**Update mechanism:** Edit the JSON file. Authority = whoever controls the HTTPS server and DNS.

**Recovery:** None at the method level — it's whatever recovery your DNS registrar + hosting provider offers. In practice, "phone support and pray." [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md) calls this out explicitly.

**Production status:** Highest enterprise adoption of any method. Microsoft Entra Verified ID migrated *to* `did:web` after dropping `did:ion` in December 2023. The EU eIDAS 2.0 EUDI Wallet pilots use it. Trust roots that are organizationally large (`did:web:gov.bc.ca`) work well; individual users rarely set up DNS.

**Strengths:** Trivial to operate — any developer with a static site can host. Composes with normal HTTPS/CDN infrastructure. Resolves in browsers.

**Weaknesses:**

- Whoever controls the domain controls the identity. Lose the domain → lose the identity.
- No history — current DID document is whatever the server returns *now*. A resolver fetching at T2 has no way to verify that the document signed something at T1.
- DNS + TLS as the security anchor. If a CA is compromised or DNSSEC isn't deployed, a MITM attacker can serve a different DID document.
- Hosting cost / availability is on the holder.

**Implications for Myrhiza:** A useful import target — Myrhiza could publish a `did:web:<domain>` representation of a peer-author for external resolvers that don't speak Myrhiza's native format. Useless as Myrhiza's *native* identity primitive because it requires DNS.

## `did:key`

**Spec:** [w3c-ccg/did-key-spec](https://w3c-ccg.github.io/did-key-spec/), current draft. CCG track.

**Identifier shape:** `did:key:z6Mki...` where `z` is multibase base58btc, the suffix encodes a multicodec key-type prefix + raw public key bytes. The DID *is* the key — no resolution needed; the resolver can decode the DID document deterministically from the identifier.

**Update mechanism:** **None.** A `did:key` cannot be updated — the DID is the key.

**Recovery:** **None.** Lose the private key, lose the identity.

**Key types supported:** Ed25519, secp256k1, P-256, P-384, P-521, RSA, BLS12-381. Multicodec prefix identifies the type.

**Production status:** Heavy use as a *bootstrap* identifier — e.g. in DIDComm pairwise key exchange, in early-binding-then-upgrade patterns, in `did:peer` numalgo 0 (which is just a wrapper around `did:key`). Almost no use as a primary user identity, because the no-update + no-recovery posture is incompatible with multi-year identity.

**Strengths:** Fully decentralized — no VDR. Zero infrastructure. Resolvable offline. Conceptually clean: the DID is the verification material.

**Weaknesses:**

- Key rotation = identity change. Cannot decouple long-term identity from active signing key.
- No services, no key history, no controller delegation. Just a key.

**Implications for Myrhiza:** Mostly relevant as a *building block* — Myrhiza's `wuser` bech32m encoding of an Ed25519 public key is functionally equivalent to a `did:key` for the same Ed25519 key. The two encodings are interconvertible. If Myrhiza ever wants to mint a DID-Core-compatible representation of an author key for external consumers, `did:key` is the path of least resistance. **It is not a candidate for Plan B-2's primary identity** — the no-rotation property is disqualifying.

## `did:ion`

**Spec:** [Sidetree v1.0.0](https://identity.foundation/sidetree/spec/), 2021-03-09. ION-specific spec is the README of the `decentralized-identity/ion` repository.

**Identifier shape:** `did:ion:EiC...` — the suffix is a hash committing to the initial DID document. ION is a Layer-2 protocol: Sidetree batches anchor commitments into Bitcoin transactions, while DID documents themselves live in IPFS.

**Update mechanism:** Sidetree operations (create, update, recover, deactivate) are signed by holder-controlled update/recovery keys, batched by a Sidetree node, and anchored on Bitcoin. A resolver replays operations from the genesis Bitcoin block + IPFS to compute the current DID document.

**Recovery:** Built into the protocol — a *recovery key* (kept offline) can rotate the active signing/update keys without changing the DID. This is the most architecturally clean recovery model of any method. See [`rotation.md`](rotation.md).

**Production status:** **Microsoft removed `did:ion` as a trust system option from Microsoft Entra Verified ID in December 2023, replaced by `did:web`.** The `decentralized-identity/ion` repo's last release is v1.0.4 (2022-06-09). The Sidetree spec repo had its v1.0.0 release in 2021-03 and minor activity since. The public ION network is nominally still running — anyone can operate a node — but the major operator (Microsoft) stopped running production nodes.

This matters for Myrhiza: ION's architecture is the *most sophisticated* of any method (Bitcoin-anchored, Sidetree-batched, IPFS-hosted, multi-key recovery), but the operational ecosystem has collapsed. See [`abandoned.md`](abandoned.md) for the full post-mortem.

**Implications for Myrhiza:** Lift the *architecture* — recovery-key vs update-key vs signing-key tri-split is exactly the asymmetry Plan B-2 needs. Don't lift the *implementation* — Bitcoin-anchoring is not a Myrhiza requirement and the maintenance ecosystem isn't there. See [`rotation.md`](rotation.md) §"did:ion lessons".

## `did:peer`

**Spec:** [DIF peer-did-method-spec](https://identity.foundation/peer-did-method-spec/), v1.0 Draft. Author: Daniel Hardman (Evernym/Indicio).

**Identifier shape:** `did:peer:<numalgo><method-specific-id>` — five numalgo variants:

| Numalgo | Description | Use |
|---|---|---|
| **0** | Wraps a single `did:key` — same encoding | Bootstrap / single-key identity |
| **1** | Genesis DID document hash | Early Aries; mostly historical |
| **2** | Encoded keys + services in identifier | DIDComm v2 endpoints, multi-key |
| **3** | SHA-256 hash of a numalgo-2 DID | Short form after initial peer exchange |
| **4** | Long-form + short-form with static resolution | Current production for DIDComm v2 |

**Update mechanism:** Pairwise — both parties hold the DID document; rotation requires out-of-band sync between peers. No global registry.

**Recovery:** Each pairwise relationship is independent — there's no aggregate identity to recover. If you lose a `did:peer` you lose that pairwise relationship (Bob's view of Alice) but not your other relationships.

**Production status:** The standard DID method in DIDComm v2 (DIF) and Hyperledger Aries. Numalgo 4 is current production. Aries-based deployments (Indicio, Trinsic, Evernym/Avast) ship this at modest scale.

**Strengths:** No registry. Offline-creatable. Multiple keys + service endpoints encoded directly in the identifier (numalgo 2/4). Inherent privacy — different `did:peer` per relationship, no correlatable identifier.

**Weaknesses:** No aggregate identity — you have N `did:peer`s, one per relationship, not one identity. Pairwise rotation requires both parties online (or a deferred message). Numalgo 1 abandoned mid-evolution.

**Implications for Myrhiza:** The *pairwise* and *offline-creatable* properties match Myrhiza's P2P posture well. The *no aggregate identity* property does NOT — Plan B-2 wants one user identity across many topics/peers. Useful as a building block for a sub-identifier (e.g. per-peer per-behavior keypair, like a Myrhiza `behavior_id`), not as the primary user identity.

## `did:plc`

**Spec:** [did-method-plc](https://github.com/did-method-plc/did-method-plc). Operator: Bluesky PBC.

**Already covered in depth at [`prior-art/at-protocol/identity.md`](../at-protocol/identity.md). DO NOT duplicate here.** That file is the load-bearing prior art for Myrhiza's rotation/signing key split. The summary points for this catalog:

- ~12M+ DIDs as of October 2024.
- Centralized in operation — `plc.directory` is run by Bluesky PBC. Anyone can resolve; only Bluesky can write the canonical log.
- 1–5 rotation keys (priority-ordered, secp256k1 or P-256, kept offline).
- Single signing key (`#atproto`) on the PDS, rotates frequently.
- 72-hour rewrite window via higher-priority rotation key — the recovery story.
- Bluesky has published "future federation" intent for `plc.directory` but has not delivered it.

**Implications for Myrhiza:** Architectural model of choice for Plan B-2 (validated by `prior-art/at-protocol/identity.md`). **Do not lift the centralized operator** — Myrhiza's P2P posture is incompatible with a `plc.directory`-equivalent. The pattern Myrhiza wants is the *rotation-key set + signing-key separation*, materialized in Myrhiza's own per-peer DAG (signed PLC ops → signed Myrhiza identity events), not in a central directory.

## `did:webvh` (formerly `did:tdw`)

**Spec:** [DIF didwebvh](https://identity.foundation/didwebvh/), v1.0 stable + Editor's Draft. Formerly "Trust DID Web" (`did:tdw`); renamed to `did:webvh` (web + verifiable history) circa the v0.4 → v0.5 transition in 2025. Primary-source citation for the rename date is pending — the DIF spec page redirects from the old `did:tdw` slug but does not preserve a changelog with date.

**Identifier shape:** `did:webvh:<scid>:<host-and-path>` where `<scid>` is a self-certifying identifier hash committing to the genesis state.

**Update mechanism:** A JSONL log (`did.jsonl`) lives alongside the `did.json` on the HTTPS host. Each line is a signed entry containing:

- A version ID.
- The previous version's hash (Merkle-chained).
- The full DID document for this version (or a JSON Patch from the prior).
- A signature by an *authorized key* from the prior version.
- Optionally, witness proofs (a quorum of external attesting servers).

Resolution = fetch the JSONL log, verify the chain from genesis, return the latest verified DID document. Each version is verifiable — a resolver at T2 can prove what the document was at T1.

**Recovery:** Authorized-update-key rotation is in the log. Pre-rotation commitments (committing to *future* update keys via hashes in the current entry) provide quantum-resistant key rotation when the keys themselves aren't quantum-safe.

**Production status:** BC Government (British Columbia) is the lead deployment partner; Trust over IP supports the spec. Several Indy-to-`did:webvh` migration pilots reported in 2025. Modest scale (hundreds of orgs); not yet at `did:web` adoption.

**Strengths:** Fixes `did:web`'s "no history" problem without requiring a blockchain. Same HTTPS-as-VDR convenience. Verifiable log gives a third party the ability to detect server-side history tampering. Pre-rotation gives quantum hedge.

**Weaknesses:** Still anchored on DNS + TLS. Server can refuse to serve old log entries (resolver sees a *truncated* log; nothing in the protocol forces the server to be complete unless witnesses are mandatory). Witness ecosystem is nascent.

**Implications for Myrhiza:** The *verifiable history* idea — a log of signed identity-state transitions, anchored in cryptographic chain rather than a central operator — is a direct precedent for what Myrhiza's per-author DAG already does. Myrhiza's "identity events on a DAG, validated by `state-apply`" is functionally `did:webvh`-shaped without DNS dependency. The lesson is "the data model is right; the transport (HTTPS) is wrong for our use case."

## `did:ethr`

**Spec:** [DIF ethr-did-resolver](https://github.com/uport-project/ethr-did-resolver). Originally uPort (2018); now under `decentralized-identity` org maintained by mirceanis (also Veramo lead). Current resolver version v13.0.0 (2026-05-18).

**Identifier shape:** `did:ethr:<network>:<eth-address-or-pubkey>` — e.g. `did:ethr:mainnet:0xf3beac30c498d9e26865f34fcaa57dbb935b0d74` or `did:ethr:polygon:0x...`. Default network is Ethereum mainnet.

**Update mechanism:** ERC-1056 smart contract on the relevant chain. The Ethereum account (EOA) controlling the address can:

- Add/remove keys via `setAttribute` transactions.
- Change controller (the "owner" of the DID — defaults to the address itself).
- Add service endpoints.

A resolver scans contract events from genesis (or last cached block) to compute the current DID document.

**Recovery:** EOA-level — whoever holds the private key for the Ethereum address controls the DID. Multi-sig or smart-contract wallet patterns work (delegate control to a Gnosis Safe).

**Production status:** Widest blockchain-DID deployment by far. DeFi identity, Lens Protocol predecessors, SpruceID's Sign-In With Ethereum (SIWE) flows interop. Veramo's flagship method. Multi-chain support is real (mainnet, polygon, optimism, base, arbitrum).

**Strengths:** Existing wallet ecosystem (every Ethereum wallet can sign for a `did:ethr`). On-chain history is auditable. Cost-amortizable via L2s. Decentralized in a meaningful sense.

**Weaknesses:** Per-update gas cost (a few cents on L2, a few dollars on mainnet). Requires the resolver to scan contract events — heavy infra for a public resolver. Privacy: every update is publicly logged forever. secp256k1-only natively (other key types can be added via key list, but the "controller" is always an EOA which is secp256k1).

**Implications for Myrhiza:** **Wrong shape for Myrhiza.** Myrhiza is P2P, not blockchain-anchored. Useful as a *bridge* — a Myrhiza peer could publish a `did:ethr` representation of their author key for external Ethereum-native consumers — but as a primary identity it imposes gas costs and a wallet UX Myrhiza doesn't want.

## `did:cheqd`

**Spec:** [cheqd DID method ADR-001](https://docs.cheqd.io/product/architecture/adr-list/adr-001-cheqd-did-method). Cosmos SDK chain operated by the cheqd Foundation; mainnet live since 2021.

**Identifier shape:** `did:cheqd:<namespace>:<uuid-or-indy-style>` — e.g. `did:cheqd:mainnet:zABCD...`. Namespace separates mainnet from testnet.

**Update mechanism:** Signed Cosmos transactions to the `cheqd-node` chain. The cheqd ledger module handles create/update/deactivate. Updates require signatures from *all controllers* (multi-controller requires all sign; not a quorum).

**Recovery:** Controller key rotation via update transaction. No protocol-level recovery — if you lose all controller keys, the DID is unrecoverable. (A small ecosystem of custodial cheqd-as-a-service operators acts as a recovery path commercially.)

**Production status:** Commercial SSI focus. Partnerships with Anonyme Labs, Vouched (May 2026), and an Animo Solutions integration. Live mainnet, validators publicly listed. Native CHEQ token economics, fee-paid identity transactions. Ed25519 (Cosmos-native) and secp256k1 supported.

**Strengths:** Live commercial deployment. Better key-type story than `did:ethr` (Ed25519 native via Cosmos). Cheaper per-operation than mainnet Ethereum. Built for trust-registry use cases (KYC providers, education credential issuers).

**Weaknesses:** Token-coupled — using the network requires CHEQ tokens. Single-chain (not multi-chain like `did:ethr`). All-controllers-signed update is awkward for delegated management. Validator set is small (Cosmos-style).

**Implications for Myrhiza:** Like `did:ethr`, wrong shape — blockchain anchoring isn't a Myrhiza requirement. The cheqd choice of Ed25519 over secp256k1 is the right call for modern crypto (matches Myrhiza) and demonstrates that DID methods *can* bless Ed25519, contradicting `did:plc`'s P-256/secp256k1-only design.

## See also

- [`rotation.md`](rotation.md) — load-bearing comparison of rotation mechanisms.
- [`crypto.md`](crypto.md) — which key types each method supports + Cremers ETK 2025 implications.
- [`adoption.md`](adoption.md) — honest scale audit.
- [`abandoned.md`](abandoned.md) — `did:ion`, `didkit`, the long tail.

## Sources

- W3C DID Extensions — Methods registry — <https://www.w3.org/TR/did-extensions-methods/>.
- `did:web` spec — <https://w3c-ccg.github.io/did-method-web/>.
- `did:key` spec — <https://w3c-ccg.github.io/did-key-spec/>.
- `did:ion` repository — <https://github.com/decentralized-identity/ion>.
- Sidetree spec — <https://identity.foundation/sidetree/spec/>.
- `did:peer` spec — <https://identity.foundation/peer-did-method-spec/>.
- `did:plc` spec — <https://github.com/did-method-plc/did-method-plc>.
- `did:webvh` spec — <https://identity.foundation/didwebvh/>.
- `ethr-did-resolver` — <https://github.com/uport-project/ethr-did-resolver>.
- cheqd DID method ADR — <https://docs.cheqd.io/product/architecture/adr-list/adr-001-cheqd-did-method>.
- Microsoft Entra Verified ID — `did:ion` deprecation — <https://learn.microsoft.com/en-us/entra/verified-id/whats-new>.
- `prior-art/at-protocol/identity.md` — `did:plc` deep dive (in-tree).
