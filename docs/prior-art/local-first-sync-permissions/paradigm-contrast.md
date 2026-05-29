**Date:** 2026-05-29
**Status:** active
**Subject:** The two paradigms — peer-verified authority (Jazz, Myrhiza-shaped) vs trusted-middlebox authority (Zero / Triplit / PowerSync, Myrhiza-rejected)

# Two paradigms for "where does the verdict live?"

Every local-first sync product answers one question: **when a write arrives, who
decides whether it is allowed, and what do other replicas have to trust?** The
shipped products split cleanly into two camps. This is the central contrast of
the folder and the spine of [lessons.md](lessons.md).

## Paradigm A — peer-verified authority (rare; Myrhiza-shaped)

The authority verdict is a **deterministic function of signed data that every
peer re-computes locally**. No server is trusted to adjudicate.

- **Exemplar**: Jazz / cojson ([jazz-cojson.md](jazz-cojson.md)).
  `determineValidTransactions` re-derives, on every peer, whether each
  Ed25519-signed transaction was made by a member whose role-at-the-time
  permitted it. The relay only relays.
- **Identity**: self-sovereign keypairs. No issuer to trust.
- **Reads**: enforced by encryption (read key sealed to authorized members), not
  by withholding bytes.
- **Failure mode of the infrastructure**: a malicious relay can withhold, delay,
  or reorder — it **cannot forge an accepted write or read encrypted content.**

## Paradigm B — trusted-middlebox authority (common; Myrhiza-rejected)

A trusted server (or trusted proxy) evaluates authorization and is the source of
truth. Replicas trust the server's verdict.

- **Exemplars**: Zero (server mutators / deprecated RLS rules,
  [zero-rocicorp.md](zero-rocicorp.md)); Triplit (schema rules evaluated
  server-side, [triplit.md](triplit.md)); PowerSync (Sync Rules over JWT claims,
  [powersync.md](powersync.md)); ElectricSQL (authorization in an external proxy,
  [electricsql-comparator.md](electricsql-comparator.md)).
- **Identity**: a JWT minted by a **trusted issuer**.
- **Reads**: enforced by the server **withholding rows** the client isn't
  entitled to (partial replication = authorization).
- **Failure mode of the infrastructure**: compromise the server or its signing
  key and the authority model is fully bypassed — forged writes, leaked reads.

## Why most shipped products are Paradigm B

Honest accounting: **Paradigm B is the overwhelmingly common shipped choice.** It
is easier — a trusted server can run arbitrary code, see plaintext, enforce
cross-row invariants with a transaction, and express ad-hoc rules without a
cryptographic protocol. Paradigm A demands a signed-state authority protocol,
encryption-based reads, and key rotation on revocation — all of which are subtle
(cojson's own source notes a past key-rotation bug). Jazz is the **rarer** model,
and even Jazz is drifting toward a trusted tier in its 2.0 rewrite
([maturity-and-the-2.0-pivot.md](maturity-and-the-2.0-pivot.md)). Do not present
peer-verified authority as the mainstream — present it as the deliberate, harder,
trust-minimizing choice that Myrhiza shares with cojson.

## The mapping to Myrhiza

| Concept | cojson (Paradigm A) | Myrhiza |
|---|---|---|
| Signed unit | transaction in a session | event in a per-author chain |
| Ordering | session = per-device stream | per-author seq + `prev` Merkle link |
| Signature / hash | Ed25519 + BLAKE3 | Ed25519 + EventHash |
| Authority holder | Group CoValue (role map) | app state + `myrhiza-permission-*` module |
| Verdict function | `determineValidTransactions` (per peer) | `state-apply` Accept/Reject (per peer) |
| Verdict timing | re-evaluated at every ingest | apply mode + pre-check dry-run (§4.4) |
| Read confidentiality | read key sealed to members | group encryption ([crypto.md](../../specs/2026-05-09-myrhiza-master-design/crypto.md)) |
| Revocation | role→`revoked` + `rotateReadKey()` | revocation flow ([distribution.md](../../specs/2026-05-09-myrhiza-master-design/distribution.md) §10.7) |
| Infrastructure | dumb relay / Jazz Cloud | dumb iroh relay ([networking.md](../../specs/2026-05-09-myrhiza-master-design/networking.md) §11.4) |

cojson is the closest shipped analogue to the Myrhiza authority model that exists.
Myrhiza's distinguishing additions: the verdict is a **WASM Component**
(`state-apply`), not framework-internal code; and pre-check is *mechanically the
same function* as apply (§4.4), so an originating peer can fail-closed before
signing — cojson validates only after a transaction exists.

## The library-layer gap this fills

[../crdts/open-problems.md](../crdts/open-problems.md) §2 ("Authority /
authorization") and §3 ("Validation / invariants") state that **no CRDT library
enforces who may write or whether the result is legal** — "CRDTs converge, then
violate." Both paradigms here are *product-layer* answers to that *library-layer*
gap. Paradigm A is the answer Myrhiza adopts; Paradigm B is the answer it
rejects. See [open-problems.md](open-problems.md) for what even Paradigm A does
**not** solve.

## Sources

- Synthesizes the per-product files in this folder; primary sources cited there.
- [../crdts/open-problems.md](../crdts/open-problems.md) (library-layer gap).
- Myrhiza spec: [convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md) §4.4,
  [networking.md](../../specs/2026-05-09-myrhiza-master-design/networking.md) §11.4.
