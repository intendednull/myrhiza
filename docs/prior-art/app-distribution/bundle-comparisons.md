**Date:** 2026-05-22
**Status:** active
**Subject:** Side-by-side: how each Myrhiza-adjacent runtime defines its "app bundle". Spin, wasmCloud, Holochain, Endo, Pears, plus the CM canonical shape.

# Bundle format comparisons

The word "bundle" is overloaded across the WASM and adjacent P2P-runtime ecosystems. This file unpacks what each system means and how the shapes relate. The TL;DR comparison matrix lives at the bottom.

## The CM canonical bundle — a single `.wasm`

Per the WebAssembly Component Model spec, a **component is itself a self-describing bundle**. The CM `.wasm` binary format carries:

- The component's WIT-shaped interface (imports + exports + types) embedded as a custom section.
- Zero or more inner core modules (the actual WebAssembly code).
- Zero or more nested components (a CM component can contain other components).
- Metadata custom sections (author, name, version, build info).

A `wac compose` output is just another `.wasm` — the composition is fused into the same binary format. So "one CM component" and "many components composed into one" are byte-format-identical: both are `.wasm` files with a CM-shaped envelope.

**This is the canonical answer to "what is a WASM app?"** Everything else in this folder is either (a) a transport for this `.wasm` or (b) an alternative not-CM bundle format. Myrhiza's stance: CM `.wasm` is the bundle.

## Spin — `spin.toml` manifest + N components

[Spin](../spin/) packages an app as a `spin.toml` manifest plus N referenced CM components. The manifest:

```toml
spin_manifest_version = 2

[application]
name = "myapp"
version = "0.1.0"
authors = ["..."]

[[trigger.http]]
route = "/api/..."
component = "api"

[[trigger.http]]
route = "/auth/..."
component = "auth"

[component.api]
source = "target/wasm32-wasip1/release/api.wasm"
allowed_outbound_hosts = ["https://example.com"]

[component.auth]
source = "target/wasm32-wasip1/release/auth.wasm"
key_value_stores = ["default"]
```

OCI shape (via `spin registry push`): one OCI manifest pointing at one layer per component plus a config layer holding the `spin.toml`. The `artifactType` is Spin-specific. Resolution of cross-component WIT deps uses `wkg`.

**Key insight:** Spin's bundle is the **manifest + components together as one OCI artifact**. There's no separate "Spin app file" — the OCI manifest *is* the Spin app bundle. This is structurally cleaner than Holochain's tar-of-everything.

See [`spin/sdks-and-tooling.md:91-104`](../spin/sdks-and-tooling.md).

## wasmCloud — components + capability providers as separate OCI artifacts

[wasmCloud](../wasmcloud/) historically (v1) treated **each component as its own OCI artifact** and each capability provider as a separate OCI artifact. There's no aggregated "app bundle"; the wasmCloud `wadm` orchestrator pulls components + providers from registries at deploy time per a YAML manifest declaring which-OCI-refs-go-where.

The v2 architecture (post-2026-03 K8s pivot) layers Kubernetes CRDs on top — the K8s manifest holds the OCI refs and the runtime-operator pulls them. Underlying shape: still one OCI artifact per component, one per provider.

Push/pull:

```bash
wash oci push ghcr.io/foo/my-component:0.1.0 ./my-component.wasm
wash oci pull ghcr.io/foo/my-component:0.1.0
```

**Key insight:** wasmCloud's "bundle" is *deliberately not* an aggregated unit. Components and providers are decoupled, swappable, and versioned independently. The aggregation lives in the declarative manifest at deploy time, not in the artifact. This is the opposite philosophy from Spin.

See [`wasmcloud/architecture.md`](../wasmcloud/architecture.md) and [`wasmcloud/tooling.md`](../wasmcloud/tooling.md).

## Holochain — `.happ` / `.webhapp` (gzip + MessagePack of `mr_bundle::Bundle<M>`)

[Holochain](../holochain/) packages a "hApp" as a gzip-compressed MessagePack-serialized `mr_bundle::Bundle<M>` struct, where `M` is one of `AppManifest`, `DnaManifest`, `WebAppManifest`, `CoordinatorManifest`. File extensions (`.happ`, `.dna`, `.webhapp`, `.coordinators`) are markers only — the wire format is identical and the conductor identifies the bundle by which manifest variant deserializes.

Inside a `.happ`:

```
my-app.happ  (gzip → MessagePack)
└── AppManifest
    ├── manifest_version: '1'  (or '0' since 0.6)
    ├── name: "..."
    ├── description: "..."
    └── roles:
        - name: "chat"
          dna:
            bundled: my-chat.dna           # inline DNA bytes, also gzip+msgpack
              └── DnaManifest
                  └── integrity: { zomes, network_seed, properties }
                  └── coordinator: { zomes }
```

Each DNA's hash is `H(integrity_zomes_wasm || network_seed || properties)`. Bundles are **not seekable or streamable** — `unpack` reads the entire gzip stream into a `Vec<u8>` before msgpack-decoding. There's **no native bundle signing**; only `installed_hash` pinning at install time.

Discovery: historically the Holochain DevHub (a hApp-as-package-manager DHT app), now effectively shelved as of v0.400.0 (March 2025); the team recommends shipping standalone Electron/Tauri builds and discovery via GitHub releases pages.

See [`holochain/distribution.md`](../holochain/distribution.md).

**Critical difference from CM-on-OCI:** Holochain's bundle has all dependencies inlined as bytes. There's no "registry resolves missing deps" step — the bundle is fully self-contained. This trades discoverability and storage efficiency for offline installability.

## Endo — bundle-hash + Compartment Map (Agoric's bundle story)

[Agoric/Endo](../agoric-endo/) packages a JS application as a **bundle** in the Endo sense: a Hardened-JS-shaped object literal containing the application's source modules, transitive deps, and a [Compartment Map](https://github.com/endojs/endo/blob/master/packages/compartment-mapper/README.md) declaring import linkage.

The **bundle hash** (SHA-512 of the canonicalized bundle JSON) is the identity. Bundle hashes are computed deterministically; the same source + deps produces the same hash on any machine. This hash is what gets registered with Agoric's chain via `installBundle()` — chain consensus stores the hash, not the bundle.

```javascript
const bundle = await bundleSource('./entry.js');
// bundle.endoZipBase64Sha512 is the canonical identity
```

Two flavors of bundle:

- **`endoZipBase64`** — a base64-encoded ZIP archive of the modules. Compact.
- **`getExport` / `nestedEvaluate`** — the older `nestedEvaluate` format predates the ZIP shape and is still supported for backward compat.

No OCI integration — Endo bundles live alongside the Agoric chain's `installBundle` storage, retrieved by hash. **This is the cleanest existence-proof of "ship a code artifact by its hash, not by a name in a registry"** that exists in production today.

See [`agoric-endo/modules-and-bundling.md`](../agoric-endo/modules-and-bundling.md) and [`agoric-endo/determinism.md`](../agoric-endo/determinism.md).

## Pears / Hypercore — versioned append-only logs as "the app"

[Pears](../pears/) doesn't have a "bundle" in the traditional sense. A Pears app is a `pear://` URL that resolves to a Hypercore (signed append-only log) whose latest version is the current app code. Updates ship as new entries appended to the log; clients resolve the latest signed version and replicate it via Hyperswarm. Identity = the Hypercore's discovery key (the pubkey hash).

```bash
pear stage <link>           # publish new version to your Hypercore
pear seed <link>            # serve the app over Hyperswarm
pear run pear://<link>      # fetch + run the latest signed version
```

**Key insight:** the bundle isn't a file format — it's a versioned, signed, P2P-distributable data structure. Hypercore is closer to a content-addressed P2P registry than a bundle format. **This is the closest existing-art for Myrhiza's "ship apps over a P2P transport" story**, modulo Pears not being WASM. See [`pears/distribution.md`](../pears/) (linked from `pears/README.md`).

## ES Modules + import maps (browser-side comparator)

In the browser, the analog to "registry + bundle" is **ES Modules + import maps**:

```html
<script type="importmap">
{
  "imports": {
    "lodash": "https://cdn.jsdelivr.net/npm/lodash-es@4.17.21/+esm",
    "myorg/auth": "https://my-cdn.example.com/auth/1.0.0/index.js"
  }
}
</script>
<script type="module">
  import { sortBy } from 'lodash';
  import { authenticate } from 'myorg/auth';
</script>
```

- **HTML Standard** since the WICG `import-maps` repo was archived 2025-02-26 and folded into the [HTML spec import-maps section](https://html.spec.whatwg.org/multipage/webappapis.html#import-maps).
- **Browser support:** Baseline widely available since March 2023 (Chrome 89, Firefox 108, Safari 16.4).
- **Distribution:** modules fetched over HTTPS by URL; "registry" is "whatever CDN you point at." No central index; everything is URL-resolution.
- **Versioning:** in the URL (`lodash-es@4.17.21`).
- **Signing:** import maps support [Subresource Integrity (SRI)](https://html.spec.whatwg.org/multipage/webappapis.html#integrity-metadata) via the `integrity` field — `"integrity": {"https://...": "sha384-..."}`. This is **not author-identity signing**; it's tamper-detection only. The pubkey-signed JS module story doesn't exist standardized in browsers.

**Key insight:** the ES modules story is *pull-by-URL with optional hash-integrity*. No first-class registry, no first-class signing, no canonical bundle format — just URLs and hashes. **OCI's "name → digest" indirection is the missing piece in the browser**; OCI for native CM, ES modules + import maps + SRI for browser.

For Myrhiza's browser story (jco-transpiled components in the browser), the ES modules + import maps + SRI shape is the natural transport. See [`browser-distribution.md`](./browser-distribution.md).

## The comparison matrix

| Aspect | CM canonical | Spin | wasmCloud | Holochain | Endo | Pears | ESM + import maps |
|---|---|---|---|---|---|---|---|
| **Bundle shape** | single `.wasm` | OCI manifest + N `.wasm` layers | N separate OCI artifacts | gzip+msgpack of `mr_bundle::Bundle<M>` | base64-encoded ZIP or `nestedEvaluate` JS | versioned Hypercore | URLs in HTML |
| **Manifest format** | WIT embedded in `.wasm` | `spin.toml` (TOML) | YAML (wadm) / K8s CRD | YAML (`happ.yaml`, `dna.yaml`) | Compartment Map (JSON) | (Hypercore metadata) | import-map JSON in `<script>` |
| **Transport** | OCI registry (HTTP) | OCI registry (`spin registry push/pull`) | OCI registry (`wash oci push/pull`) | (none standard; GitHub releases) | (chain `installBundle`; Endo IPC) | Hyperswarm (P2P) | HTTP (CDN, server) |
| **Identity** | content hash (OCI digest) | OCI digest | OCI digest | DNA hash + bundle blob | bundle-hash (SHA-512) | Hypercore discovery key | URL + optional SRI hash |
| **Signing** | optional (Cosign / Notation via Referrers) | optional (Cosign) | optional (Cosign) | none native; "Verified" badge deprecated | (chain consensus on bundle hash) | Hypercore pubkey signs every entry | SRI = tamper-detect only, not author |
| **Cross-component deps** | `wac` (build-time) | `wac` (build-time) | runtime (wRPC link defs) | inlined bytes | Compartment Map (build-time) | per-app | per-URL |
| **Discovery / registry** | OCI registry by name | OCI registry by name | OCI registry by name | DevHub (deprecated) / GitHub releases | chain `installBundle` storage | DHT (Hyperswarm) | (none — just URLs) |
| **Offline-installable** | yes (artifact = full content) | yes | partial (provider deps too) | yes (everything inlined) | yes (bundle is self-contained) | partial (need Hyperswarm peer) | partial (need URLs reachable) |
| **WASM-aware?** | yes | yes | yes | yes (zomes are WASM) | no (JS only) | no (JS only) | no |

## Implications for Myrhiza

**Adopt:** CM `.wasm` (possibly wac-composed) as the canonical bundle shape. This is the lowest-friction choice given the CM commitment.

**Transport choice is open:** OCI registry + iroh blob store + Hypercore-style append-only logs are not mutually exclusive. The brief is to pick the **canonical name resolution** path. Options worth considering:
- **OCI-canonical:** Myrhiza apps are OCI artifacts by URL (`ghcr.io/foo/myapp:0.1.0`). Discovery via existing registry tooling. P2P seeding optional. (Spin/wasmCloud path.)
- **Hash-canonical:** Myrhiza apps are referenced by digest (`b3:abc123...`). Resolution is "find a peer / registry serving this hash." P2P-native. (Endo-flavored.)
- **Hybrid:** OCI for human-readable discovery; once resolved, the digest is what propagates over iroh. (Probably the right answer for Myrhiza.)

**Don't repeat Holochain's mistake:** the `.happ` / `.webhapp` decision to inline everything as bytes makes apps undiscoverable and unsignable. CM-on-OCI is structurally better.

**Don't repeat the import-maps mistake:** SRI is not author-signing. Myrhiza needs author identity in the trust chain, not just hash-integrity.

**Look closely at Endo's bundle-hash story** for the P2P / chain-consensus path. The "ship a hash, peers fetch the bytes" pattern works in production today. See [`agoric-endo/modules-and-bundling.md`](../agoric-endo/modules-and-bundling.md).

**Look closely at Pears for the P2P-versioned-app story.** Hypercore-as-app-history is the most production-tested "P2P versioned-app" we have. The mismatch with Myrhiza's WASM commitment is real, but the design lessons translate. See [`pears/`](../pears/).

## Sources

- WebAssembly Component Model: <https://github.com/WebAssembly/component-model>
- Spin spec / SIP-008: <https://github.com/spinframework/spin/blob/main/docs/content/sips/008-using-oci-registries.md>
- wasmCloud `wash oci`: <https://wasmcloud.com/docs/cli/wash>
- Holochain `mr_bundle`: <https://crates.io/crates/mr_bundle>
- Endo `compartment-mapper`: <https://github.com/endojs/endo/tree/master/packages/compartment-mapper>
- Endo `bundle-source`: <https://github.com/endojs/endo/tree/master/packages/bundle-source>
- Pears `pear://` link format: <https://docs.pears.com>
- HTML import maps: <https://html.spec.whatwg.org/multipage/webappapis.html#import-maps>
- Subresource Integrity: <https://html.spec.whatwg.org/multipage/webappapis.html#integrity-metadata>
