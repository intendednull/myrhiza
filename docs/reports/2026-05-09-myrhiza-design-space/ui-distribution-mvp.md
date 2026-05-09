**Date:** 2026-05-09
**Status:** active
**Subject:** Myrhiza design-space — UI app contract, custom-pixel surfaces, multi-tenancy, app distribution + signing, browser viability, MVP demo shape, Willow→Myrhiza migration

This report mines the prior-art corpus for the brainstorming session that will lock the Myrhiza master spec on user-surface and packaging questions. Companion clusters cover convergence/state-apply, WASM substrate/ABI, identity/crypto/capabilities, and networking/sync/workers.

## Domain index

1. **UI app contract** — `ui:*` WIT shape; per-call capability gating on privileged surfaces; "UI is an app" vs "UI is the runtime's surface."
2. **Custom-pixel surfaces** — sandboxed iframes + postMessage on web; cross-platform equivalents (TUI / mobile-native / MCP).
3. **UI app catalog at v1** — which UI apps ship, which are aspirational, and which forcing function they exert on the kernel.
4. **Multi-tenancy** — per-app namespacing (event log, storage, gossip topics, key handles, fuel budget); Grove-model generalization; cross-app messaging scoping.
5. **App distribution & manifest format** — `manifest.toml` shape, OCI-artifact alignment, iroh-blobs hash-pinning, app discovery.
6. **App signing & trust root** — Ed25519-over-manifest-hash vs OCI signing (cosign/sigstore) vs publisher-key versioning chain.
7. **Browser viability strategy** — jco-transpile + sync-ABI submit-and-poll vs native-only-v1 vs WebRTC-only.
8. **MVP demo app shape** — what proves "kernel doesn't know about chat"; the implicit ABI lock that follows.
9. **Willow → Myrhiza migration** — when Willow refactors onto Myrhiza, and what gates that.

---

## 1. UI app contract

### A. WIT `ui:*` interface family imported by interaction components

**Mechanism.** A small, growing set of typed interfaces (`ui:panel`, `ui:list`, `ui:message`, `ui:form`, `ui:menu`, `ui:command-bar`, `ui:rich-card`, `ui:context-menu`) that interaction components import. The default UI app exports them; alternate UI apps export the subset they implement. App authors target the interface contract, not a specific UI app.

**Pros.** Language-agnostic; capability-typed at the WIT layer; multiple UI apps can serve the same interaction component without recompile; graceful degradation when an interface is missing (e.g. TUI without `ui:rich-card`).

**Cons.** Interfaces are non-trivial design work — Slack's Block Kit, Discord components, Microsoft's Adaptive Cards all evolved over years. Each `ui:*` addition is an ABI change on a privileged surface. Reusable view-model abstractions (panel, list, message) leak desktop-chat shape; bespoke surfaces (whiteboard, map, video grid) bypass the contract entirely.

**Sources.** PR #636 lines 124-159; Willow `ui.md` "UI-as-app reframe"; CM `lessons.md` "world / interface split"; `wasmcloud/capability-model.md` "deny-by-default + WIT-import-as-permission"; Spin SIP-021 `factor` shape.

**Precedent.** wasmCloud v2 `host_interfaces`-on-workload + WIT-typed boundary; Spin's `inherit_configuration` per-key; Slack/Discord/Adaptive-Cards as the only successful "third-party app extends a host UI" precedents (all proprietary, all per-platform). No P2P precedent — every prior P2P stack is single-app (Keet, Volla) or single-vendor-UI (AT Proto's clients).

### B. UI-is-the-runtime reframe (pull the carve-out into the kernel)

**Mechanism.** Drop "UI is an app" framing; declare Myrhiza ships kernel + default UI surface together. The default UI is the runtime's UI module; alternate hosts (TUI, MCP) are outright re-implementations of the runtime, not "alternate apps."

**Pros.** Honest about the privilege gap — the default UI binds DOM + clipboard + file picker + push + IndexedDB + service workers; calling that "an app" is a fiction. Avoids defining the `ui:*` ABI prematurely. Lets us ship interaction components that import Leptos types directly at v1.

**Cons.** Closes the door on third-party UI apps (`willow-ui-tui`, `willow-ui-mcp`) being first-class peers. Inverts the PR #636 commitment that drove the reframe. Loses the forcing function that "kernel doesn't know about UI" provides.

**Sources.** PR #636 lines 130-138 (honest "broad and unstable capability surface"); Willow `runtime-vision.md` reframe-question ("`UI is an app`... should perhaps be `Myrhiza has no built-in UI; UIs are apps from day one`"); Pears `pear-runtime.md` (Pear treats apps as Bare-runtime processes — no UI substrate at all).

**Precedent.** Pears: no `ui:*`-equivalent, every Pear app paints its own pixels. wasmCloud v2: no UI concern at all. Holochain: UI is fully out-of-scope (HTTP gateway + whatever-app-author-builds).

### C. Hybrid — `ui:*` exists but the default UI is named privileged

**Mechanism.** Keep PR #636's `ui:*` WIT interfaces. *Also* document that the default UI app has a privileged capability set the kernel won't grant to arbitrary third-party UI apps (e.g. raw DOM access, service-worker registration). Third-party UI apps may export `ui:*` but bind a smaller capability set; users grant the broader set only to the default UI by manifest.

**Pros.** Honest about the privilege gap without abandoning the contract. Permits future `willow-ui-tui` while explicitly carving out "the default web UI is in the TCB for its own DOM."

**Cons.** Two-tier UI-app classification adds spec surface. Every interaction component must reason about "what if my UI app doesn't have surface X." Per-call capability gating on `ui:*` privileged surfaces (PR #636 lines 326-335) is non-trivial to specify and enforce.

**Sources.** PR #636 lines 130-138 + 326-335; Willow `ui.md` "per-call capability checking on `ui:*` proxies"; wasmcloud `capability-model.md` on TCB partitioning ("plugins are TCB; components are not").

**Precedent.** Browser extension model — extensions get broader capability surface than tab-content scripts; declared per manifest, gated per call by the browser. Closest direct precedent for what PR #636 sketches.

### Per-call capability gating on privileged `ui:*` surfaces

**The load-bearing commitment from PR #636 lines 326-335.** Clipboard writes, file pickers, top-level navigation, push registration, drag-and-drop, IndexedDB are gated by the *calling component's* manifest, not the UI app's broad surface. Mechanism: when a translation component composes inside the UI app and asks for a clipboard write via `ui:context-menu/copy`, the UI app proxies through a kernel-mediated call that checks the *translation component's* manifest. The UI app is in the TCB for its own chrome; not for third-party callers' intents.

**Open question.** Can this work without explicit kernel mediation on every `ui:*` call? Or does it force every `ui:*` proxy through the kernel, bottlenecking the UI hot path? Per-call vs per-binding is the perf/security tradeoff to settle.

### Willow position

Willow PR #636 commits to (A) with hardness on the per-call gating. Default UI app exists but is in-tree. CLAUDE.md already names "Capabilities are the only host surface" as a hard rule.

### Re-evaluation question for Myrhiza

PR #636 framed (A) against an existing chat-monolith Willow product where the Leptos client *was* the product. Myrhiza has no incumbent UI; the "just-an-app" framing is honest only if we actually ship a third-party UI app at v1 (e.g. `willow-ui-tui`) whose existence forces the contract honestly. If v1 ships only the default web UI, (A) is performative — (C) or even (B) is more honest. **Brainstorming must lock whether at least one alternate UI app ships at v1, because that decides which of (A)/(B)/(C) is honest.**

### Open questions

- Initial `ui:*` interface set — minimum to express chat (panel, list, message, form, menu) vs the larger set the chat product needs (reactions, presence, typing, file uploads, voice notes, profile cards, command palette).
- Capability-gated calls on `ui:*` — list of which calls require the calling component's per-call grant vs which can ride on the UI app's binding.
- Versioning policy for `ui:*` — semver-shaped, additive-only within minor, breaking only with major (per CM WIT semver convention).
- Resource handle ownership — does the UI app own panel/list resource handles, or does the kernel?

---

## 2. Custom-pixel surfaces (the iframe escape hatch)

### A. Sandboxed iframe + postMessage on web; platform-specific elsewhere (PR #636 commitment)

**Mechanism.** Whiteboard, code editor, network-graph viz, 3D voice room ship as a self-contained web bundle (HTML+JS+WASM) the default UI app embeds in a sandboxed iframe. Communication is a kernel-mediated postMessage protocol — the iframe cannot reach `window.parent` directly; the UI app proxies messages through a kernel capability, which decides which messages are permitted based on the iframe's manifest.

**Pros.** Web-shaped escape hatch is honest — the web is where this kind of thing actually works. Sandbox is real (browsers enforce iframe origin / sandbox attributes). Bevy/wgpu/Pixi-based surfaces compose here, not as `ui:*` competitors.

**Cons.** Native (TUI / mobile-native) has no equivalent — these surfaces just don't exist on those platforms. PR #636 says "unavailable on this surface" is the correct answer for non-web hosts; that's a real product-shape constraint (whiteboard apps don't run on a TUI). Iframe sandbox is a browser-engine concern; we depend on browser security.

**Sources.** PR #636 lines 161-171; Willow `ui.md` "Custom-pixel iframe escape hatch"; `wasm-component-model/browser.md` (iframe-sandbox is the only honest browser sandbox layer beyond the WASM sandbox).

**Precedent.** Webxdc (Delta Chat's web-app embeddable format — runs in a sandboxed webview, postMessage to the host app). Glimesh / Twitch extensions (sandboxed iframes embedded in a host UI). Slack's "modals from app" (postMessage to host). Discord's activity SDK (iframe-sandboxed games inside Discord channel surface).

### B. WIT-typed custom-pixel interface (no iframe escape hatch)

**Mechanism.** Define a `ui:custom-canvas` WIT interface — interaction components export it, the UI app calls into it for paint events, mouse/touch events flow back via a callback interface. No iframe; the interaction component is just another component.

**Pros.** Uniform model — everything is a WIT-typed component. No browser-specific concept leaking into the kernel. TUI hosts could in principle render through the same interface (poorly).

**Cons.** Per-pixel paint over the WASM canonical ABI is dramatically slower than `<canvas>` direct DOM. Whiteboard / code-editor / 3D-voice-room's whole point is that they need direct GPU/DOM access. PR #636 explicitly rejects this approach for v1 (Bevy ruled out as primary substrate).

**Sources.** PR #636 lines 167-171 ("GPU-driven UI substrates ... compose here, not as a replacement"); CM `abi.md` on canonical-ABI cost (`realloc` round-trip per call); Willow `ui.md` (Bevy retired, Leptos-only).

**Precedent.** None successful. Holochain's UI story is "render whatever you want in your own webview" — closer to (A). Spritely uses CRDT-typed interfaces but doesn't ship custom-pixel UIs. Bevy's web tooling is "2027-2028 timeframe" per PR #636.

### C. Out-of-scope — defer custom-pixel surfaces to post-v1

**Mechanism.** v1 doesn't have whiteboard / code-editor / 3D-voice. The `ui:*` contract is enough. Customer-pixel surfaces are a v2 spec.

**Pros.** Simplest. Ships sooner. Custom-pixel surfaces are not on the chat-shaped MVP critical path.

**Cons.** Cuts off voice/video as v1 features. Voice especially is the most-wanted Willow feature *and* relies on WebRTC, which won't fit a `ui:*` contract — so deferring custom-pixel surfaces effectively defers voice. Closes the door on in-tree examples that would force the iframe-escape-hatch design.

**Sources.** Pears `keet-and-apps.md` "WebRTC + Hyperswarm beats inventing a media stack" — voice/video is non-negotiable for any messenger competitive offering, and WebRTC is the only viable path.

### Native equivalents — TUI / mobile-native / MCP

**TUI.** Custom-pixel surfaces map to "unavailable on this surface" gracefully. A TUI host renders a placeholder ("[Whiteboard unavailable on terminal]"). No equivalent escape hatch.

**Mobile-native.** Compose / SwiftUI hosts can embed `WebView` / `WKWebView` for the iframe analog — same postMessage shape, kernel-mediated. But this re-introduces a browser engine dependency on mobile, defeating the "native UI" pitch of `willow-ui-mobile-native`. Alternatively, mobile-native hosts could expose platform-native equivalents (`UIDocumentBrowserViewController`, `MapView`, `AVKit`) — but each is a fresh kernel ABI to design.

**MCP.** LLM-host has no concept of custom-pixel surfaces. The `willow-ui-mcp` host renders interaction-component-defined structured data; custom-pixel surfaces simply don't apply.

### Willow position

Willow PR #636 commits to (A) for web; explicitly says non-web hosts say "unavailable." No design for native-mobile equivalent.

### Open questions

- Is "unavailable on this surface" honest enough, or does Myrhiza need a portable-degraded-mode contract (e.g. whiteboard renders as "list of strokes I can read but not paint" on TUI)?
- Mobile-native — embedded WebView for parity, or platform-native escape hatch per surface?
- Trust root for the iframe content — same hash/signing as the bundling app, or separate?
- Does the iframe see iroh peer state, or only postMessage from the parent UI app?

---

## 3. UI app catalog at v1

### Candidates (PR #636 lines 147-154)

| App | Substrate | v1 status | Forcing function |
|---|---|---|---|
| `willow-ui-leptos` (default web UI) | Leptos + Rust→WASM | **Required v1.** | Lifts Willow's 60+ existing components onto `ui:*`. Privileged broad-capability binding. |
| `willow-ui-tui` | ratatui + Rust native | **Aspirational / v1 candidate.** | Forces the `ui:*` ABI honest — TUI exporting only the chat-shaped subset proves graceful degradation works in the first ship. |
| `willow-ui-mcp` | rust-mcp-sdk, exposes Myrhiza apps as MCP tools/resources | **v1 candidate per PR #636 framing** ("today's `willow-agent` becomes this"). | Forces apps to be agent-readable from day one, not retrofit. Validates `ui:*` non-deterministic-host-OK profile boundary. |
| `willow-ui-mobile-native` | Compose + SwiftUI | **Far-future.** | Two-platform-native maintenance burden; iroh-ffi is unmaintained (`iroh/mobile-and-wasm.md`). |
| `willow-ui-dioxus` | Dioxus Blitz | **Post-Blitz-maturity, ~2027.** | Replaces Leptos default once Blitz is production-ready. |

### Option recommendations

**A. Default + TUI + MCP at v1.** Three-UI-app set is the minimum honest ship of the "UI is just an app" framing. TUI and MCP are existence-proofs. MCP is high strategic value (LLM agents as first-class peers).

**B. Default-only at v1.** Ships fastest. "UI is just an app" becomes performative until v2. Shifts toward the (B) reframe in §1.

**C. Default + MCP at v1.** Skip TUI; MCP serves the "alternate UI exists" forcing function and solves a real product need (agent integration). TUI is not on critical path; defer.

### Sources

PR #636 lines 124-159 (UI app catalog); Willow `apps.md` lines 110-124 (UI app catalog); `iroh/mobile-and-wasm.md` (mobile-native is unmaintained territory upstream); `wasm-component-model/browser.md` (jco is the only browser path).

### Precedent

No precedent for multiple-first-class-UI-apps on a single P2P runtime. AT Protocol comes closest (Bluesky, Skylight, etc.) — but those are independent products against a shared protocol, not "UI apps on a runtime." MCP-as-UI-host has no production precedent yet (MCP is new).

### Brainstorming question

If only one UI app ships at v1, the "UI is an app" framing is performative — Myrhiza is a runtime + bundled UI, not a runtime. If three UI apps ship, the framing is honest but the schedule slips. **Pick: (A) honest-with-three, (B) ship-default-only and call (C) reframe in §1, or (C) default+MCP as the high-leverage middle.**

---

## 4. Multi-tenancy (Grove-model generalization)

### A. Per-app namespace, kernel-enforced, on every primitive (PR #636 trajectory)

**Mechanism.** Each app instance the kernel hosts gets its own:
- **Event log topic.** Distinct gossip topic-ID per (app, instance); events on topic A do not appear on topic B.
- **Storage namespace.** Per-app KV namespace; per-app blob-tag namespace. Apps cannot read each other's storage.
- **Key handles.** Per-app handle namespace. App A's handle 5 is not app B's handle 5.
- **Fuel budget.** Per-app, per-call budget for `state-apply`; per-app, per-tick budget for `interaction`/`behavior`.
- **Memory cap.** Per-component-instance memory limit (Wasmtime store quota).

Cross-app messaging only via kernel-mediated capability — app A imports `inter-app:send-message` and the kernel routes through. Receiver app must export an inbox interface; granted at install time.

**Pros.** Strict isolation; "different state component, different topic, no leakage" is MVP criterion #4. Maps cleanly onto WASM Component Model's per-instance isolation. Fuel-as-consensus for `state-apply` (`wasm-component-model/lessons.md`) requires this.

**Cons.** Per-call kernel mediation for cross-app messaging adds latency. Handle namespace ownership unsettled — what happens when two apps install keys under the same opaque handle (Willow `runtime-vision.md` "Handle namespace ownership"). Fuel budget defaults are unknown (one of PR #636's open questions).

**Sources.** PR #636 MVP criterion #4; CLAUDE.md component-profile table; Willow `apps.md` "Grove generalizes to apps a peer has joined"; CM `lessons.md` "shared-nothing across apps, shared-everything within an app"; Spin `architecture.md` SIP-023 per-key inheritance; wasmcloud `capability-model.md` "deny-by-default."

**Precedent.** Spin's manifest-static `allowed_outbound_hosts` / `key_value_stores` (per-component scope). wasmCloud v2 `host_interfaces` on workload. Wasmtime's `Store` per-component as the natural enforcement boundary. Holochain's per-DNA cell isolation.

### B. Operator-configurable resource limits, app-declared caps

**Mechanism.** App's `manifest.toml` declares its desired fuel budget, memory cap, gossip-topic count, etc. Operator (the peer running the kernel) configures hard limits; app's declared values clamp to operator's max. Per-app namespace is structural (always per-app); resource limits are policy.

**Pros.** Policy/structure separation matches Spin/wasmCloud; operator can run different policies for the same app on different peers. App authors can request the budget they need.

**Cons.** Operator-configurable means non-determinism risk for `state-apply` — if operator A allows 1M instructions and operator B allows 500K, peer A's apply succeeds while peer B's fails for the same event. Determinism violated.

**Sources.** Spin SIP-023 (per-key configuration inheritance); wasmcloud `capability-model.md` `localResources.allowed_hosts`; CM `lessons.md` "Profile-the-substrate-not-the-app."

**Precedent.** Spin manifest-static + runtime-config-toml. wasmCloud v2 `localResources`. Both are server-only; neither has determinism-as-consensus constraint that Myrhiza imposes.

### C. Strict per-app structural namespacing + fixed kernel-wide fuel/memory budgets

**Mechanism.** Per-app namespacing on event log, storage, handles, gossip is structural (kernel-enforced, non-negotiable). Fuel + memory limits are *kernel-wide constants* — same value on every peer running the same kernel version. No operator knob.

**Pros.** Determinism preserved by construction. `state-apply` always runs with the same fuel ceiling everywhere. Cross-peer convergence is unconditional.

**Cons.** Kernel-wide constants must be conservative (largest legitimate state-apply must fit). Apps with genuinely-larger state-apply needs cannot opt up. Kernel version upgrades shift the constant — apps must be compatible with the new ceiling.

**Sources.** CM `lessons.md` "fuel-for-state-apply is part of the consensus invariant"; CLAUDE.md "Determinism is a load-bearing property"; PR #636 "fuel exhaustion ... is a determinism property."

### Cross-app messaging — kernel-mediated only (PR #636)

PR #636 already commits to "all cross-component calls go through the kernel" (lines 481-482). For cross-*app* messaging specifically, the kernel needs to verify both apps' manifests permit the cross-app interface. The shape is:
1. Source app imports `inter-app:send-to(target-app-id, message)`.
2. Target app exports `inter-app:inbox(message)`.
3. Kernel checks both manifests; if grants align, kernel routes; if not, deny-all.

The unscoped question: how does cross-app messaging *scope* when both apps are multi-instance (one peer running multiple instances of app A and multiple instances of app B)? Per-(app-instance) addressing is needed; PR #636 doesn't specify the addressing scheme.

### Willow position

Willow's Grove model already does per-server gossip-topic + per-server `EventDag` + per-server `ServerState`. Per-server `StateActor`s isolate from each other. The mechanism transfers to per-app at the kernel layer; Willow's Grove model is precedent. Willow `apps.md` "Grove generalizes to apps a peer has joined."

### Re-evaluation question

Willow's Grove is fully kernel-side (server registry actor). For Myrhiza, the registry is *app-instances*, not servers. The kernel needs to track (app bundle hash, instance ID, materialized state component instance, gossip topic). PR #636 calls for this but doesn't specify the registry shape — that's a child spec.

### Open questions

- Cross-app messaging addressing scheme (app-instance-id or app-id with instance-routing inside).
- Fuel budget defaults — kernel-wide constants vs operator-configurable vs app-declared. Determinism tilts hard toward kernel-wide-constants for `state-apply`.
- Handle-namespace ownership when apps collide. (Willow `runtime-vision.md` open question.)
- Snapshot portability across component-version upgrades — when app A upgrades from v0.1 to v0.2, does the per-app state survive?

---

## 5. App distribution & manifest format

### A. `manifest.toml` + content-addressed bundle on iroh-blobs (PR #636 commitment)

**Mechanism.** App = directory containing `manifest.toml` (TOML) + `state.wasm` + `interaction.wasm` + zero-or-more `behavior-*.wasm` + `schema.wit`. The whole directory hashes to a BLAKE3 hash; the bundle ID is that hash. Distribution: bundles live as iroh-blobs HashSeqs; peers fetch by hash; receivers verify content via BLAKE3-Bao streaming.

Manifest declares: app name, version, component file hashes, declared capabilities (which kernel imports each component binds), exported interfaces, signing key, signature.

**Pros.** Content-addressed = tamper-evident-by-construction. iroh-blobs gives verified streaming + range requests + sparse fetching. BLAKE3 root *is* the bundle ID. Maps cleanly onto WASM Component Model's per-instance isolation.

**Cons.** TOML manifest is yet-another-config-format. Bundle directory layout is not OCI-compatible without a wrapper; cosign/sigstore/registries assume OCI artifact layout. iroh-blobs has no discovery — bundle hash + provider node-addr must be carried out-of-band (`iroh/blobs.md` "Discovery: there isn't one").

**Sources.** PR #636 lines 96-122; Willow `apps.md` "Apps as bundles"; `iroh/blobs.md`; CM `lessons.md` "OCI content-addressed registry convention"; Spin `sdks-and-tooling.md` (OCI artifacts + `wkg` resolution).

**Precedent.** Pear's `pear://<length>.<key>` — Hyperdrive-content-addressed with append-only versioning. Spin's OCI-artifact-with-content-digest layers. wasmCloud's OCI artifacts. CM `wkg`. All converge on content-addressed-app-bundle as the right shape.

### B. OCI-artifact-shape (Spin/wasmCloud/wkg alignment)

**Mechanism.** Same component bundle but laid out per the [CNCF TAG Runtime WASM OCI artifact layout](https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/) — manifest is OCI ImageManifest; layers are component blobs; signing via cosign/sigstore. iroh-blobs distribution is layered atop: the OCI ImageManifest's layer digests are also iroh-blobs hashes.

**Pros.** Reuses the entire OCI tooling ecosystem (cosign, SBOM, vulnerability scanning, mirroring). Spin proves this works at production scale. App authors can use existing CI/CD to publish.

**Cons.** OCI manifests carry HTTP-shaped assumptions (mediaType, schemaVersion). iroh-blobs is bytestreams; the impedance match has fiddly edges. Cosign signing assumes a registry as trust root; P2P doesn't have one.

**Sources.** Spin `sdks-and-tooling.md`; CM `lessons.md` "OCI content-addressed registry convention"; wasmcloud `architecture.md`.

**Precedent.** Spin v4 + `wkg` + cosign is the prevailing stack. Microsoft's WASM-via-OCI blog. CNCF TAG Runtime layout. wasmCloud v2 ships OCI artifacts as the workload artifact format.

### C. `spin.toml`-shape (TOML, but aligned with Spin's existing format)

**Mechanism.** Adopt Spin's `spin.toml` format with Myrhiza-specific extensions for component profiles. Reuses Spin's existing tooling (`spin build`, `spin watch`).

**Pros.** Existing tooling, existing parser, broad community familiarity.

**Cons.** Spin's manifest is request-handler-shaped; Myrhiza's apps aren't request-handlers. The shape misalignment forces extension cruft. CLAUDE.md explicitly avoids "modeling Myrhiza components as request-handlers" (Spin `lessons.md` Avoid).

**Precedent.** Spin v4. The decisive negative: Spin per `lessons.md` is the *opposite* design point from Myrhiza on multiple axes; reusing the manifest format means inheriting the request-handler framing.

### Discovery — out-of-band install vs in-band publish

**Out-of-band install (PR #636 default).** User pastes a `myrhiza://` URL or scans a QR code; kernel fetches by hash via iroh-blobs. No in-network discovery. Same as Pear's `pear://<key>` ticket model.

**In-band publish.** A peer's app-publish event broadcasts the bundle hash on a well-known gossip topic; other peers can install by browsing that gossip stream. Discovery becomes a kernel concern.

**Hybrid.** Both — out-of-band install for trust-rooted distribution; in-band publish gossip for "apps the peers I follow have published."

PR #636 defers this to a child spec; the brainstorming should commit to out-of-band as the default for v1 (Pear precedent) and decide whether in-band publish is in scope.

### Willow position

PR #636 commits to (A) — `manifest.toml` + iroh-blobs hash-pinning. Distribution by hash + provider node-addr is out-of-band.

### Open questions

- TOML vs YAML vs JSON for manifest. (TOML matches Cargo, Spin; YAML matches Kubernetes, K8s tooling; JSON is universal.)
- OCI alignment depth — bytewise-compatible-with-OCI vs Myrhiza-native-with-OCI-export.
- Discovery — out-of-band only vs in-band publish vs hybrid.
- Bundle-author identity — single Ed25519 key vs multisig (Pears `pear multisig` precedent).

---

## 6. App signing & trust root

### A. Ed25519 over manifest hash, single publisher key (PR #636 implied default)

**Mechanism.** Bundle author generates Ed25519 keypair. Manifest hash is signed; signature is in the manifest. Trust root: user knows publisher's pubkey and trusts publisher (out-of-band).

**Pros.** Simple. Matches Willow's Ed25519-everywhere convention. No external trust infrastructure.

**Cons.** Single-key compromise = total compromise. No revocation primitive. No multi-author signing.

**Sources.** PR #636 line 49 (Ed25519 identity); Willow `lessons.md` "Ed25519 identity validates"; CM `lessons.md` "content-addressed packaging."

### B. OCI-style signing — cosign/sigstore over OCI artifact (Spin/wasmCloud alignment)

**Mechanism.** Bundle is an OCI artifact; signed via cosign with optional sigstore-backed transparency log. Trust root is a public-key infrastructure (Fulcio root + sigstore TUF) or a self-sovereign keypair.

**Pros.** Mature tooling. Transparency log gives revocation primitive. Works with existing CI/CD signing pipelines.

**Cons.** Trust root is a centralized PKI (Fulcio + Rekor) — incompatible with P2P "no servers" framing. Self-sovereign mode is just (A) with extra steps. CM `lessons.md` flags OCI-via-GHCR as flaky in prior art.

**Sources.** CM `lessons.md`; Spin `sdks-and-tooling.md`; wasmcloud `architecture.md`.

### C. Multisig publisher key (Pears `pear multisig` pattern)

**Mechanism.** Bundle's publisher identity is N-of-M signing keys. Publishing a new version requires N signatures over the new manifest hash. Each manifest carries the N signatures; verifiers check all N against the known M-key set.

**Pros.** Anti-rogue-publisher defense (one compromised key cannot publish bad bundles). Real-world precedent — Holepunch added `pear multisig` in v2.5.0 specifically for this. Self-sovereign — no PKI dependency.

**Cons.** N-of-M coordination cost (off-line signing flow). Schema bigger. Non-trivial UX for solo developers — "do I really need 3-of-5?"

**Sources.** Pears `lessons.md` "Multisig for production-release signatures"; Pears `pear-runtime.md` (`pear multisig` v2.5.0).

**Precedent.** Pears `pear multisig`. Sigstore policy controllers (multi-key required). Software Update Frameworks (TUF) has equivalent shape.

### Version chain semantics

**Append-only Hypercore-length-as-version (Pear).** Publisher key + integer length. Updates are appends. No semver, no branches.

**Semver + signed manifest.** Each version is independently signed; semver-shaped (`1.2.3-rc1`).

**Hybrid.** Publisher-key-rooted append-only history + semver tags as labels on the chain.

PR #636 doesn't pick. Pears precedent strongly favors append-only chain. Spin/wasmCloud use semver.

### Willow position

Willow uses Ed25519 for everything (event signing, identity). PR #636 lifts this to bundle signing. Multisig is mentioned as a Pears-precedent worth borrowing per Pears `lessons.md` but PR #636 doesn't commit.

### Open questions

- Multisig at v1 or as v2 escalation path?
- Trust root for new-app installs — user-keyrings vs publisher-key-rooted vs no-default-trust ("install-time prompt with manifest summary," PR #636 line 318).
- Revocation — append-only revocation events on a publisher's chain vs sigstore-style transparency log vs no formal revocation.
- Cross-publisher verification — does Myrhiza ship a "verified publisher" registry, or is identity always self-sovereign?

---

## 7. Browser viability strategy

### A. Browser-first via jco-transpile + sync-ABI submit-and-poll (PR #636 commitment)

**Mechanism.** Kernel compiles to WASM via jco. Every component (kernel-internal too) is jco-transpiled. Host imports satisfied by Myrhiza-specific JS shim implementing kernel capabilities against browser APIs (WebSockets / WebRTC / IndexedDB / SubtleCrypto). Async on browser handled via submit-and-poll: sync host call returns a `request-token`; kernel re-enters component via `on-completion(token, result)` later.

**Pros.** Browser-first means Willow's web client is a viable Myrhiza app from day one. Single kernel image + jco transpile = "build once, run anywhere." User experience matches what people expect ("paste URL, run app").

**Cons.** ~350KB JS shim floor on every browser load. jco maturity is real schedule risk — preview3 not in jco yet. componentize-js bundles SpiderMonkey; size cost is real for JS-authored apps. Sync-only ABI on browser is a significant ergonomic tax. Browser cannot use `wasi:sockets`; all transport via `iroh-relay` over WebSocket (`iroh/mobile-and-wasm.md` "browser-bound = relay-bound"). Nested-WASM-in-browser-WASM (interaction component loaded inside Leptos UI app loaded inside browser) is unproven.

**Sources.** PR #636 lines 444-498; `wasm-component-model/browser.md`; `wasm-component-model/preview-status.md`; `iroh/mobile-and-wasm.md`.

**Precedent.** Fastly Compute (jco internally for edge). No production precedent for nested-CM-in-browser-WASM. jco itself uses `js-component-bindgen` (a CM component compiled to JS via jco — bootstrapping in production). preview2-shim ships at v0.17.9.

### B. Native-first; defer browser to v2

**Mechanism.** v1 ships native kernel only (Wasmtime). Browser deferred until preview3 jco support stabilizes (~2027) or until we have evidence the Myrhiza-specific JS shim is tractable. Willow's web client refactors onto Myrhiza only after browser-v2 ships.

**Pros.** Skips the ~350KB shim cost, the sync-only ABI tax, and the "is jco production-ready for Myrhiza's surface" question. Native is well-trodden — Wasmtime is proven in Spin, wasmCloud, Fastly Compute. Lets us use full preview2 (filesystem-backed storage natively) without browser-shim equivalents.

**Cons.** "P2P apps in your browser" is the killer demo. Native-only makes Myrhiza desktop-first; phone story stays unsolved (iroh-ffi is unmaintained). Willow's existing investment in the web client cannot lift onto Myrhiza for ~1 year. Closed-source-flagship (Pear/Keet model) becomes architecturally tempting.

**Sources.** `wasm-component-model/browser.md` ("not yet shippable"); `iroh/mobile-and-wasm.md` ("hundreds of thousands of devices on Delta Chat" via Rust-on-mobile, not browser); Pears `keet-and-apps.md` "mobile-first runtime is achievable" via native-rust path.

### C. Browser-second; native-first but design ABI for browser day-one

**Mechanism.** v1 ships native kernel. ABI choices (sync-only, submit-and-poll for async) are made *as if* browser were a target — so the v2 browser ship is mechanical, not a redesign. Willow's web client refactors after v2.

**Pros.** Avoids the schedule risk of jco maturity. Doesn't lock browser out of v2. Per-component ABI choices make sense regardless of browser-or-native (sync-only state-apply is required for determinism anyway).

**Cons.** Still doesn't get "P2P apps in your browser" demo at v1. Two ship dates — coordination cost. Risk: native-only design ossifies and v2 browser becomes a wholesale rewrite.

### Browser cost summary

- jco transpile = ~350KB JS shim per component minimum. (`wasm-component-model/browser.md`)
- iroh in browser = relay-bound (`iroh/mobile-and-wasm.md`).
- preview3 in browser = not shipping (`wasm-component-model/preview-status.md`).
- componentize-js = ~5MB SpiderMonkey-included for JS-authored apps.
- preview2-shim = active (`v0.17.9`, 2026-04-17). preview3-shim Node-only.

### Willow position

PR #636 commits to (A) — browser-first, jco-transpile, sync-only, submit-and-poll. CLAUDE.md doesn't explicitly commit to (A); it leaves room.

### Re-evaluation question

PR #636's commitment to (A) is the riskiest schedule item. The ~350KB shim floor + sync-only ABI + jco preview3 immaturity + browser-relay-only-via-iroh = four compounding constraints. (B) is the schedule-safe answer. (C) is the design-discipline-safe answer that punts execution. **Brainstorming must commit which.**

### Open questions

- jco maturity — is `@bytecodealliance/jco@1.19.0` good enough for Myrhiza's CM-nested-in-browser shape? We don't have a proof.
- Myrhiza-specific JS shim scope — `myrhiza:peer`, `myrhiza:state`, `myrhiza:authority` etc. all need browser-side implementations.
- Browser-side persistent storage — IndexedDB-backed `wasi:keyvalue` shim or Myrhiza-specific storage capability.
- Bundle size budget for v1 web — ~350KB floor + Leptos UI app + interaction components. Can it fit a "<5MB initial load" budget?

---

## 8. MVP demo app shape

### A. Tiny shared-counter (PR #636 candidate, preferred for "kernel doesn't know chat")

**Mechanism.** ~50 lines state-apply (counter increment/decrement, monotonic merge), ~100 lines interaction (panel showing count, two buttons). Deliberately not chat. Two peers running same component, see same count after operations from both.

**Pros.** Smallest possible thing exercising determinism + interaction loop. Easy to verify visually ("did both peers show 7?"). Low ABI surface — only needs `ui:panel` + `ui:command-bar`. Can ship with Extism v1 in 4-6 weeks.

**Cons.** Toy. Doesn't exercise multi-component, doesn't exercise behaviors, doesn't exercise per-app namespacing under load. Only proves determinism + roundtrip; nothing about scale.

**Sources.** PR #636 lines 575-598 (MVP candidates); Willow `apps.md` lines 134-141; CLAUDE.md "MVP shape locks ABI choice."

### B. Single-channel non-Willow chat

**Mechanism.** A chat app that does NOT reuse `willow-state`'s `ServerState`. Implements its own event variants, its own apply, its own `ui:*` view. Single channel, no permissions, no roles, no encryption.

**Pros.** Proves "chat is just one app" by *being* a chat app on the runtime. Chat shape is what users care about; demo resonates. Exercises message state + multi-peer convergence + UI binding.

**Cons.** Risks confusion ("but Willow is also a chat app?"). Doesn't escape the chat-shaped ABI gravity well. ABI-wise, requires `ui:list` + `ui:message` + `ui:form` — broader surface than (A).

### C. Real-time poll

**Mechanism.** Create poll, vote yes/no, see live tally. Vote weighting by author. Quorum threshold.

**Pros.** Not chat. Exercises authority predicate (must verify voter is allowed to vote, vote cannot duplicate). Similar surface to (A) but with more interesting authority semantics. Demonstrates `state-propose` (compose vote payload) + `state-apply` (apply vote, check authority).

**Cons.** Slightly bigger than (A). Polls in production are a strange product.

### D. Kanban / wiki

**Mechanism.** Substantial CRDT-backed board or wiki. Per-card / per-page state with concurrent edits.

**Pros.** Shows off non-chat-shaped P2P apps. Demonstrates real CRDT use under the runtime.

**Cons.** Multi-month build. Defers MVP by 3+ months. Not in PR #636's MVP candidate list.

### MVP locks ABI choice

PR #636 lines 444-498: "**(A) Full Component Model from day one** ... Cost: heavier toolchain, browser CM still maturing, ~350 KB JS shim floor, no async on browser side. **(B) Extism for v1** ... Ship faster on a simpler runtime ... Migration to full Component Model later is a real refactor for app authors."

The MVP shape implicitly chooses:
- Counter / Poll = ~50 lines state, ~100 lines interaction = **Extism v1 ships in 4-6 weeks.**
- Chat / Kanban = full multi-component, multi-instance = **Full CM v1 ships in 3-4 months.**

PR #636 leans (B) Extism but leaves it open. The MVP choice forces it.

### Six MVP acceptance criteria (PR #636 lines 575-598; lifted into Willow `apps.md`)

1. Kernel loads + instantiates a WASM state component from an iroh-blobs bundle.
2. Multi-peer convergence — same component bytes converge to same state hash.
3. UI app loads interaction component, projects view, submits command, observes resulting state change.
4. Two app instances coexist on one peer without event-crossing.
5. Capability declarations actually gate access — component cannot import what its manifest doesn't declare.
6. Behavior component runs on designated peer, observes events, logs them. (Emitting events under kernel-custodied behavior identity is the next milestone, blocked on capability + identity-custody child specs.)

These six criteria are independent of MVP-app shape. Any of (A)/(B)/(C) can satisfy them; size differs only in what proves the criterion.

### Willow position

PR #636 leans toward (A) shared-counter as "deliberately irrelevant to chat" — but leaves all three (counter, single-channel-chat, poll) open.

### Brainstorming question

ABI choice (Extism vs CM) is implicitly locked by MVP timeline. **If MVP must ship in 4-6 weeks: Extism + counter/poll. If MVP can wait 3-4 months: full CM + a more substantial demo.** This is the highest-leverage decision in this domain.

---

## 9. Willow → Myrhiza migration

### Per CLAUDE.md (and the user direction)

> "Eventually Willow will be refactored to use our library but for now we need to use Willow as a reference point."

Willow is the reference point for shape; Myrhiza is built standalone. Willow refactors onto Myrhiza after Myrhiza is real.

### Migration gates

The migration cannot happen until at least:

- **Myrhiza v1 ships** with MVP demo app proving the runtime is real.
- **`ui:*` interface family is stable enough** for Willow's 60+ components to bind against.
- **Capability model child spec lands** (so Willow's broad surface is auditable).
- **Distribution + signing spec lands** (so Willow can ship as a Myrhiza app bundle).
- **Crypto + key-custody spec lands** (so Willow's MLS adoption survives the kernel boundary).

### Migration shape (per PR #636 lines 421-444)

> `willow-state` splits. A payload-agnostic kernel half (events, DAG, sync primitives, HLC) stays as kernel. The chat-specific half (`EventKind`, `ServerState`, `apply_event`, `required_permission`) becomes the `chat-server` app.
>
> The web client becomes the default UI app. Its bindings to chat semantics route through the kernel and the chat-server interaction component rather than through direct Rust imports of chat types.

For Myrhiza this is:
- Chat-specific `EventKind` becomes the chat-server app's state-component event variants.
- `ServerState` becomes the chat-server app's materialized state (per-instance in the kernel).
- `materialize::apply_event` becomes the chat-server app's `state-apply` export.
- `required_permission()` table becomes the chat-server app's per-event authority predicate.
- Web client becomes default UI app; binds chat-server's interaction component via `ui:*`.

### Three migration options

**A. Big-bang migration after Myrhiza v2.** Wait until Myrhiza is mature (capability + crypto + distribution all landed). Refactor Willow in one sprint. Cuts schedule risk but defers Willow benefits.

**B. Incremental migration by component.** As each Myrhiza primitive lands, migrate the corresponding Willow piece. Web UI first (becomes default UI app); then `EventKind` becomes chat-server app's events; then workers become generic peers. Spreads risk, costs continuous coordination.

**C. Parallel-track Willow runs unchanged on its own kernel; Myrhiza runs new apps.** Willow continues as today. Myrhiza ships with new (non-chat) apps. Willow refactor is a research-tract, never gated.

### Willow position

Per user direction, Willow is reference, not migration target for Myrhiza v1. Migration is "much later" (PR #636 lines 636-637). Implicit lean: (A) or (B) — but later.

### Open question

When does Willow refactor begin? Three triggers:

1. **After v1 MVP** — earliest. Aggressive. Forces `ui:*` to be production-ready.
2. **After capability model child spec** — middle. Aligned with Myrhiza maturity.
3. **After distribution spec** — latest. Most conservative.

PR #636 leaves it open. The user's direction commits only that "eventually" Willow refactors. **Brainstorming should pick a trigger so Willow's roadmap stays decoupled from Myrhiza's v1 critical path.**

---

## Cross-domain interactions

- **§3 UI app catalog at v1 ↔ §1 UI contract framing.** If only `willow-ui-leptos` ships at v1, "UI is an app" is performative; (B) UI-is-the-runtime reframe is more honest. If TUI / MCP also ship, (A) WIT `ui:*` is honest.
- **§7 Browser viability ↔ §8 MVP shape.** Browser-first (A) requires preview3 jco maturity Myrhiza doesn't have. MVP-shipping-fast favors Extism v1 (which is browser-shippable). Native-first MVP can ship in CM from day one.
- **§5 Distribution + §6 Signing ↔ §4 Multi-tenancy.** Per-app namespacing is meaningful only if the app's bundle hash is verifiable. Distribution = signing = identity = namespace. The four are one decision.
- **§8 MVP ↔ §9 Willow migration.** MVP cannot be Willow's chat (that'd be migration, not MVP). MVP must be a non-Willow app. If MVP is too small (counter), it doesn't force the multi-app, multi-component shape Willow's migration needs. Tension: MVP small enough to ship vs MVP big enough to validate the migration target.
- **§2 Custom-pixel ↔ §3 UI app catalog.** Custom-pixel surfaces only make sense if the host UI app exists to embed them. TUI / MCP hosts cannot embed iframes. Custom-pixel + multi-UI-host = "unavailable on this surface" must be a first-class affordance.
- **§1 Per-call capability gating ↔ §4 Multi-tenancy.** Per-call gating on `ui:*` privileged surfaces requires the calling component's manifest to be visible at the UI-app boundary. Per-app namespacing must extend to "which app's manifest is the source of authority for this call."

## Brainstorming question list

1. **(§1)** Does Myrhiza v1 ship at least one non-default UI app? If yes, "UI is an app" is honest; if no, reframe to "UI is the runtime's surface" or commit to it being performative-until-v2.
2. **(§1)** What is the initial `ui:*` interface set? Minimum-chat (panel/list/message/form/menu) or larger?
3. **(§1)** Per-call capability gating mechanism — how does the UI app know the calling component's manifest at call time?
4. **(§2)** Custom-pixel iframe escape hatch is web-shaped on purpose. TUI / MCP say "unavailable on this surface" — is that honest enough, or do we need a portable-degraded-mode contract?
5. **(§2)** Mobile-native equivalent of iframe — embedded WebView (re-introduces browser engine) or platform-native escape hatch per surface?
6. **(§3)** v1 UI app catalog: default-only / default+TUI / default+MCP / default+TUI+MCP?
7. **(§3)** Where does `willow-ui-mcp` fit — v1 ship or post-v1? Strategic value (LLM agents as first-class peers) is high; engineering cost is moderate.
8. **(§4)** Cross-app messaging addressing — by app-id with internal routing, or by (app-id, instance-id) addressing?
9. **(§4)** Fuel budget defaults — kernel-wide constants (determinism-safe) vs operator-configurable (more flexible, but determinism risk on `state-apply`)?
10. **(§4)** Handle namespace ownership when two apps install keys under the same opaque handle.
11. **(§5)** Manifest format — TOML (Spin/Cargo), YAML (K8s), or JSON?
12. **(§5)** OCI-alignment depth — bytewise-OCI (reuse cosign/sigstore tooling) or Myrhiza-native-with-OCI-export?
13. **(§5)** Discovery — out-of-band install only (Pear precedent) vs in-band publish gossip vs hybrid?
14. **(§6)** Signing model — single Ed25519 publisher key (simple) vs multisig N-of-M (Pear precedent, anti-rogue-publisher) vs cosign/sigstore (PKI-rooted)?
15. **(§6)** Version chain semantics — append-only Hypercore-length-as-version (Pear) vs semver+signed-manifest (Spin/wasmCloud)?
16. **(§6)** Trust root for new-app installs — user-keyrings, publisher-key-rooted, or no-default-trust ("install-time prompt with manifest summary")?
17. **(§7)** Browser viability strategy — browser-first (A, schedule risk), native-first (B, defers browser by ~1 year), or browser-second-with-design-discipline (C, native-first but ABI-correct from day 1)?
18. **(§7)** What's the target bundle size for v1 web (~350KB jco shim + Leptos UI + interaction components)? Can it fit a "<5MB initial load" budget?
19. **(§8)** MVP demo app shape — counter (smallest, Extism-feasible), single-channel-chat (most-relatable), real-time poll (interesting authority semantics), or larger?
20. **(§8)** ABI choice locked by MVP timeline — Extism-v1 (ship 4-6 weeks) vs full-CM-v1 (ship 3-4 months)?
21. **(§8)** Does MVP need to be agent-readable (i.e., does `willow-ui-mcp` count as part of the v1 acceptance criteria)?
22. **(§9)** Willow → Myrhiza migration trigger — after v1 MVP, after capability model spec, or after distribution spec?
23. **(§9)** Migration shape — big-bang post-v2 vs incremental-by-component vs parallel-tracks?

## Sources

**Primary spec:**
- `/tmp/willow-pr-636.diff` — Willow PR #636 master runtime spec, especially "UI is an app" (lines 124-171), "Capability model" (lines 326-335), "Apps as bundles" (lines 96-122), "ABI commitments" (lines 444-498), "MVP, in spirit" (lines 575-598), "What changes about Willow" (lines 421-444).
- `/mnt/storage/projects/willow/docs/specs/2026-04-19-ui-design/README.md` — Willow's target UX bundle (22 specs); design-language baseline.

**Prior-art primary:**
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/ui.md` — Leptos default UI; PR #636 reframe; per-call capability gating.
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/apps.md` — Apps as bundles; UI app catalog; MVP candidates.
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/runtime-vision.md` — PR #636 reframe-questions; "UI is an app" carve-out re-evaluation; ABI (A) vs (B).
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/lessons.md` — validates/avoid/borrow tables; chat-shaped baked-in patterns to reject.
- `/mnt/storage/projects/myrhiza/docs/prior-art/wasm-component-model/browser.md` — jco; preview2-shim; browser limitations.
- `/mnt/storage/projects/myrhiza/docs/prior-art/wasm-component-model/preview-status.md` — preview2 vs preview3 readiness; WASI version pinning.
- `/mnt/storage/projects/myrhiza/docs/prior-art/wasm-component-model/abi.md` — Canonical ABI; lift/lower; resource handles; shared-nothing linkage.
- `/mnt/storage/projects/myrhiza/docs/prior-art/wasm-component-model/lessons.md` — validates/avoid/borrow; OCI content-addressed registry; world/interface split.
- `/mnt/storage/projects/myrhiza/docs/prior-art/spin/architecture.md` — SIP-021 factor architecture; manifest-static capability; OCI artifacts.
- `/mnt/storage/projects/myrhiza/docs/prior-art/spin/sdks-and-tooling.md` — `spin.toml` shape; CLI verbs; componentize-* ecosystem; `wkg` distribution.
- `/mnt/storage/projects/myrhiza/docs/prior-art/spin/lessons.md` — validates/avoid/borrow for Myrhiza host design.
- `/mnt/storage/projects/myrhiza/docs/prior-art/wasmcloud/architecture.md` — v2 host topology; workload-as-deployment-unit; default-deny imports; control-plane vs data-plane separation.
- `/mnt/storage/projects/myrhiza/docs/prior-art/wasmcloud/capability-model.md` — `HostPlugin` trait; per-component authority via `host_interfaces`; v2 retreat from `wasmcloud:secrets`.
- `/mnt/storage/projects/myrhiza/docs/prior-art/pears/keet-and-apps.md` — Keet flagship; mobile push-relay; WebRTC + Hyperswarm for voice/video; honest-scale numbers.
- `/mnt/storage/projects/myrhiza/docs/prior-art/pears/pear-runtime.md` — `pear://<key>` content-addressed apps; sidecar pattern; Hypercore-length-as-version.
- `/mnt/storage/projects/myrhiza/docs/prior-art/pears/lessons.md` — validates/avoid/borrow; multisig publisher; mobile-first achievable.
- `/mnt/storage/projects/myrhiza/docs/prior-art/iroh/blobs.md` — iroh-blobs HashSeq + BLAKE3-Bao verified streaming; discovery gap; tagging/GC.
- `/mnt/storage/projects/myrhiza/docs/prior-art/iroh/mobile-and-wasm.md` — iroh-ffi unmaintained; browser is relay-bound; no built-in idle/wake.

**Cross-references:**
- `/mnt/storage/projects/myrhiza/CLAUDE.md` — locked decisions (component profiles, capabilities-only, determinism).
