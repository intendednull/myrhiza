**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — consumer-mobile P2P stack (Hypercore + Hyperswarm + Bare + Pear runtime + Keet messenger), single-vendor-stewarded by Holepunch Inc, Tether-funded

# Pears / Holepunch

The Holepunch stack is the highest-volume *production-deployed* P2P-app codebase in our prior-art set. Three layered concerns live in one company:

- **The Hypercore stack** — `hypercore`, `hyperdrive`, `hyperbee`, `hyperswarm`, `hyperdht`, `autobase`. The data + transport substrate. Predates Holepunch by ~6 years (started 2015 under the Dat Project). **Mixed-permissive licensing**: `hypercore`, `hyperbee`, `hyperswarm`, `hyperdht` are MIT (Dat-era); `hyperdrive` and `autobase` are Apache-2.0 (Holepunch-era).
- **The Bare runtime** — embeddable JavaScript runtime, Holepunch's own (alternative to Node/Bun/Deno). Built on libjs + libuv. Mobile-embeddable. Apache-2.0. Created 2022-12-18.
- **The Pear runtime** — application runtime that uses Hyperdrive to distribute apps and Bare to run them. P2P "app store" by hash, no central servers, `pear://` link addressability. Apache-2.0. Created 2024-02-03 — the youngest major piece.
- **Keet messenger** — flagship app. **Closed-source** (no `keet-desktop` repo on `holepunchto`). iOS, Android, macOS, Windows, Linux. React Native + Expo + Bare-embedded; voice/video calls; invite-by-link, no accounts.

**Honest scale calibration.** Marketing positions Pears as consumer-scale P2P shipping. Verified Keet adoption is closer to **research-grade-but-shipping**: bundle id `io.keet.app`, current v4.14.0 (2026-04-29), 99 iOS ratings (4.59 stars), ~690K Android lifetime downloads, ~1K Android ratings — low-tens-of-thousands monthly-active-user class, not millions. Treat the corpus as the "what consumer-mobile P2P UX actually looks like under iOS/Android constraints" data point, not the "P2P at consumer scale" data point. The Holochain Volla folder ([`../holochain/`](../holochain/)) carries roughly the same scale; Iroh's Delta Chat ([`../iroh/`](../iroh/)) is a different lens (transport-only adoption inside an existing app).

This is not WASM. Pears is JavaScript end-to-end. The lessons we take are *design lessons + UX reality checks*, not API commitments. Myrhiza is WASM-Component-Model, peer-symmetric, capability-mediated; Pears is JS, Holepunch-stewarded, mostly capability-implicit.

## Key facts

| Fact | Value |
|---|---|
| Steward | Holepunch Inc (Mexico legal entity: Tether Data S.A. de C.V., per `electron-forge-maker-flatpak` README). Single-vendor; no foundation, no consortium |
| GitHub org | [`holepunchto`](https://github.com/holepunchto), created **2021-03-25**, **617 public repos** as of 2026-05-09 |
| Founders | Mathias Buus Madsen (CEO, ex-Dat Project), Paolo Ardoino (CSO; also Tether CEO + Bitfinex CTO), Andrew Osheroff |
| Public launch | **2022-07-25** (joint Tether/Bitfinex announcement; ~$10M committed initial + up to $50–100M follow-on per third-party crypto press; **primary source of dollar amount not findable — figure flagged as approximate**) |
| Funding model | Patient capital from Tether (the stablecoin issuer). No VC raise. No revenue model — Keet is free, all OSS substrate is permissive-licensed, no enterprise tier |
| License (mixed) | **MIT** for `hypercore`, `hyperbee`, `hyperswarm`, `hyperdht` (Dat-era). **Apache-2.0** for `hyperdrive`, `autobase`, `pear`, `bare` (Holepunch-era; `hyperdrive` was relicensed). Verified per-repo via `gh api repos/holepunchto/<name>/license` on 2026-05-09. Cross-repo mix surfaced in [governance.md](governance.md) |
| Lineage | Dat Project (2013, Max Ogden + Karissa McKelvey + Mathias Buus); Hypercore branches off ~2015; Holepunch absorbs the codebase ~2020–2021; `holepunchto` org created 2021-03-25 |
| Hypercore | `hypercore@11.30.1` (2026-05-06); 2,794 stars; MIT; repo created 2015-12-20. Protocol v11 introduced **RocksDB-backed storage** (replacing the v10-and-earlier `.metadata`/`.tree`/`.bitfield`/`.data` file layout) |
| Hyperdrive | `hyperdrive@13.3.2` (2026-03-27); 1,986 stars; **Apache-2.0** (Holepunch-era; not MIT — verified via `gh api repos/holepunchto/hyperdrive/license`). v13 layout = metadata-Hyperbee + Hyperblobs (NOT raw two-Hypercore as older docs describe) |
| Hyperbee | `hyperbee@2.27.3` (2026-01-27); 297 stars; MIT |
| Autobase | `autobase@7.28.0` (2026-05-05); 145 stars; **Apache-2.0**. Multi-writer linearization. README explicitly avoids "CRDT" terminology — calls it "multiwriter data structure + event sourcing pattern" |
| Hyperswarm | `hyperswarm@4.17.0` (2026-02-20); 1,261 stars; **MIT** (NOT Apache-2.0). Noise-**IK** handshake (NOT Noise-XX) |
| HyperDHT | `hyperdht@6.32.0` (2026-05-05); 391 stars; MIT. **Three hardcoded bootstrap nodes**: `node1.hyperdht.org`, `node2.hyperdht.org`, `node3.hyperdht.org`, all on port `49737` |
| `protomux` (multiplexer) | `3.11.0` (2026-05-05) |
| `dht-rpc` | `6.27.0` (2026-05-05) |
| `@hyperswarm/secret-stream` | `6.9.1` (2025-10-07) |
| Bare runtime | `holepunchto/bare`, **1,072 stars**, Apache-2.0, created 2022-12-18, current updated 2026-05-06; npm `bare@1.28.5` (2026-05-06). Built on libjs + libuv. C embedding API: `bare_setup`/`bare_load`/`bare_run`/`bare_teardown`/`bare_suspend`/`bare_resume` |
| Pear runtime | `holepunchto/pear`, 241 stars, Apache-2.0, created 2024-02-03. Install via `npx pear` (NOT via `pear-cli` npm — that's stale at v2.5.9 from 2022-02-14). Production bootstrap key `pear://gd4n8itmfs6x7tzioj6jtxexiu4x4ijiu3grxdjwkbtkczw5dwho` |
| Keet messenger | iOS App Store bundle id `io.keet.app` (NOT `to.holepunch.keet`); v4.14.0 published 2026-04-29; **99 iOS ratings, 4.59★**, originally released 2023-01-30, available in 16 languages, 181 MB; ~690K Android lifetime downloads; ~1K Android ratings. **Closed-source.** React Native + Expo + Bare-embedded (changelog evidence: `expo-blur`, `Expo background-task`, CallKit) |
| PearPass | Second showcase Pear app; built by **Tether Data** (separate Tether-affiliated entity, not Holepunch); App Store id `6752954830` |
| Other Holepunch tools | `hyperbeam` (terminal-sharing, 539 stars), `hyperssh` (156), `sodium-native` (331), `mininet` (117) |
| Production deployment relays | `blind-relay` is a real TURN-equivalent dependency in `hyperdht`, but **no default Holepunch-operated fleet** — opt-in via `relayThrough`. Bootstrap nodes ARE Holepunch-operated. The "no servers" marketing has caveats |

(All version numbers, dates, founders, App Store metadata, repo stars, and license claims verified via `gh api` / `crates.io` / `registry.npmjs.org` / `itunes.apple.com/lookup` / GitHub source-file reads on 2026-05-09. Note: my brief had three errors agents corrected in-place — Wasmer-style: hypercore-stack license = MIT not Apache-2.0; hyperswarm Noise pattern = IK not XX; Keet bundle id = `io.keet.app` not `to.holepunch.keet`.)

## Contents

15 files, ~3,370 lines (excluding this README and glossary). Each file independently skimmable.

**Runtime layer (Pear + Bare)**
- [**Pear runtime**](pear-runtime.md) — the application runtime: app distribution via Hyperdrive, `pear run pear://<key>`, `pear stage`, `pear://` link addressability, no central app store, hash-addressed apps, update mechanism via Hypercore-length versioning.
- [**Bare runtime**](bare-runtime.md) — Holepunch's embeddable JavaScript runtime. libjs + libuv. Tier 1 mobile platforms (iOS/Android). C embedding API (`bare_setup` / `bare_load` / `bare_run` / `bare_suspend` / `bare_resume`). The mobile path Keet uses.

**Data layer (Hypercore stack)**
- [**Hypercore stack**](hypercore-stack.md) — Hypercore (append-only signed log, protocol v11 with RocksDB storage), Hyperdrive (filesystem on metadata-Hyperbee + Hyperblobs), Hyperbee (sorted KV B-tree), Autobase (multi-writer linearization). Protocol versions v8 → v9 → v10 → v11.
- [**Data model**](data-model.md) — append-only-only mutation, single-writer-per-Hypercore + Autobase for multi-writer, sparse replication, performance shape (append O(log N), random read O(log N)), CRDT comparison.

**Network layer (Hyperswarm + transport)**
- [**Hyperswarm**](hyperswarm.md) — DHT-based peer discovery + UDP holepunching. Three hardcoded bootstrap nodes. Noise-IK handshake at the DHT layer. `BIRTHDAY_SOCKETS=256`, `HOLEPUNCH_TTL=5`, `connectionKeepAlive=5000`. UDX-over-UDP transport (no TCP fallback in current `hyperdht`). `blind-relay` opt-in TURN.
- [**Transport comparison**](transport-comparison.md) — direct compare-and-contrast with [Iroh](../iroh/). Hyperswarm runs servers in **control path only** (DHT bootstrap, opt-in `blind-relay`); Iroh runs DERP in **data path** (always-available HTTP-relay fallback). Both ed25519-keyed; both Noise-flavored; differ on relay strategy and language substrate.

**Apps + commercial**
- [**Keet and apps**](keet-and-apps.md) — Keet messenger end-to-end: bundle id `io.keet.app`, React Native + Expo + Bare, voice/video via WebRTC + Hyperswarm signaling (inferred from CallKit + new-call-engine changelog), 24-word seed identity, room-key-based access control, push relay design, **honest adoption numbers** (~tens-of-thousands MAU, not millions).
- [**Apps**](apps.md) — broader Pear-app ecosystem. PearPass (built by Tether Data, not Holepunch). Hyperbeam (terminal-sharing). Hypershell, Hypertele, Hyperssh, Drives. Research-grade rather than consumer-grade ecosystem flagged explicitly.
- [**Commercial**](commercial.md) — Holepunch Inc, Tether-funded ($10M committed + $50–100M follow-on per third-party press; primary source of figure not verified), founders Mathias Buus + Paolo Ardoino + Andrew Osheroff, public launch 2022-07-25, no revenue model.

**Project lens**
- [**Governance**](governance.md) — single-vendor steward, Apache-2.0 / MIT license mix across repos (Dat-era MIT, Holepunch-era Apache-2.0), `pear/CONTRIBUTING.md` verbatim quote "any pull-request or issue may be closed without explanation," Tether Data S.A. de C.V. as legal entity.
- [**History**](history.md) — Dat Project 2013 → Hypercore 2015 → Holepunch incorporates ~2020-2021 → org `holepunchto` 2021-03-25 → public launch 2022-07-25 → Keet iOS 2023-01-30 → Bare 2022-12-18 → Pear runtime 2024-02-03 → Hypercore protocol v11 (2025-01-13).
- [**Comparisons**](comparisons.md) — vs Iroh, Holochain, WASM CM + wasmCloud, Spritely OCapN, Agoric SwingSet, Automerge/Yjs CRDTs. Six 2-row tables. Closes with a "when to borrow what" summary.
- [**Critiques**](critiques.md) — bootstrap-node hardcoding (verified via `hyperdht/lib/constants.js`), `hyperswarm#194` mobile-battery thread verbatim, `hyperswarm#212` 2026-03-29 Russia-bootstrap-volunteer thread, `hyperswarm#47` six-years-open battery toggle, single-vendor governance risk, closed-source flagship, JS-only-no-typesafety, Hypercore protocol-version churn (v8→v9→v10→v11 not all backward-compatible).
- [**Open problems**](open-problems.md) — 8 structurally unresolved questions Myrhiza inherits if borrowing Pears patterns: identity portability (Hypercore key = device), iOS push-notifications without a server, multi-author conflict resolution at the application layer, Sybil resistance at the DHT topic layer, storage growth in append-only-logs, encryption-key rotation, browser-side parity, determinism (JS execution).

**Reference**
- [**Lessons for Myrhiza**](lessons.md) — validates / avoid / borrow — **the consult-this-when-designing file**. 11-row Validates, 12-row Avoid, 5-section Borrow with 26 concrete techniques. Decision tree at the end.
- [**Glossary**](glossary.md) — Pear runtime, Bare runtime, Hypercore, Hyperdrive, Hyperbee, Autobase, Hyperswarm, HyperDHT, blind-relay, Noise-IK, `pear://` link, `protomux`, room key, etc.

## Recommended reading order

For a Myrhiza spec author working on **the kernel-network-cap (transport/discovery)**: start with [**lessons.md**](lessons.md), then [**transport-comparison.md**](transport-comparison.md) (the direct Iroh-vs-Hyperswarm contrast), then [**hyperswarm.md**](hyperswarm.md) for Hyperswarm internals. The takeaway: Iroh's DERP-in-data-path is the engineering bet that pure holepunching tail-latency on hard NATs is too costly; Hyperswarm validates that pure holepunching CAN ship at consumer-mobile scale, but only with extensive retry machinery (256 birthday-sockets, multi-second timeouts, suspend/resume hooks).

For a spec author working on **state-apply event log + replication**: [**lessons.md**](lessons.md), then [**hypercore-stack.md**](hypercore-stack.md) and [**data-model.md**](data-model.md). Hypercore is the closest production-shipping shape for what we want from the event log substrate — append-only, signed-by-author, sparse-replicated, deterministic-merge-via-Autobase. The catch: it's JS-only with no WASM-side bindings.

For a spec author working on **app-bundle distribution**: [**pear-runtime.md**](pear-runtime.md) for the `pear://`-link + Hyperdrive distribution model, then `[../wasm-component-model/tooling.md](../wasm-component-model/tooling.md)` (OCI-as-component-registry alternative), then `[../agoric-endo/modules-and-bundling.md](../agoric-endo/modules-and-bundling.md)` for the bundle-hashing comparison.

For a spec author working on **mobile UX**: [**bare-runtime.md**](bare-runtime.md) (Bare's mobile-binary discipline), [**keet-and-apps.md**](keet-and-apps.md) (Keet engineering specifics: 24-word seed, push relay, CallKit, suspend/resume), [**critiques.md**](critiques.md) §battery (the `hyperswarm#47` six-years-open issue is the operative reality). The honest framing: mobile-shipping P2P is mostly UX engineering, not protocol engineering.

For anyone evaluating "should we adopt Hypercore as our event-log substrate": [**hypercore-stack.md**](hypercore-stack.md), [**data-model.md**](data-model.md), [**critiques.md**](critiques.md) §protocol-version-churn, [**open-problems.md**](open-problems.md) §identity-portability.

## How to use this prior-art doc

This corpus is reference for future Myrhiza spec writing. Pin numbers and dates accurate as of the **Date:** in this README; bump the date when meaningful churn happens upstream (next Hypercore protocol version, next Pear major, Keet major release, Holepunch governance change, Tether stewardship shift).

**Framing disclosure.** These docs are written from a WASM-Component-Model, peer-symmetric, capability-mediated-host-imports stance — most "Implications for Myrhiza" sub-sections frame Pears' choices through that lens. Pears chose JavaScript, single-vendor stewardship, and consumer-mobile as deployment substrate; we choose WASM, peer-symmetric, and (initially) desktop/server with mobile via Bare-style embedded host. Future readers auditing whether *those* choices are themselves right should weigh the corpus accordingly: it's a learn-from-Pears-into-Myrhiza artifact, not a neutral catalog. The Spritely / Agoric / Holochain / wasmCloud / WASM-CM folders carry the same disclosure for the same reason.

**Honest-scale disclosure.** "Pears is the consumer-mobile-P2P-shipped-at-scale data point" is the framing this folder was built around. The verified scale (Keet at low-tens-of-thousands MAU class) is more modest than the marketing. Treat Pears as the *highest-volume P2P-app codebase we have*, not as Discord/Telegram-class consumer scale. The lesson density doesn't drop; the "validation" framing does.

**Not a tutorial.** Upstream documentation (`docs.pears.com`, `github.com/holepunchto/<repo>` READMEs) is the right source for hands-on use. This folder is the curated, version-pinned, Myrhiza-perspective synthesis those docs do not provide.

## Sources

- Pears documentation: https://docs.pears.com/
- Holepunch GitHub org: https://github.com/holepunchto
- Pear runtime: https://github.com/holepunchto/pear
- Bare runtime: https://github.com/holepunchto/bare
- Hypercore: https://github.com/holepunchto/hypercore
- Hyperswarm: https://github.com/holepunchto/hyperswarm
- HyperDHT: https://github.com/holepunchto/hyperdht
- Keet: https://keet.io/
- Keet App Store metadata: https://itunes.apple.com/lookup?bundleId=io.keet.app
- Tether/Bitfinex 2022-07-25 launch announcement: https://tether.to/en/tether-and-bitfinex-launch-keet/ (verify on access)
- Dat Project (archived): https://github.com/datproject/dat
