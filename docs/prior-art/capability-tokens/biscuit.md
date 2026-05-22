**Date:** 2026-05-22
**Status:** active
**Subject:** Biscuit — Protobuf + Ed25519-chained capability token with Datalog caveat language (Clever Cloud → Eclipse Foundation, 2019–)

# Biscuit

Biscuit is a capability token that combines two ideas: a **Macaroon-style block-chain of attenuations** (each block restricts the previous), and a **Datalog dialect for caveat predicates**. Unlike Macaroons, Biscuit uses **Ed25519 (or ECDSA secp256r1) public-key signatures** rather than HMAC — so any party with the issuer's public key can verify, no shared-secret precondition.

## Key facts

| Fact | Value |
|---|---|
| Origin | Clever Cloud (2019), led by [Geoffroy Couprie](https://www.clever-cloud.com/blog/engineering/2021/01/14/biscuit-tokens/) |
| Current stewardship | [Eclipse Biscuit Project](https://github.com/eclipse-biscuit), Eclipse Foundation |
| Current spec | v3.3, [SPECIFICATIONS.md](https://github.com/eclipse-biscuit/biscuit/blob/main/SPECIFICATIONS.md) (Dec 17, 2024) |
| Versions supported | v3.0 through v3.3 (wire-encoded as ints 3–6); v1 + v2 legacy |
| Wire format | Protocol Buffers, designed to fit in HTTP cookies |
| Crypto | Ed25519 (primary, RFC 8032); ECDSA secp256r1 (secondary, SEC2v1) |
| License | Apache-2.0 (spec + implementations) |
| Reference impl | [`eclipse-biscuit/biscuit-rust`](https://github.com/eclipse-biscuit/biscuit-rust) — crate `biscuit-auth` 6.0.0 (Jul 16 2025) |
| Other impls | Python, Haskell, Java (v1 + v3.3 WIP), Go (v1), C#, WASM; coverage varies per language |
| Caveat language | Datalog with no negation, with closures + boolean expressions |
| Production users | Clever Cloud, Apache Pulsar (via plugin) |

## Construction

A Biscuit token is an **append-only chain of signed blocks**, each carrying Datalog facts/rules/checks. The first block is the *authority* signed by the issuer's key pair; each subsequent block is signed by an ephemeral key pair, where the previous block carries the next block's public key.

```
Authority Block (B0)         Block (B1)                   Block (B2)
─────────────────────        ─────────────────            ─────────────────
[ Datalog: facts/rules ]     [ Datalog: checks ]          [ Datalog: checks ]
next_pubkey: pk1             next_pubkey: pk2              next_pubkey: pk3
sig: ECDSA(issuer_sk, ...)   sig: ECDSA(eph_sk1, ...)     sig: ECDSA(eph_sk2, ...)
                             attenuated by holder           attenuated by holder
```

The final block carries a **proof**:
- For an *attenuable* token: the still-unused private key `eph_skN` (so the next holder can extend).
- For a *sealed* token: a sealing signature on the chain (so the chain is closed and tamper-evident but not further extendable).

Verification: walk the chain checking each signature with the previous block's `next_pubkey`, confirm the issuer's authority signature matches a known public key, then evaluate the Datalog program against the verifier's facts.

The "ephemeral key per block" pattern is critical: it gives chain-of-trust semantics (each holder commits to an ephemeral identity for their attenuation step, then discards the key) without requiring a long-term key per delegator.

## The Datalog dialect

Biscuit's policy language is a restricted Datalog with three kinds of clauses per block:

- **Facts:** ground predicates the block adds. `user("alice");` `right("read", "/files/a.txt");`
- **Rules:** Datalog rules with a head and body. `right($u, $op, $r) <- user($u), authorized($u, $op, $r);`
- **Checks:** assertions the verifier must satisfy. `check if user($u), right($u, "read", "/files/a.txt");`

Key restrictions:
- **No negation** (no negation-as-failure). Simplifies semantics; avoids stratification.
- **Trust scopes:** rules can specify `trusting <pubkey>, ...` to constrain which blocks' facts they can use, preventing later blocks from spoofing facts a rule depends on.
- **Closures + booleans** for expressions (arithmetic, string ops, set ops, short-circuit `&&`/`||`).
- **Datatypes:** integers, strings, byte arrays, dates, booleans, null, sets, arrays, maps.

The verifier also brings *its own* facts (request context: `time(2026-05-22T...)`, `operation("read")`, `resource("/files/a.txt")`) and **policies** (allow/deny clauses). The token's checks plus the verifier's policies must all be satisfied.

The Datalog model is powerful — you can express "this token is valid only on Tuesdays, only for resources matching a prefix, only when the user is in a group" — but **requires an evaluator embedded in every verifier**. This is the load-bearing complexity. For an in-process kernel, that's fine; for a peer's interaction component, it's another WASM-sized dependency.

## Attenuation model

To attenuate a Biscuit, the holder:
1. Generates a fresh ephemeral key pair `(eph_skN+1, eph_pkN+1)`.
2. Constructs a new block with additional checks (and/or facts/rules), pointing to `eph_pkN+1` as `next_pubkey`.
3. Signs the new block with `eph_skN` (the private key they hold).
4. Either keeps the new ephemeral private key (still attenuable) or seals.

This is structurally similar to Macaroon attenuation but uses *signatures* rather than HMACs, so anyone with the original issuer's public key can verify the full chain — no shared secret with the issuer required.

## Production usage

- **Clever Cloud** uses Biscuit internally for service-to-service authorization across its PaaS infrastructure ([Clever Cloud engineering blog](https://www.clever-cloud.com/blog/engineering/2021/01/14/biscuit-tokens/)).
- **Apache Pulsar** has a plugin ([biscuit-auth-pulsar](https://github.com/biscuit-auth/biscuit-pulsar)) for client authorization, replacing JWT-based auth.
- The Eclipse move (2024) suggests the project is positioning for broader institutional adoption.

## What Biscuit does NOT solve

- **Revocation.** Same problem as Macaroons. A leaked Biscuit is valid until expiry. Biscuit has support for a `revocation_id` (a fixed identifier per block) the verifier can blacklist — but this requires a centralized revocation list lookup, which Biscuit itself does not provide.
- **Datalog complexity in the verifier.** Every verifier must embed a Datalog evaluator. For a kernel that wants minimal trusted code, this is a real cost.
- **Delegation across issuer realms.** The chain is linear and rooted in one issuer. Multi-issuer scenarios (e.g. "I trust X's cap if Y also signs") require workarounds like third-party blocks.
- **DID/PKI binding.** Biscuit identifies issuers by raw Ed25519 public keys, not DIDs. There's no standard mapping from a Biscuit issuer to a DID document, so cross-system identity is application-layer.

## Implications for Myrhiza

Biscuit is the closest format to "what we'd build if we had to ship a wire format tomorrow." Three pieces are worth a hard look:

1. **The ephemeral-key-per-block pattern** is a clean way to avoid asking every delegator for a long-term key. We'd want this regardless of which format we picked.
2. **Datalog-for-policy** is powerful, but it's a *language* — and we're already building a runtime that hosts WASM-typed handles. The capability *expression* (which fn, which resource) should live at the WIT level; the *delegation conditions* (under what circumstances) is where Datalog could earn its keep, IF we're comfortable embedding an evaluator in `state-apply`.
3. **Protobuf-on-the-wire** is fine but not where we should optimize. The wire format is a serialization concern; the model is the thing.

**Avoid:** treating Biscuit as plug-and-play. Datalog in `state-apply` would need to be deterministic across implementations (which Datalog evaluation order subtly is not, depending on indexing strategy). Either fix the order or use a more restricted predicate language.

See [`comparisons.md`](comparisons.md) for Biscuit vs Macaroon vs UCAN side-by-side, and [`lessons.md`](lessons.md) for the action items.

## Sources

- Couprie, G. "Biscuit: a bearer token with offline attenuation and Datalog authorization policies." [Clever Cloud engineering blog, 2021-01-14](https://www.clever-cloud.com/blog/engineering/2021/01/14/biscuit-tokens/).
- Eclipse Biscuit Project. [Spec v3.3](https://github.com/eclipse-biscuit/biscuit/blob/main/SPECIFICATIONS.md), Dec 17 2024.
- [`eclipse-biscuit/biscuit-rust`](https://github.com/eclipse-biscuit/biscuit-rust), crate `biscuit-auth` 6.0.0.
- [biscuitsec.org](https://www.biscuitsec.org/) — canonical introduction.
- Apache Pulsar Biscuit plugin: [`biscuit-auth/biscuit-pulsar`](https://github.com/biscuit-auth/biscuit-pulsar).
