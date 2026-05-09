**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — apps and tools beyond Keet

# The Pear app ecosystem outside Keet

Keet is the consumer flagship (see [`keet-and-apps.md`](keet-and-apps.md)). Outside Keet, the Pear ecosystem is a mix of (1) one second-party app from Tether Data, (2) a handful of Holepunch-built CLI tools, and (3) developer tooling. There is no third-party-developer-built consumer app at non-trivial scale on Pear as of May 2026. Be honest about this — the runtime is real but its non-Keet adoption is research-grade.

## The official showcase

The [`docs.pears.com`](https://docs.pears.com/) homepage lists exactly two apps in its "Showcase" section as of May 2026:

| App | What it is | Status |
|---|---|---|
| **[Keet](https://keet.io)** (`pear://keet`) | Encrypted P2P messenger, mobile + desktop | Shipping; see [`keet-and-apps.md`](keet-and-apps.md) |
| **[PearPass](https://pass.pears.com)** (`pear://pass`) | P2P password / secrets manager | Shipping; iOS and desktop |

That is the full curated set. Compare with Holochain's hApp store (`apps.holochain.org` lists ~15–20 apps; see [`../holochain/apps.md`](../holochain/apps.md)) — Pear's curated showcase is deliberately narrower.

### PearPass

PearPass is built by **Tether Data** (a Tether-affiliated entity, not Holepunch directly) and uses Pear runtime for cross-device vault sync without a server. Open source — confirmed via [docs.pass.pears.com](https://docs.pass.pears.com/). On the iOS App Store as `id6752954830` ("PearPass App", much newer than Keet, fewer ratings).

Architecturally similar to Keet — a Hypercore (or set of Hypercores via Autobase) per vault, room-key-based access control, Hyperswarm for device-to-device replication. The interesting design question is multi-device sync without a server: PearPass uses the same `blind-pairing-core` invite-and-grant flow that Keet uses for room membership, applied to the case of "this person's two devices both authorised to read this vault."

PearPass is the second proof point that the substrate works for non-messenger consumer apps. It is much smaller than Keet — App Store evidence shows tens to low hundreds of ratings, not hundreds to thousands. It is shipping, but it is not a network effect that pulls users in. For Myrhiza, PearPass is useful as a "what does a non-messenger app look like on a Hypercore-style runtime" datapoint, not as scale evidence.

## Holepunch-built CLI tools

The [`docs.pears.com/index.html#tools`](https://docs.pears.com/) "Tools" section lists CLI utilities that the Holepunch team ships as `npm install -g`-able binaries built on the same Hyperswarm + HyperDHT primitives as Keet:

| Tool | Description | Use case |
|---|---|---|
| [Hyperbeam](https://docs.pears.com/tools/hyperbeam/) | One-shot encrypted pipe between two terminals identified by a passphrase | Send a file, share stdout, ad-hoc pipe |
| [Hypershell](https://docs.pears.com/tools/hypershell) | E2E-encrypted shell over Hyperswarm — connect to a remote shell using a key | SSH replacement that doesn't need a public IP |
| [Hyperssh](https://docs.pears.com/tools/hyperssh) | "Run SSH over Hyperswarm" — wrapper that tunnels real ssh through a Hyperswarm-mediated transport | Same as Hypershell but uses the local OS's `ssh` |
| [Hypertele](https://docs.pears.com/tools/hypertele) | "Swiss-knife proxy powered by HyperDHT" — exposes local services to peers over the DHT | Reverse-proxy without a public IP or port forwarding |
| [Drives](https://docs.pears.com/tools/drives) | CLI for working with Hyperdrive instances | File-sync workflows |

These exist primarily to demonstrate "you can build an X with the substrate" — they are not products with users. Hypershell-as-SSH-replacement is the most practically useful and the most directly compelling for a developer evaluating the stack: you can `npm install -g hypershell`, generate a keypair, and shell into your laptop from a coffee-shop wifi without configuring a router or running a server. That's the Hyperswarm value proposition in 30 seconds.

For Myrhiza these tools are reference implementations of "what does a single-purpose CLI app on the runtime look like" — small enough to read end-to-end, real enough to actually use. The pattern they share: each tool is a thin shell over one Hyperswarm topic + one Hypercore-derived primitive, with key management as the user-visible surface.

## Third-party Pear apps

There is no equivalent of an "awesome-pears" curated registry that would list third-party apps comprehensively. Search for `pear://` URLs across GitHub turns up a small set of toy / demo / hobbyist apps. The structurally interesting third-party work is mostly at the **module** layer (publishing `pear-`prefixed modules to npm) rather than the **app** layer:

- The `holepunchto` org has ~600 public repos; most are Hypercore / Hyperswarm / Pear modules that compose into apps, not apps themselves.
- Notable non-app Pear modules: `pear-electron` (Electron-based desktop apps on Pear; archived May 2026), `pear-expo-hello-world` (template for React Native + Expo on Pear; archived), `pear-message` / `pear-messages` (inter-app pattern-matched messaging primitives), `pear-wakeups` (link wakeups external to the app), `pear-pipe` / `pear-run` (parent-child Pear app composition).
- `gasolin/keetlink` — third-party HTTP-to-`keet://` link-share wrapper, ~one-off contribution.
- `pear-android-hello-world`, `pear-ios-hello-world` (and the related `bare-android` / `bare-ios` projects in the Holepunch org) — the developer onboarding templates for mobile.

The honest framing: **the Pear ecosystem outside Keet is a developer ecosystem, not an end-user ecosystem.** People are publishing modules for other developers. The handful of apps that exist are demos, tools, or the two showcase apps above. This is structurally similar to Holochain's situation pre-2025 (see [`../holochain/apps.md`](../holochain/apps.md)) and not structurally different from where iroh sits today (see [`../iroh/apps.md`](../iroh/apps.md)) — most P2P-runtime projects have one or two real apps and a long tail of experiments.

## Developer tooling — the `pear` CLI

Pear apps are developed and distributed using a single CLI binary. The workflow ([source](https://docs.pears.com/guide/sharing-a-pear-app.html)):

```
pear init <template>      # scaffold a new Pear app from a template
pear run <link>           # run a remote Pear app from its pear:// link, or a local dir
pear stage <channel>      # build / mirror local files into Pear's app storage
                          #   yields a pear:// link encoding the app key
pear seed <channel>       # announce the app on the DHT via Hyperswarm
                          #   peers with the link can now fetch and run it
pear release <channel>    # cut a release the runtime treats as "stable"
```

Distribution model: an app's identity is its **app key** (a deterministic public key derived during staging from app name + channel). The pear:// URL encodes this key. There is no centralised store, no review process, no signing authority above the app developer — anyone with a pear:// link can `pear run` it and the runtime verifies the content against the key as it streams.

Sparse replication is the bandwidth optimisation that makes this practical: only the files on the critical loading path are fetched eagerly, the rest stream in on demand. This is inherited from Hyperdrive's built-in sparse-replication. For Myrhiza this is a directly transferable pattern — apps as content-addressed bundles, lazy fetch, key-as-identity.

Reseeding: any peer that has run an app can reseed it. The original developer can go offline once enough peers have replicated. This is meaningful resilience compared to a centralised app store (Google removed Briar from the Play Store at one point — there is no equivalent removal action against a `pear://` app), but it is also unmoderated and unaudited (see [`./critiques.md`](critiques.md) and [`./open-problems.md`](open-problems.md)).

## "Discovery" without a directory

How does a user find a new Pear app? The mechanism set is small:

1. The official Pears.com showcase (curated by Holepunch — Keet, PearPass, and that is currently it).
2. An out-of-band shared `pear://` link — a developer tweets it, a friend DMs it, a documentation page lists it. The link *is* the discovery channel.
3. Stumbling onto a GitHub repository that publishes a `pear://` link in its README.

There is **no search**, no rankings, no review aggregation, no app-store-style browsing. This is structurally identical to the early-web "you find a website by being given the URL" pattern — and it is the same property Holochain's hApp distribution has (see [`../holochain/distribution.md`](../holochain/distribution.md)) and the same property Spritely's apps would have if Spritely had apps (see [`../spritely-ocapn/apps.md`](../spritely-ocapn/apps.md)).

For Myrhiza this is a real design decision, not a placeholder. **Discovery-by-hash is the *only* discovery model compatible with "no servers."** Any app-store-like directory is a centralised service, run by someone, that can be censored, taken down, or rate-limited. The tradeoff is real: discovery becomes social (you find apps via people who tell you about them) rather than algorithmic (you find apps via search and ranking). Keet has been able to grow on word-of-mouth + crypto-Twitter network effects; whether that scales to Myrhiza's apps depends on whether the apps have a comparable distribution channel.

## Implications for Myrhiza

1. **One flagship is enough to validate a runtime.** Pear has Keet and arguably PearPass, period — and that has been enough to drive three years of runtime development with shippable releases. Myrhiza does not need to launch with five apps; it needs to launch with one app the team uses every day.

2. **The CLI workflow is good prior art.** `pear init` / `pear run` / `pear stage` / `pear seed` is a clean four-verb interface that maps onto the lifecycle Myrhiza needs: bootstrap, execute, package, distribute. Don't reinvent — borrow the verb shape, adapt to Myrhiza's WASM-component bundle format. See [`./pear-runtime.md`](pear-runtime.md) for the verb walkthrough.

3. **Sparse replication is not optional.** If apps are content-addressed bundles loaded over P2P, eager-loading the entire bundle on first run is unworkable for any non-trivial app. Hyperdrive's lazy-fetch pattern is what makes Pear apps actually launch in seconds rather than minutes. Myrhiza's bundle-distribution layer needs the same property — only the WASM components on the critical path should fetch on first run.

4. **Discovery-by-hash is the price of "no servers" — accept it explicitly.** Don't promise an app store. The discovery model is "someone shares the bundle hash with you." This makes the social fabric of the user network the only marketing channel — which is a feature for privacy / censorship resistance and a real growth constraint to plan for. See [`./lessons.md`](lessons.md).

5. **A developer ecosystem can precede a user ecosystem.** Pear's reality today is "developers publishing modules for other developers" + "two real apps." This is fine; it took Holochain almost a decade to get to "many apps shipping" (see [`../holochain/apps.md`](../holochain/apps.md)). Myrhiza should expect the same shape — the runtime ships first, real apps second, third-party apps third.

## See also

- [`./keet-and-apps.md`](keet-and-apps.md) — Keet, the only Pear app with non-trivial adoption (low-tens-of-thousands MAU class)
- [`./pear-runtime.md`](pear-runtime.md) — the runtime that loads `pear://` apps + the `pear init`/`run`/`stage`/`seed` CLI walkthrough
- [`./bare-runtime.md`](bare-runtime.md) — the embedded JS runtime underneath Pear
- [`./commercial.md`](commercial.md) — Holepunch / Tether-Data / who builds what
- [`./critiques.md`](critiques.md) — the "no servers" marketing reality check
- [`../holochain/apps.md`](../holochain/apps.md) — comparable mobile-P2P ecosystem (slower / smaller)
- [`../iroh/apps.md`](../iroh/apps.md) — comparable transport-layer ecosystem (transport-only adopters)
