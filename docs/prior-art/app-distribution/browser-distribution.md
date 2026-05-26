**Date:** 2026-05-22
**Status:** active
**Subject:** Browser-side distribution — ES modules, import maps, Subresource Integrity, and the jco transpile path. The browser's answer to OCI.

# Browser distribution

The browser has no OCI registry. It has URLs. The question is whether URL-based delivery can carry the same load OCI carries natively. The verdict is "mostly yes, but no first-class signing."

## ES Modules — the loading mechanism

The browser-native module loader is the [ECMAScript Modules](https://tc39.es/ecma262/#sec-modules) spec implemented in all major browsers. Native syntax:

```html
<script type="module">
  import { sortBy } from 'https://cdn.jsdelivr.net/npm/lodash-es@4.17.21/+esm';
  import { authenticate } from '/static/auth.js';
</script>
```

The `import` specifier is either an **absolute URL**, a **relative path**, or (with import maps in play) a **bare specifier** that resolves through the import map.

ES Modules:
- **First shipped in browsers:** 2018 (Chrome 61 / Firefox 60 / Safari 11).
- **Are universally Baseline since:** 2020.
- **HTTP transport:** module files MUST be served with `Content-Type: application/javascript` (or `text/javascript`); CORS applies; HTTP/2+ improves the deep-import-graph performance hit.

## Import maps — the resolution mechanism

[Import maps](https://html.spec.whatwg.org/multipage/webappapis.html#import-maps) (HTML Standard) let pages declare bare-specifier → URL mappings:

```html
<script type="importmap">
{
  "imports": {
    "lodash": "https://cdn.jsdelivr.net/npm/lodash-es@4.17.21/+esm",
    "myorg/auth": "https://my-cdn.example.com/auth/1.0.0/index.js"
  },
  "scopes": {
    "/admin/": {
      "lodash": "https://different-cdn.example.com/lodash/+esm"
    }
  },
  "integrity": {
    "https://cdn.jsdelivr.net/npm/lodash-es@4.17.21/+esm": "sha384-abc..."
  }
}
</script>
<script type="module">
  import { sortBy } from 'lodash';  // resolves to the URL above
</script>
```

### Versions verified 2026-05

- **WICG repo archived:** 2025-02-26. Spec lives in the HTML Standard.
- **Baseline status:** widely available since March 2023.
- **Browser support:**
  - Chrome 89 (2021-03-02)
  - Firefox 108 (2022-12-13)
  - Safari 16.4 (2023-03-27)
- **Multi-import-map support** (added 2024-ish) lets pages declare multiple `<script type="importmap">` blocks merged in order; older browsers allow only one.
- **`integrity` field** (added 2024) carries [Subresource Integrity](https://html.spec.whatwg.org/multipage/webappapis.html#integrity-metadata) hashes — but applies only to top-level imports, not transitive (a foot-gun).

### What import maps don't do

- **No registry.** Resolution is URL-based; there's no first-class "go fetch from registry X" concept. CDN URLs are the de-facto registry.
- **No versioning.** The version is in the URL (`lodash@4.17.21`); if you don't pin in the URL, you don't pin.
- **No content negotiation.** No equivalent of OCI's `Accept` header for media-type-specific responses.
- **No author signing.** The `integrity` field is hash-only.

## Subresource Integrity (SRI) — the tamper-detection layer

[SRI](https://html.spec.whatwg.org/multipage/webappapis.html#integrity-metadata) lets a page declare cryptographic hashes for fetched resources:

```html
<script src="https://cdn.example.com/lib.js"
        integrity="sha384-Bz1YNs/EVwabhUSdaJTQhDi7G3xKQXdvjkqDjP8C5Gd2x..."
        crossorigin="anonymous"></script>
```

The browser fetches the resource, computes the hash, and **refuses to execute if it doesn't match**. Algorithms: `sha256`, `sha384`, `sha512` (multiple may be listed; browser picks strongest).

In import maps, the `integrity` field carries the same hashes per-URL:

```json
"integrity": {
  "https://cdn.example.com/lib.js": "sha384-Bz1..."
}
```

### What SRI gives you

- **Tamper detection in transit.** If the CDN serves wrong bytes (compromise, MITM, accidental corruption), execution is blocked.
- **Cache-busting / pinning.** A specific hash pins a specific build of a specific version. URL changes that update the hash require explicit page update.
- **No trust burden on the CDN.** A compromised CDN that serves different bytes is detected, not blindly trusted.

### What SRI doesn't give you

- **Author identity.** SRI says "this is the hash someone embedded in the HTML." It doesn't say *who* signed it or *who's accountable*. Compare Cosign-keyless, which binds a signature to an OIDC identity.
- **Hash discovery.** You need the hash in the HTML before you load. No "fetch and learn the hash from the registry" flow.
- **Revocation.** If a signed-into-HTML hash turns out to be compromised, every page hosting that HTML must be updated. There's no "revoke the signature" mechanism.
- **Transitive integrity.** A SRI'd module that itself imports other modules — those imports are NOT covered. Browsers do not propagate SRI through the dep graph. This is a known gap; proposals exist (e.g. [W3C SRI Level 2](https://www.w3.org/TR/SRI/#future)) but nothing has shipped.

## The jco transpile path

[jco](../jco/) is the Bytecode Alliance's tool that transpiles a CM component to JavaScript-plus-WASM that runs in browsers + Node. For Myrhiza's browser-peer story, jco is the only viable path; see [`jco/README.md`](../jco/README.md).

The transpile output is **a directory of JS + .wasm files** that exports the component's interface as a JS module:

```
my-component.transpiled/
├── my-component.js           # the JS shim, ESM
├── my-component.core.wasm   # core WASM module(s)
├── interfaces/
│   ├── wasi-clocks-0.2.0.js
│   ├── wasi-io-0.2.0.js
│   └── ...
└── package.json
```

Distribution: any ESM-aware host. Specifically:
- **Static CDN.** Drop the directory at `https://my-cdn.example.com/components/my-component/0.1.0/`; pages reference `import { greet } from '/components/my-component/0.1.0/my-component.js'`.
- **npm.** `npm publish` the transpiled directory; pages or bundlers consume from npm.
- **Inside an OCI artifact.** Wrap the directory in a tarball, push as OCI; runtime fetches and unpacks. Combines OCI's registry semantics with the browser's loading model.

**The signing gap:** none of these paths inherits OCI 1.1 + Cosign/Notation signing for free. The browser sees an ES module URL; it has no idea there's a signed OCI artifact upstream. Practical options:

1. **SRI hashes in import maps.** Computes a hash of the transpiled JS bundle; pages include it. Tamper-detects but doesn't carry author identity.
2. **Cosign-sign the OCI artifact upstream.** Page operators verify before unpacking-and-serving. Trust travels server-side, not client-side.
3. **Signed JSON manifest.** Cosign-sign a manifest declaring the transpile bundle's hashes; pages fetch + verify the manifest before loading modules. Custom integration; no browser-native support.

None of these is great. **The browser's lack of first-class author-identity signing is a real gap** — see [`open-problems.md`](./open-problems.md) §browser-signing.

## ESM + import maps vs OCI — the comparison

| Aspect | ES Modules + import maps + SRI | OCI 1.1 + Cosign |
|---|---|---|
| **Identity** | URL + optional SRI hash | content digest |
| **Discovery** | URL (out-of-band) | registry name lookup |
| **Tamper detection** | SRI (hash match) | digest match |
| **Author identity / signing** | none (SRI is hash-only) | Cosign / Notation via Referrers |
| **Versioning** | in URL path | OCI tag + digest |
| **Transitive integrity** | NOT propagated | covered by full-bundle digest |
| **Revocation** | edit HTML | Sigstore log / Notation policy |
| **Registry standardization** | none (CDN URLs) | OCI distribution-spec 1.1 |

**The verdict:** OCI is strictly more capable. The browser has no equivalent for author-identity signing or transitive integrity. Myrhiza's browser story will have to either accept these gaps or build custom verify-before-execute machinery on the page side.

## Implications for Myrhiza

**The browser is the weak link in any spec involving signing.** OCI + Cosign works cleanly for native peers; the browser path can ship JS modules with SRI but no author signing. Spec authors who write "all Myrhiza apps are signed by their author" need to add the caveat "except in browsers where SRI tamper-detects but doesn't authenticate the signer."

**Bundle the verify step into the loader.** If Myrhiza's browser peer is a service worker / web component that loads other Myrhiza apps, *that loader* can do the OCI fetch + Cosign verify before handing the bundle to the browser's ES module loader. The browser stays in "load these blessed bytes" mode; the trust check happens upstream.

**Don't try to fix browser signing in Myrhiza spec.** This is a multi-decade browser-platform gap that import maps + SRI Level 2 may eventually address. Myrhiza can layer on top, not replace.

**The "host page" trust model matters.** If a Myrhiza browser peer loads `https://myrhiza.example.com/peer.js`, the user is trusting `myrhiza.example.com` (and whatever served the page) as much as the signer of the loaded components. This is the existing browser trust model and unavoidable.

**Import maps + SRI are the right *transport*-layer choice.** Use them. They're Baseline. They're the cleanest browser-native distribution mechanism for ESM-shaped jco output.

## Sources

- HTML Standard import maps: <https://html.spec.whatwg.org/multipage/webappapis.html#import-maps>
- Subresource Integrity: <https://html.spec.whatwg.org/multipage/webappapis.html#integrity-metadata>
- MDN import maps: <https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script/type/importmap>
- WICG import-maps (archived 2025-02-26): <https://github.com/WICG/import-maps>
- jco: <https://github.com/bytecodealliance/jco>
- ES Modules: <https://tc39.es/ecma262/#sec-modules>
- W3C SRI Level 2: <https://www.w3.org/TR/SRI/>
- es-module-shims polyfill: <https://github.com/guybedford/es-module-shims>
