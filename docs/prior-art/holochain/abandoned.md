# Abandoned features

Holochain has a long history of features that shipped, partially shipped, or were widely promised and later removed or stalled. The pattern is informative for any new P2P runtime: the runtime is a research artifact and the publicly-visible surface area shifts faster than the marketing.

This file is a non-exhaustive timeline. Each entry: what it was, when it was attempted, when it was dropped, why, and whether a replacement is planned.

## DPKI / DeepKey

**What.** A system hApp providing distributed public key infrastructure: cross-app and multi-device key management with M-of-N change rules, key rotation, hierarchical seed derivation from a root seed, and a query API ("is this pubkey still the active key for its keyset?") usable by every other hApp.

**Timeline.** First sketched in 2018 ([dpki#1](https://github.com/holochain/dpki/issues/1)). Multiple rewrites over 2019–2023; the original `holochain/dpki` repo deprecated in favor of `holochain/deepkey`. Shipped behind the `unstable-dpki` compile-time feature flag in 0.4 (December 2024) ([Upgrade 0.3 → 0.4](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4)). Removed entirely from `conductor-config.yaml` in 0.6 (2025) ([Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)).

**Why dropped.** No canonical post-mortem. The 0.6 release notes document removal without explaining it. Reading across [Dev Pulse 153](https://blog.holochain.org/dev-pulse-153-holochain-0-6-released-with-immune-system/) and the 0.4 framing of "shrink to a stable core": DeepKey was a system hApp every conductor had to install, every Holochain upgrade had to migrate it, and the engineering bandwidth went to networking (iroh transport, Kitsune2) and the warrant-based "immune system" instead. After roughly seven years the team chose reliability over cross-app identity.

**Replacement.** None. Per-app pubkey is the current model. Multi-device, key rotation, and cross-app linking are app authors' problem.

## `lib3h`

**What.** The original Rust networking library for Holochain — a reimplementation of n3h's protocol in Rust. Intended to be the production peer-to-peer transport.

**Timeline.** Active development 2018–2019. Never reached production. The team adopted an "iteratively pull in aspects of lib3h as they're ready" approach rather than landing it whole ([Sim2h blog post, Nov 2019](https://blog.holochain.org/sim2h-holochains-simple-switch-board-networking/)).

**Why dropped.** Too much complexity to land in one go. Replaced by a sequence of simpler centralized switchboards (sim1h → sim2h) used during the redesign, then by the new RSM ("Refactor State Management") networking based on Kitsune over QUIC.

**Replacement.** Kitsune (Holochain RSM, 2020+), then Kitsune2 with the iroh transport in 0.6.1+.

## `sim2h`

**What.** A "centralized switchboard" replacement for direct peer-to-peer networking during development. Nodes connected to a central in-memory router that brokered messages, using lib3h's websocket layer for the wire format ([sim2h](https://github.com/holochain/sim2h); [blog post](https://blog.holochain.org/sim2h-holochains-simple-switch-board-networking/)).

**Timeline.** Introduced November 2019 as a successor to `sim1h` (which used centralized DynamoDB persistence). Used for ~12 months as the de-facto test transport.

**Why dropped.** Always meant as a stepping stone. The 2020 RSM rewrite ("Announcing and Unpacking the New Holochain") replaced it with rrDHT-based peer routing, where nodes maintain their own routing tables instead of routing through a central switchboard ([A Big Leap Forward, 2020](https://medium.com/h-o-l-o/a-big-leap-forward-for-holochain-holo-2efaaa54ed08)). Marked obsolete and unmaintained in the rust-redux changelog.

**Replacement.** Kitsune (rrDHT, QUIC).

## `n3h`

**What.** Node.js implementation of the Holochain networking protocol — the very first networking layer, predating both lib3h and sim2h ([n3h](https://github.com/holochain/n3h)).

**Timeline.** 2018–2019. Used by the Go prototype and early Rust prototype. Deprecated alongside sim1h when sim2h shipped, marked "obsolete and unmaintained" in the rust-redux change log.

**Why dropped.** Wrong language for an in-process peer-to-peer engine; required a Node child process for every conductor.

**Replacement.** lib3h → sim2h → Kitsune.

## Mobile (early Tauri, p2p Shipyard, Kangaroo)

**What.** Holochain runs a native conductor; making that work on Android/iOS was a multi-year project.

**Timeline.**
- 2019–2022 — multiple Tauri/Cordova attempts; none shipped a usable mobile path. Holochain's roadmap has listed "mobile" since 2019.
- 2024 (July) — `darksoil studio` published the **p2p Shipyard**, a Tauri+Nix toolchain that bundles Holochain + a hApp + a UI into Android APKs and platform desktop bundles ([p2p Shipyard intro](https://darksoil.substack.com/p/introducing-the-p2p-shipyard); [Holochain 0.3 + HC on Mobile](https://blog.holochain.org/holochain-0-3-a-new-launcher-and-hc-on-mobile/)).
- Same window — `kangaroo-tauri` (an "official" repo under `holochain-apps`) provides a simpler Tauri-only desktop wrapper for hApps ([kangaroo-tauri](https://github.com/holochain-apps/kangaroo-tauri)).

**Why a mess.** Mobile was treated as "downstream packaging" rather than a runtime concern. Two parallel solutions still exist (Kangaroo for desktop, Shipyard for mobile+desktop), with overlapping scope.

**Status.** Not abandoned but a cautionary tale: source-available with a $100k retroactive-crowdfunding gate before open-sourcing the Shipyard. iOS support is "coming soon" as of 2025. Volla Quintus phone ships with a Holochain stack; the only mainstream mobile deployment.

## Non-Rust HDK ports

**What.** Holochain DNAs are WASM modules — in principle, any language compiling to WASM with serde-compatible serialization can author them. Two ports were attempted.

- **Go HDK.** The original 2017–2018 prototype was *written in* Go (`holochain/holochain-proto`). Not an HDK port — the entire conductor was Go. Abandoned in late 2018 in favor of the Rust rewrite ([Architecture README](https://github.com/holochain/holochain-rust/blob/develop/doc/architecture/README.md)).
- **AssemblyScript HDK.** `holochain/hdk-assemblyscript` was opened to give web developers a JavaScript-like authoring path. README marked "experimental" and "partially blocked by lack of JSON parsing in AssemblyScript" ([hdk-assemblyscript](https://github.com/holochain/hdk-assemblyscript)). No commits in years.

**Why dropped.** AssemblyScript ergonomics weren't ready; no community demand strong enough to push through. The HDK macros (`hdk_extern`, `hdk_entry_helper`) are deeply Rust-specific (`derive`, attribute macros, type-level invariants) so a port is closer to "redesign the whole authoring API" than "compile to the same WASM ABI."

**Replacement.** None. Rust is the only HDK.

## HoloFuel & the HOT token

**What.** HoloFuel is the mutual-credit cryptocurrency of the Holo hosting marketplace: hosts get paid HoloFuel for hosting, end users pay HoloFuel for hosted hApp use. The 2018 ICO sold an ERC-20 placeholder, **HoloToken (HOT)**, with a 1:1 swap to HoloFuel promised at launch ([HOT and HoloFuel](https://medium.com/h-o-l-o/holos-erc20-token-hot-and-mutual-credit-cryptocurrency-holo-fuel-6d8b6d3938d6); [Holo currency page](https://holo.host/currency/)).

**Timeline.** Original swap deadline: launch of HoloFuel. Re-targeted to Q2 2024. Slipped through 2024 and 2025; as of 2025 there is still no scheduled date ([What is HOT?](https://holo.host/faq/what-is-hot-when-can-i-swap-it-for-holo-fuel/)).

**Why stalled.** HoloFuel is itself a hApp. It depends on Holochain stability, on HoloPort hosts to mediate it, and on a finished hosting marketplace. The pivot away from HoloPort-as-hosting (toward Allograph cloud nodes — see below) effectively pulled the rug out from under the original HoloFuel design. After a leadership change at end of 2025 the roadmap is "in the process of getting clarified" ([2025 Year in Review](https://holo.host/blog/2025-year-in-review-the-year-we-built-the-edge-XqpCNKmMRVh/)).

**Replacement.** Unyt is now mentioned as the economy/billing layer. HoloFuel's status is unclear; the swap is "not mandatory" in current FAQ language, suggesting holders can keep HOT indefinitely.

## Light client / WASM conductor

**What.** A Holochain conductor running in the browser — agents fully in-page, no native install. On the roadmap since 2019 ([WASM Conductor and Light Client groundwork](https://blog.holochain.org/the-groundwork-for-the-wasm-conductor-and-light-client/)).

**Timeline.** PR #894 against `holochain-rust` in 2019 added a "WASM Container Skeleton". Never made it to functional. The redux rewrite shelved the work. As of 2025, no browser-native conductor.

**Why stalled.** A full conductor wants long-running native sockets, persistent storage, libsodium, an out-of-process keystore — the browser has none of these natively, and a WASM port would have to replace each. Each replacement is its own subproject. Holo's pivot to an HTTP "Web Bridge" (Q1 2025) sidesteps the problem: web users hit a hosted node over HTTP rather than running a conductor ([Cloud Nodes + Web Bridge](https://press.holo.host/248711-introducing-cloud-nodes-web-bridge-for-holochain-applications)).

**Replacement.** Web Bridge (HTTP gateway to a hosted node) for read-mostly use cases. No browser-native peer.

## Cross-DNA composition (bridging)

**What.** In Holochain-Redux (pre-RSM), the conductor configuration could declare named "bridges" between DNAs running on the same conductor; a zome in DNA A could call a zome in DNA B by bridge name. Bridges were a first-class conductor primitive.

**Timeline.** 2019–2020 in `holochain-rust`. Removed in the RSM rewrite. The ecosystem replacement is `dna-auth-resolver`, which the docs describe as "replaces what was formerly known as 'bridging' in Holochain-Redux" ([dna-auth-resolver](https://github.com/holochain-open-dev/dna-auth-resolver)).

**Why dropped.** The conductor-config approach didn't scale to the RSM model where DNAs are dynamically installable. Cross-DNA calls now go through the same `call` host function as local zome calls but addressed by `CellId`.

**Replacement.** `call(CellId, zome, fn, payload)` and `call_remote` for cross-network. No runtime-level "this app is composed of DNAs A+B" primitive — composition is by app-level convention. See [open-problems.md §7](open-problems.md#7-cross-dna-discovery-and-group-identity).

## Holo Host (the hosting business)

**What.** The original Holo Inc. business model: a marketplace of HoloPort hardware devices (small ARM appliances) hosting hApps for end users who pay in HoloFuel. The HOT ICO funded this.

**Timeline.**
- 2018 — ICO, HoloPorts pre-sold.
- 2019–2021 — HoloPorts shipped to backers; "Alpha" hosting network running.
- 2022–2024 — slow uptake, persistent reliability issues with the Legacy network.
- 2025 — pivot to **Allograph**, an OCI-container-based cloud-node platform that hosts both Holochain workloads and conventional services. HoloPorts being migrated off the Legacy network onto Allograph. Static-site hosting added as a product. Three-entity restructure (Holochain Foundation, Unyt, Holo Host) ([Holo Forward](https://holo.host/blog/holo-forward-Cf5h1g8UhaZ/); [2025 Year in Review](https://holo.host/blog/2025-year-in-review-the-year-we-built-the-edge-XqpCNKmMRVh/)).
- End of 2025 — leadership change; roadmap "in the process of getting clarified."

**Status.** Not abandoned but substantially different from the 2018 vision. HoloPort-as-edge-hardware persists; the marketplace economics, the consumer-facing hosting product, and the HoloFuel payment rail are all in flux. The "decentralized AWS" framing of 2018 has narrowed to "containers on community hardware plus a static-site service."

**Implication for Myrhiza.** A runtime that needs an end-user payment story should not assume the Holochain-adjacent payment story will be ready when the runtime is. Build the runtime to be useful without the marketplace, and let the marketplace catch up if it does.

## Pattern across the list

Three recurring failure modes show up:

1. **Cross-cutting system features are hardest.** DPKI, bridging, the WASM conductor, mobile — every one of these touches conductor + lair + networking + DNA ABI at once. Any one of those layers churning kills the cross-cutting feature. Networking churned for seven years.
2. **"Behind a feature flag" is a polite shelf.** DPKI lived behind `unstable-dpki` for one release before being deleted. Sharding has lived behind `unstable-sharding` for longer and shows the same trajectory.
3. **Marketing precedes shipping by years.** HoloFuel, the WASM conductor, and DPKI were all named features of "Holochain" in marketing materials for half a decade before being dropped. Rule for Myrhiza: don't name a feature publicly until its first end-to-end version compiles.

## Sources

- [Upgrade 0.5 → 0.6 (DPKI removed)](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
- [Upgrade 0.3 → 0.4 (DPKI behind flag)](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.4)
- [Dev Pulse 153: Holochain 0.6 with Immune System](https://blog.holochain.org/dev-pulse-153-holochain-0-6-released-with-immune-system/)
- [DeepKey design doc (2023)](https://github.com/holochain/deepkey/blob/main/docs/2023/README.md)
- [DPKI Initial Idea (2018)](https://github.com/holochain/dpki/issues/1)
- [holochain#4138 (DPKI key update)](https://github.com/holochain/holochain/issues/4138)
- [Sim2h: Holochain's Simple "switch-board" Networking (2019)](https://blog.holochain.org/sim2h-holochains-simple-switch-board-networking/)
- [sim2h repo](https://github.com/holochain/sim2h)
- [n3h repo](https://github.com/holochain/n3h)
- [A Big Leap Forward for Holochain & Holo (RSM rewrite, 2020)](https://medium.com/h-o-l-o/a-big-leap-forward-for-holochain-holo-2efaaa54ed08)
- [Holochain 0.3, a new Launcher, and HC on Mobile](https://blog.holochain.org/holochain-0-3-a-new-launcher-and-hc-on-mobile/)
- [Introducing the p2p Shipyard](https://darksoil.substack.com/p/introducing-the-p2p-shipyard)
- [kangaroo-tauri](https://github.com/holochain-apps/kangaroo-tauri)
- [hdk-assemblyscript](https://github.com/holochain/hdk-assemblyscript)
- [Holochain architecture README (Go history)](https://github.com/holochain/holochain-rust/blob/develop/doc/architecture/README.md)
- [HOT and HoloFuel](https://medium.com/h-o-l-o/holos-erc20-token-hot-and-mutual-credit-cryptocurrency-holo-fuel-6d8b6d3938d6)
- [What is HOT?](https://holo.host/faq/what-is-hot-when-can-i-swap-it-for-holo-fuel/)
- [Holo currency page](https://holo.host/currency/)
- [The Groundwork for the WASM Conductor and Light Client](https://blog.holochain.org/the-groundwork-for-the-wasm-conductor-and-light-client/)
- [Cloud Nodes + Web Bridge for Holochain Applications](https://press.holo.host/248711-introducing-cloud-nodes-web-bridge-for-holochain-applications)
- [dna-auth-resolver (replaces bridging)](https://github.com/holochain-open-dev/dna-auth-resolver)
- [Holo Forward](https://holo.host/blog/holo-forward-Cf5h1g8UhaZ/)
- [2025 Year in Review](https://holo.host/blog/2025-year-in-review-the-year-we-built-the-edge-XqpCNKmMRVh/)
