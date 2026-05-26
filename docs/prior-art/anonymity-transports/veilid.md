**Date:** 2026-05-22
**Status:** active
**Subject:** Veilid — a modern DHT-based P2P anonymity framework from Cult of the Dead Cow. The closest stylistic parallel to Myrhiza's "P2P-runtime + anonymity by default" stack — and a cautionary lesson in research-grade vs production scale.

# Veilid — DHT-based P2P with onion-routing-flavored privacy

Unveiled at **DEF CON 31** on **2023-08-11** by **Christien "DilDog"
Rioux** and **Katelyn "Medus4" Bowden** of the **Cult of the Dead
Cow** (cDc). The pitch from Rioux's DEF CON talk: *"like Tor, but for
apps."* The project had been in development for **~3 years** before
the public unveiling.

## What it is

Veilid is a **peer-to-peer application framework** that bundles three
things together:

1. **Anonymized message routing** — onion-routing-like layered
   encryption through volunteer-operated nodes, with separate
   sender-privacy and receiver-privacy primitives.
2. **A distributed hash table** for both peer discovery and an app-
   level key-value store. (Conceptually closer to **Mainline DHT /
   Kademlia** than to Tor.)
3. **A high-level developer API** in Rust core + Python + Dart
   bindings — apps written on Veilid get the routing + DHT primitives
   without needing to understand the underlying crypto.

Veilid markets itself as *Tor + IPFS in one framework*. That framing
is roughly accurate as a sketch; in detail Veilid is its own design,
not a literal composition.

## Key facts

| | |
|---|---|
| **Stewardship** | **Veilid Foundation** — US 501(c)(3) nonprofit |
| **Lead developers** | DilDog (Christien Rioux) + Medus4 (Katelyn Bowden) |
| **Affiliation** | **Cult of the Dead Cow** — 40-year-old hacktivist group; DilDog also of BO2K fame |
| **Language** | **Rust** (core) + Dart, Python bindings |
| **License** | **MPL-2.0** (Mozilla Public License 2.0) |
| **Repository** | <https://gitlab.com/veilid/veilid> |
| **Crate** | `veilid-core` — current **0.5.3** (published **2026-03-23**) |
| **Public unveiling** | 2023-08-11 (DEF CON 31, Las Vegas) |
| **Funding model** | Donations to the Foundation; no token, no VC |
| **Platforms** | Linux, macOS, Windows, Android, iOS, WebAssembly (browser) |
| **Flagship app** | **VeilidChat** (E2E messaging) |

## Architecture

### Safety Routes (sender privacy) + Private Routes (receiver privacy)

Veilid's most distinctive design choice: **two separable
onion-style primitives that compose**.

**Safety Route** — built by the *sender*. A sequence of hops the
sender chooses, layer-encrypted such that each hop sees only the next
hop. Hides the sender's IP and node ID from the destination and from
intermediate observers. Conceptually equivalent to the **client side
of a Tor circuit**.

**Private Route** — published by the *receiver*. A sequence of hops
that lead from a publicly known entry node to the receiver. The
*destination identity* is hidden from the sender — the sender only
knows the entry node and the layered keys, not the final hop. The
sender attaches its Safety Route's outbound side to the published
entry of the Private Route.

**Compiled Route** = Safety Route + Private Route concatenated. By
default Veilid uses 1+1 = **3 total hops once compiled** (one chosen
by the safety route, one by the private route, plus the receiver's
edge node). The default is configurable; users wanting stronger
anonymity can pick longer routes at the cost of latency.

This is **fundamentally similar to Tor hidden services** (sender's
client circuit + receiver's introduction-point chain rendezvousing
at a meeting point). The differences:

| | Tor v3 hidden services | Veilid |
|---|---|---|
| Total hops (default) | 6 (3+3+rendezvous) | 3 (1+1+receiver) |
| Sender circuit chosen by | Tor circuit builder | Sender's `RoutingContext` |
| Receiver circuit chosen by | Service-published descriptor | Receiver's published Private Route |
| Identifier hides receiver | Yes (descriptor-blinding) | Yes (Private Route is opaque) |
| Identifier rotates | Daily (descriptor-blinding) | Per-route lifetime (configurable) |
| Cover traffic | None by default | None by default |

**Known weakness:** Veilid GitLab issue [#395 *"Privacy issue: Private
routes can deanonymize Safety Routes"*][veilid-395] documents an
attack where a malicious destination can use timing correlation
between its Private Route hops to narrow the Safety Route hop set.
The issue has been open since the early releases; it has not been
fully closed at the time of writing. **This is a real, acknowledged
limitation.**

[veilid-395]: https://gitlab.com/veilid/veilid/-/issues/395

### Distributed hash table

Veilid's DHT is **Kademlia-style**, used for two purposes:

1. **Peer discovery** — node IDs are routed via XOR-distance buckets,
   the standard Kademlia mechanism.
2. **Key-value records** — apps store and retrieve typed records by
   key. As of 0.5.x, records support **encryption by default**, **DHT
   transactions** (atomic multi-key operations), and writer-set
   controls. The 0.5.0 changelog highlights "automatic default record
   encryption" as a recent improvement.

DHT operations are routed through the same Safety/Private Route
primitives — the **DHT itself runs over the privacy layer**, not as a
parallel un-anonymized service.

### Node identity and crypto

Node IDs are **Ed25519 public keys**. Wire-format encryption uses
**XChaCha20-Poly1305** for the AEAD. The Veilid handshake is
described in the Foundation's "How It Works" document as Sphinx-like
(layer-encrypted packets) — not literally Sphinx (the Loopix /
Nym packet format) but the same construction-shape.

### Platform support

Veilid runs as a **single Rust crate** (`veilid-core`) that compiles
for:

- Linux / macOS / Windows / Android / iOS (native via FFI).
- **WebAssembly + `wasm-bindgen-futures`** for browser apps.

The browser path is what makes Veilid stylistically interesting for
Myrhiza — it is one of very few P2P anonymity stacks that runs
natively in a browser via WASM, not via a desktop SOCKS proxy.

## Current state — honest scale

**As of 2026-05, Veilid is research-grade, not production-deployed
at scale.**

- The `veilid-core` crate is on **0.5.3**, pre-1.0. The 0.4 → 0.5
  jump in late 2025 was a substantial API churn, not a minor bump.
- **VeilidChat**, the flagship application, is functional but has
  small user numbers. Verifying scale via app-store metrics is hard
  (VeilidChat ships on F-Droid and direct downloads, not Play /
  iTunes); the Foundation has not published MAU figures.
- The release cadence has been **steady** since the 2023 unveiling
  — versions 0.4.6 (May 2025), 0.4.7 (Jun), 0.4.8 (Aug), 0.5.0 (Dec
  2025), 0.5.2 (Jan 2026), 0.5.3 (Mar 2026). This signals an active
  but small team.
- Security audits: **none publicly disclosed** as of writing.
- Third-party formal analysis: **none.**

**Compare with Tor:** Tor has ~7,000 relays, ~2-4M daily users, ~700K
v3 onion services, ~25 years of cryptographic and operational
literature. Veilid has none of those. **Veilid is much closer to
"early-Tor in 2003" than to "production Tor in 2026."**

## Why Veilid is interesting for Myrhiza specifically

### Parallel stylistic commitments

Myrhiza's design commitments overlap with Veilid's in ways no other
project's do:

1. **Capability-mediated apps over an anonymized transport.** Veilid
   is closer to this than Tor (which is a transport with no app
   model), libp2p (which is a transport with no anonymity), or
   Holochain (which is a per-app DHT with no transport anonymity).
2. **Rust core with WASM browser path.** Same as Myrhiza's Component
   Model commitment.
3. **No token, no VC.** A nonprofit-foundation steward, donation
   funded. Removes the "investors push token narratives" failure
   mode that plagues Nym and HOPR.
4. **DHT + anonymity in one stack.** Apps don't compose "Tor + IPFS"
   themselves; the framework gives both.

### Where Veilid is ahead

- **Receiver privacy as a first-class primitive.** Tor's hidden
  services do this but require a rendezvous-point negotiation that
  is operationally heavy. Veilid's Private Route is a single
  publishable artifact that a sender attaches their Safety Route
  to — simpler API.
- **Browser-native** via WASM. Tor in a browser means running Tor
  Browser; arti-in-WASM is not production. Veilid has a WASM build
  today.
- **API design is application-oriented**, not transport-oriented.
  `RoutingContext::with_privacy(SafetySelection::Safe)` is a
  capability-flavored API.

### Where Veilid is behind

- **Scale.** The Tor network's anonymity set is ~2-4M daily users.
  Veilid's anonymity set is ~hundreds-to-low-thousands. **Small
  anonymity sets are not anonymity** — a global adversary can
  intersect efficiently.
- **No cover traffic** by default. Same as Tor — pattern privacy is
  not addressed.
- **The Private-Route-deanonymizes-Safety-Route issue (#395) is
  open.** A documented, acknowledged correlation attack exists.
- **No published wire-protocol spec separate from the implementation.**
  Veilid is a single-implementation protocol; if the cDc team
  stops, there is no third-party reference to read against.
- **Single research team.** ~2-5 core developers. Bus-factor risk
  is real.

## Implications for Myrhiza

- **The Safety Route + Private Route split is borrowable.** Decoupling
  "sender hides their IP" from "receiver hides their identity" gives
  applications fine-grained control, and the API shape
  (`RoutingContext::with_privacy(...)`) maps cleanly onto a
  capability surface.
- **Do not bet Myrhiza production on Veilid as the default transport
  *yet*.** Anonymity set too small, audit posture too thin, single-
  team risk too high. But:
- **Veilid is the right partner for the "Myrhiza-on-Veilid" plug-in
  path.** Same Rust + WASM stack, same nonprofit-friendly licensing,
  same architectural posture. If Myrhiza ships a custom-transport
  API (per [`iroh/lessons.md`](../iroh/lessons.md)), Veilid is
  cheaper to integrate against than Tor or I2P.
- **Watch issue #395 and Veilid's audit posture.** Until those
  resolve, treat Veilid as "promising but unproven."
- **Mirror Veilid's governance model.** Foundation-owned, donation-
  funded, no token. This is the model that Myrhiza spec authors
  should hold up as a counter-example to Nym/HOPR's
  investor-narrative pressure.

## Repo and sources

- Main repo: <https://gitlab.com/veilid/veilid>
- Foundation: <https://veilid.org/>
- VeilidChat repo: <https://gitlab.com/veilid/veilidchat>
- `veilid-core` crate: <https://crates.io/crates/veilid-core>
- Developer book: <https://veilid.gitlab.io/developer-book/>
- DEF CON 31 talk: <https://www.youtube.com/watch?v=Kb1lKscAMDQ>

## Sources

- *Cult of the Dead Cow unveils Veilid peer-to-peer project* — The Register, 2023-08-12: <https://www.theregister.com/2023/08/12/veilid_privacy_data/>
- *The Internals of Veilid* — DEF CON 31 talk by Rioux + Bowden: <https://forum.defcon.org/node/246124>
- Veilid official site: <https://veilid.com/>
- Veilid Foundation site: <https://veilid.org/>
- Veilid "How It Works" — Private Routing: <https://veilid.com/how-it-works/private-routing/>
- `veilid-core` crate on docs.rs: <https://docs.rs/crate/veilid-core/latest>
- GitLab issue #395 (Private-route deanonymizes Safety-route): <https://gitlab.com/veilid/veilid/-/issues/395>
- GitLab issue #339 (Routing-context safety defaults): <https://gitlab.com/veilid/veilid/-/issues/339>
- Cult of the Dead Cow — Wikipedia: <https://en.wikipedia.org/wiki/Cult_of_the_Dead_Cow>
