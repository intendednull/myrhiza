**Date:** 2026-05-09
**Status:** active
**Subject:** Multisynq Network — reflectors, sessions, API keys, pricing, SDK distribution, self-hosting

Sibling notes: [`glossary.md`](./glossary.md) · [`architecture.md`](./architecture.md) · [`determinism.md`](./determinism.md) · [`governance.md`](./governance.md) · [`comparisons.md`](./comparisons.md) · [`lessons.md`](./lessons.md) · [`programming-model.md`](./programming-model.md)

This note covers the *operational* side of Croquet's successor: how an app developer actually obtains synchronization service, who runs it, what it costs, and what the path off the legacy network looks like. Myrhiza is P2P with no central reflector, so the relevant question is whether Multisynq's reflector network is reproducible — what pieces are open, what pieces are not.

## 1. What "Multisynq Network" is

Multisynq operates a global mesh of *synchronizers* (the renamed reflector). A synchronizer is a stateless event-router that:

- Receives View→Model events from clients in a session.
- Stamps them with a deterministic order and a virtual-time tick.
- Echoes the ordered stream to every client in that session.
- Stores periodic encrypted snapshots and serves them to joiners.

The synchronizer never sees plaintext application data — payloads are end-to-end encrypted with the session password. This is the architectural premise that makes "the server is just a sequencer" defensible.

## 2. Reflector / synchronizer geography

The README states the network "automatically selects a server close to the first connecting user in a session" (multisynq-client README). Beyond that, the public material does not enumerate PoP locations or reflector counts. Multisynq describes itself as a DePIN (Decentralized Physical Infrastructure Network): operators ("Synqers") run synchronizer software on their own hardware and the network routes sessions to them. The total operator count is not published in the docs; the marketing material gestures at "thousands" without a confirmed number *(unverified)*.

## 3. API keys + session scoping

A session is identified by the tuple:

```
(apiKey, appId, name, password, code-hash)
```

- **`apiKey`** — issued per developer at `multisynq.io/coder`. Free to register. Routes traffic to the operator network and is used for billing/quota.
- **`appId`** — author-chosen reverse-DNS string (e.g. `io.multisynq.multiblaster`).
- **`name`** — session name; if omitted, the SDK invents a random one (typically encoded into the URL).
- **`password`** — gates session access *and* serves as the symmetric key for end-to-end encryption. Without it you cannot decrypt the snapshot or interpret the event stream, even if you reach the synchronizer.
- **`code-hash`** — automatically derived from the Model class source plus `Multisynq.Constants`. Any change forks a new session; old peers cannot accidentally talk to new code.

## 4. Pricing model

The free tier is real and fairly generous: registering at `multisynq.io/coder` issues an API key with no upfront cost; the README and tutorials assume free-tier usage. A paid tier is referenced for production / higher-throughput usage, but the public pricing page returned 403 to automated fetches at the time of writing — current numeric tiers should be confirmed against `multisynq.io/pricing` directly *(unverified)*. Multisynq is concurrently running a points-and-tokens incentive layer ("Multipoints") for synchronizer operators, distinct from the developer billing surface.

## 5. SDK distribution

Three install paths, all from the same Apache-2.0 source:

```bash
npm i @multisynq/client                                          # bundler
```

```html
<script src="https://cdn.jsdelivr.net/npm/@multisynq/client@1.1.0/bundled/multisynq-client.min.js"></script>
```

```js
import * as Multisynq from
  "https://cdn.jsdelivr.net/npm/@multisynq/client@1.1.0/bundled/multisynq-client.esm.js";
```

The `@multisynq/client` package is a pure JS library — no native bindings, no service-worker requirement. React bindings ship separately as `react-together` (`github.com/multisynq/react-together`, Apache-2.0), exposing hooks like `useStateTogether`, `useChat`, `useCursors`, `useConnectedUsers` over the same client.

## 6. Open-source posture

- **Client SDK (`@multisynq/client` 1.1.0):** Apache-2.0, source on GitHub at `multisynq/multisynq-client` (241 stars). Confirmed in npm registry metadata and the repo `LICENSE`.
- **React bindings (`react-together`):** Apache-2.0, source on GitHub at `multisynq/react-together` (31 stars).
- **Synchronizer-CLI (`synchronizer-cli`):** Apache-2.0, source at `multisynq/synchronizer-cli`. This is the *operator tooling* — Docker container management, systemd integration, dashboard — but **the synchronizer container image itself is consumed via Docker; the synchronizer server source code is not in any public Multisynq repository at the time of writing.**
- **Legacy `@croquet/croquet` 2.0.4:** Apache-2.0 in the npm registry; clients on this SDK target the deprecated Croquet network and need to migrate.

The take-away: client-side is fully open; the reflector is *operationally* decentralized (anyone can run a node, contributing capacity to the network) but *not source-open* — operators run a binary container, they do not build it from source. This matters for Myrhiza (see §10).

## 7. Self-hosting story

You can run a synchronizer node:

```bash
npm install -g synchronizer-cli
synchronize init        # interactive: Synq key + wallet
synchronize start       # launches the container
```

A "Synq Key" is generated at `startsynqing.com` after entering a Discord-issued access code; the key is bound to a wallet address for reward accounting. The CLI auto-detects architecture (ARM64/AMD64), installs Docker if needed, and exposes a local web dashboard with QoS metrics.

What you *cannot* currently do: run a fully private synchronizer that is not enrolled in the Multisynq operator network. The CLI assumes a Synq Key and reports back to the network. Whether a self-contained synchronizer image is offered for enterprise contracts is plausible but unconfirmed in public docs *(unverified)*.

## 8. Migration from the Croquet network

The legacy network was deprecated **2025-07-30**. Existing apps on `@croquet/croquet` must:

1. Swap the package: `@croquet/croquet` → `@multisynq/client`.
2. Replace `Croquet.*` namespace references with `Multisynq.*` (`Croquet.Model` → `Multisynq.Model`, `Croquet.Session.join` → `Multisynq.Session.join`).
3. Get a new API key from `multisynq.io/coder` (Croquet keys are no longer honored).
4. Re-publish; sessions on the new network are distinct from any prior Croquet session.

Croquet Labs maintains a "Migration docs" link from `croquet.io` pointing into the Multisynq docs. The two SDKs are very similar by design — the rename was effectively a brand and infrastructure change, not an API rewrite.

## 9. Pre-migration history

Croquet Labs operated the Croquet network from roughly 2018 through 2025-07-30. The architecture and API surface were largely the same as today's Multisynq; the rebrand and the spin-up of the DePIN operator network are the substantive changes. The academic lineage (David A. Smith, Alan Kay et al., 2003 Croquet Project) predates the commercial network by 15 years and is a separate body of work — see `glossary.md` for the disambiguation.

## 10. Implications for Myrhiza

- **Myrhiza has no central reflector.** Croquet/Multisynq's whole architecture rests on a sequencer that imposes a global event order. Myrhiza must produce that order from peers themselves (event-log replay, CRDT, or peer-elected sequencer). The Multisynq programming model is reproducible *iff* we solve the ordering problem differently — the developer-facing surface need not change.
- **Self-host gap is the lesson.** The strongest critique of Multisynq for our purposes is that the reflector binary is not source-open: operators run an opaque container, and a fully private synchronizer is not a documented product. Myrhiza, being P2P and component-model-based, eliminates this gap by construction — peers are the sequencer. We should call this out explicitly when comparing.
- **Sequencer-as-DePIN is one viable scaling story for Myrhiza.** If we ever need a fall-back sequencer for cold-start or extremely large sessions, the Multisynq operator-network model (token-incentivised commodity hardware running an open container) is well-trodden enough to copy. We should not need to, but it is a known-working escape hatch.
- **End-to-end encryption with the session password is a reusable trick.** It lets Myrhiza host snapshots on untrusted relays without exposing app state — the same ergonomic that lets Multisynq claim "the server doesn't see your data" applies to any opaque blob store we adopt.

## Sources

- `@multisynq/client` README and source — github.com/multisynq/multisynq-client (Apache-2.0, v1.1.0, 2025-07-24, 241 stars)
- `multisynq/synchronizer-cli` README — github.com/multisynq/synchronizer-cli (Apache-2.0, 30 stars)
- `multisynq/react-together` — github.com/multisynq/react-together (Apache-2.0, 31 stars)
- `croquet.io` deprecation notice — Croquet network deprecated 2025-07-30
- npm registry: `@multisynq/client@1.1.0`, `@croquet/croquet@2.0.4` (Apache-2.0, last published 2025-06-09)
- *Multisynq: The Application Layer of the Internet* — medium.com/multisynq
- DePIN / Synqer onboarding — `startsynqing.com`, `multisynq.io/coder`
- `docs.multisynq.io` (API reference, getting-started, react-together index)
