**Date:** 2026-06-08
**Status:** active
**Subject:** Object-capability revocation — the caretaker/revoker (revocable forwarder), membranes, sturdyrefs, and CapTP promises

# Object-capability revocation

How the E / Spritely-Goblins / CapTP lineage represents a capability as a
reference and *kills it while it is live*. This is the heart of Myrhiza's
"revocable subscription" requirement. Project context for these systems lives in
[`spritely-ocapn/`](../spritely-ocapn/README.md) and
[`agoric-endo/`](../agoric-endo/README.md); this file is the revocation-mechanics
cut.

## The caretaker / revocable forwarder

Origin: **D. D. Redell's 1974 Ph.D. thesis** ("Naming and Protection in
Extendible Operating Systems"), published as **MIT/Project-MAC TR-140** (November
1974) — credited by the
[c2 wiki CaretakerPattern](https://kidneybone.com/c2/wiki/CaretakerPattern) and by
Mark Miller's *Robust Composition* thesis (2006), §"Selective Revocation: Redell's
Caretaker Pattern". (The c2 wiki calls it an MIT thesis; the degree was conferred
at UC Berkeley and *published* as the MIT Project-MAC TR.)

The pattern: instead of handing out the real capability `target`, you hand out a
**forwarder** (the caretaker). The forwarder holds a one-bit **enable slot** and
the `target`. Every message it receives, it first checks the slot; if enabled, it
forwards to `target`; if not, it throws / returns a dead reference. You keep a
second object — the **revoker / gate** — which is the only thing that can flip the
slot to disabled. Sketch (E-style):

```
def makeCaretaker(target):
    var enabled := true
    def caretaker {
        match [verb, args] {
            if (enabled) { E.call(target, verb, args) }
            else { throw("revoked") }
        }
    }
    def revoker { to revoke() { enabled := false } }
    return [caretaker, revoker]
```

Properties that matter for Myrhiza:

- **The holder of the caretaker never gets `target`.** Revocation can't be undone
  by the holder because it never had the underlying reference.
- **Revocation is O(1) and synchronous** at the forwarder: flip one bit; the next
  message dies.
- **It composes**: hand out a caretaker *of a caretaker* and you get a delegation
  chain where any link can be cut independently (attenuation + revocation in one
  object). This is the ocap answer to "attenuable AND revocable."

### Membranes — transitive revocation

A single caretaker only revokes the *one* reference it wraps. If `target` hands
out further references during use, those survive. The **membrane** pattern fixes
this: it wraps `target` *and auto-wraps every capability that crosses the
boundary* in caretakers sharing the same enable slot. Revoking the membrane's slot
severs the entire transitively-reachable subgraph at once. Relevant to Myrhiza if
a subscription can spawn sub-capabilities (it should not — keep subscriptions
leaf-shaped to avoid needing membranes).

## Spritely Goblins in practice

Goblins (Guile/Racket; see [`spritely-ocapn/`](../spritely-ocapn/README.md)) does
**not** ship a built-in revocable-reference type. Per the Spritely core paper, you
build revocation yourself as a **logged revocable proxy**: a proxy holds a cell;
while the cell is false it forwards (and logs) arguments; setting the cell true
makes it raise `Access revoked!`. Same caretaker shape, hand-rolled. Goblins also
ships **sealers/unsealers** (cryptography-free, analogous to public-key
seal/unseal) for rights amplification and unforgeable object identity — orthogonal
to revocation but part of the same toolkit.

Takeaway: even in a mature ocap system, revocation is a *pattern you assemble from
references and a mutable slot*, not a primitive. Myrhiza can and should make it a
**primitive** at the kernel boundary, because the kernel — not the sandboxed app —
owns the slot.

## Live references vs sturdyrefs (persistence across disconnection)

CapTP distinguishes:

- **Live reference** — a connected, in-session handle to a remote actor.
  "Incredibly cheap, merely represented as integers on each side." Dies when the
  session/connection dies.
- **Sturdyref** — a persistent URI (e.g.
  `ocapn://<onion>.onion/s/<swiss-number>`) that survives disconnection and can be
  shared out-of-band. You **`enliven`** a sturdyref to get back a *promise* that
  resolves to a live reference: `($ mycapn 'enliven (string->ocapn-id "ocapn://…"))`.

The split maps cleanly to Myrhiza: a **sturdyref ≈ the manifest-declared,
content-addressed grant** ("this bundle may subscribe to topic X"), and the **live
reference ≈ the in-session WIT handle** the kernel mints when the app actually
subscribes. Reconnect = re-enliven the same grant into a fresh live handle. The
swiss-number is an unguessable secret in the URI — Myrhiza's topic IDs are already
content-addressed BLAKE3, but the *grant* still needs an unforgeable component so a
revoked grant can't be re-enlivened.

## Promises in CapTP

Both Cap'n Proto and CapTP let you message the *result of a not-yet-resolved call*
(promise pipelining). For streaming subscriptions this is mostly relevant as the
mechanism by which `enliven` returns immediately (a promise) without a round-trip.

## Implications for Myrhiza

- **The kernel is the caretaker.** The WIT handle the app holds is the forwarder;
  the kernel holds the enable slot. Revoke = kernel flips the slot and the next
  `host.*` call on that handle fails — no app cooperation needed. This is strictly
  better than Goblins because the trust boundary already exists.
- **Two-layer model**: durable grant (sturdyref-like, manifest+content-addressed,
  survives restart) vs live handle (cheap integer-indexed, dies with the session).
  Revocation must be expressible at *both* layers — kill the live handle now, and
  refuse future enlivening of the grant.
- **Avoid membranes**: keep subscription handles leaf capabilities (they deliver
  data, they don't mint further caps) so single-caretaker revocation suffices.

## Sources

- Caretaker pattern + Redell 1974 attribution: <https://kidneybone.com/c2/wiki/CaretakerPattern>
- Mark Miller, *Robust Composition* (2006), §Selective Revocation: <http://www.erights.org/talks/thesis/markm-thesis.pdf>
- Spritely Goblins CapTP API (sturdyrefs, enliven): <https://files.spritely.institute/docs/guile-goblins/0.16.1/Using-the-CapTP-API.html>
- Spritely Goblins CapTP protocol ("live references … incredibly cheap, merely represented as integers on each side"): <https://files.spritely.institute/docs/guile-goblins/0.16.1/CapTP-The-Capability-Transport-Protocol.html>
- Spritely core paper (revocable proxy, sealers/unsealers): <https://files.spritely.institute/papers/spritely-core.html>
- Sibling corpus: [`spritely-ocapn/`](../spritely-ocapn/README.md), [`agoric-endo/`](../agoric-endo/README.md)
