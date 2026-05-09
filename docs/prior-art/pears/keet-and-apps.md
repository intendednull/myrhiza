**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Keet messenger as the production mobile-P2P data point

# Keet and the apps built on Pear

Keet is the only consumer P2P app on the Holepunch stack with non-trivial adoption. Everything else in the Pear gallery is research-grade or developer-tooling (see [`apps.md`](apps.md)). Keet matters to Myrhiza for one reason: it is the existing-proof that the Hypercore + Hyperswarm + Bare combination can reach the iOS App Store and Google Play and stay there for over three years. Holochain's Volla is the closest analog — also mobile, also P2P, smaller scale. See [`../holochain/`](../holochain/).

## What Keet is

Keet is an end-to-end-encrypted P2P messenger. Text, audio messages, voice calls, video calls, file transfer, group rooms, broadcast rooms. Available on iOS, Android, macOS, Windows, Linux. iOS bundle id `io.keet.app`, App Store id `6443880549`, "Keet — Private Encrypted Chat" on the App Store, current version 4.14.0 (released 2026-04-29) — confirmed via `itunes.apple.com/lookup?id=6443880549`. Listed under developer "Holepunch Inc" (artist id 1650235666). Originally released 2023-01-30. Available in 16 languages.

Keet's invariants on the marketing page and in the App Store description:

- **No accounts, no phone numbers, no email.** Identity is a device-local 24-word seed (recovery phrase, crypto-wallet style), per [keet.io](https://keet.io) and [support.keet.io](https://support.keet.io).
- **No central servers for messaging or media.** Messages, files, audio, and video travel device-to-device. End-to-end encrypted — "Not even Keet can access your conversations" (App Store description).
- **Invite-by-link only.** Rooms cannot be discovered by search; you join through a `keet://` link, a QR code, or a curated public-communities directory ([support.keet.io/keet-groups/joining-groups](https://support.keet.io/keet-groups/joining-groups)).
- **Unlimited file size.** Direct device-to-device transfer with no server compression or quotas (per the keet.io marketing page; not independently verified).

Caveats on the "no servers" claim — DHT bootstrap nodes still exist (Hyperswarm bootstraps off a small set of well-known DHT seed nodes operated by Holepunch) and APNs / FCM are still on the path for mobile push. Both are addressed below. The marketing claim is a meaningful *architecture* statement, not a literal "zero infrastructure" statement.

## Closed-source on an open-source stack

Critical for Myrhiza's reading: **the Keet messenger is itself closed source.** The runtime stack underneath it (Hypercore, Hyperswarm, HyperDHT, Hyperdrive, Hyperbee, Autobase, Bare, Pear) is fully open source under the `holepunchto` GitHub org. The app is not. There is no `holepunchto/keet`, `holepunchto/keet-desktop`, or `holepunchto/keet-mobile` repository — verified via `gh api` returning 404 for each. The Keet-related repositories that *do* exist under `holepunchto` are infrastructure-only:

| Repo | Role | License |
|---|---|---|
| [keet-mobile-releases](https://github.com/holepunchto/keet-mobile-releases) | Public changelog only — no source | none |
| [keet-appling](https://github.com/holepunchto/keet-appling) | Desktop application shell (CMake wrapper that loads the proprietary Keet bundle from a `pear://` link) | Apache-2.0 |
| [keet-appling-next](https://github.com/holepunchto/keet-appling-next) | Successor shell | none on file |
| [keet-identity-key](https://github.com/holepunchto/keet-identity-key) | Hierarchical-deterministic key derivation library used by Keet identity | open |
| [blind-pairing-core](https://github.com/holepunchto/blind-pairing-core) | The pairing primitive Keet uses to bootstrap room membership | open |
| [keet-prefs](https://github.com/holepunchto/keet-prefs) | Preferences schema | open |

The application code — UI, room logic, call engine, notification handling, mobile shell — is delivered as a signed Pear app bundle and is not published. This is a deliberate posture: **the substrate is open, the flagship reference app is proprietary.** It is exactly the model Myrhiza could end up adopting if a flagship app emerges atop the runtime, and worth holding up as a precedent rather than dismissing — see [Implications](#implications) below.

The mobile build is React Native + Expo: the changelog mentions `expo-blur`, `Expo background-task`, and the `Drop android devices support for armeabi-v7a architecture` line from [v3.20.0](https://github.com/holepunchto/keet-mobile-releases/blob/main/CHANGELOG.md). The desktop build is the `keet-appling` shell loading a pear://-linked bundle.

## Architecture: the room model

A Keet room corresponds to one Hypercore writer (DM, single-author broadcast) or one Autobase merging multiple writer Hypercores (group room). The room key is the access-control primitive — it is both the discovery key (used to find peers on Hyperswarm) and the secret needed to derive the Hypercore encryption key.

Joining a room uses the [`blind-pairing-core`](https://github.com/holepunchto/blind-pairing-core) flow:

1. The inviter generates a fresh signing keypair, shares `{ discoveryKey, seed }` with the invitee out-of-band (via the `keet://` invite link, QR code, or another already-shared room).
2. The invitee creates a request, signs it with the invitation keypair, and encrypts it under a key derived from the invite's public key.
3. A current member of the room decrypts the request, verifies the signature, evaluates the user data, and either accepts (returning `{ key, encryptionKey }`) or denies.
4. The invitee verifies that the returned `key` matches the `discoveryKey`, confirming the responder is actually a current member.

Net effect: invite links are *bearer tokens for one membership grant*, not the room key itself. They expire (per the Keet support docs the user picks the expiry from a dropdown). Once accepted the invitee receives the actual room key and encryption key, joins the Autobase, and replicates state from peers via Hyperswarm. This is meaningfully different from a Signal-style "share my phone number" flow — there is no global namespace, no directory, no rendezvous server.

For state convergence in group rooms with many writers, Autobase linearises operations using a deterministic causal-order linearisation across the merged Hypercores. State convergence requires every peer to apply the same linearisation deterministically — this is the same property Myrhiza needs from `state-apply` and a useful precedent (see [`./hypercore-stack.md`](hypercore-stack.md)).

## E2E encryption

Three layers stack:

1. **Transport.** Hyperswarm uses Noise IK over UDX (Holepunch's UDP-based reliable stream protocol; see [`udx-native`](https://github.com/holepunchto/udx-native)) for peer-to-peer connections after DHT-mediated NAT hole punching. Connections to DHT bootstrap nodes are also Noise-encrypted.
2. **Storage / replication.** Hypercore's "encrypted core" feature wraps every block on disk and on the wire under a symmetric key (typically the room's `encryptionKey`). A Hypercore peer without the key cannot read block content even if they replicate it.
3. **Identity / membership.** Keet identity is a hierarchical-deterministic keyset rooted in the device's 24-word seed (per `keet-identity-key`). Account recovery on a new device = restore the seed, re-derive the keys, ask peers to re-grant room membership.

Keet does not (publicly) document forward secrecy at the message level — the symmetric room key is long-lived for the life of the room. Compromise of an active member's key material exposes prior messages stored on their device. Compare Signal's per-message ratchet, which Keet does not implement. This is a real tradeoff, not a missing-feature: the multi-writer-replicated-log model (Autobase) is hard to combine with per-message forward secrecy.

## Voice and video

Keet does HD audio and video calls, including group calls. The architecture is **WebRTC for the media plane, Hyperswarm for the signalling/discovery plane**. The Pear ecosystem includes `webrtc` modules that integrate the WebRTC PeerConnection lifecycle with Hyperswarm-mediated peer discovery; the room key + member set already known to participants is reused to coordinate the offer/answer/ICE flow without a STUN/TURN/signalling server.

The changelog evidence is consistent with WebRTC: explicit references to "the new call engine" (v4.12.0, v4.12.1), iOS CallKit integration ("Fixed iOS background CallKit crash from a dangling pointer", v4.14.0), camera/headset/audio-output handling typical of a WebRTC-native client. The exact crate / native library is not public — the call engine is part of the proprietary Keet code.

What this means for Myrhiza: a serverless P2P messenger can ship voice + video on consumer mobile *if* it pairs an existing media stack (WebRTC) with the P2P discovery layer. Building media-plane primitives from scratch is not what made Keet shippable.

## Mobile constraints and how Keet handles them

Push notifications are the hard problem for serverless P2P on iOS / Android. Apple requires APNs-mediated background wake; Google's Play Store requires FCM for reliable background delivery. Keet does *not* eliminate this — there is no way to. What Keet does is:

- **Run a push relay.** A Holepunch-operated relay subscribes to Hyperswarm events for peers that have opted in and forwards a content-free wake notification through APNs / FCM. The notification carries no message text, just an opaque pointer; the device wakes, opens a Hyperswarm connection to its peers, and pulls the actual encrypted blocks. The push relay sees that "device X has a pending message" but cannot read content (it does not have the room key).
- **Coordinate with the system push handler.** "Made push notifications more reliable on Android by coordinating the push handler with a lockfile" (v4.3.1) — there is a hot path between the FCM-delivered wake and the Hyperswarm connection-opening code that needed serialisation work.
- **Background call lifecycle.** "Android auto-end in background" (v3.14.0), "Call auto-ending when answered multiple times from the iOS lock screen" (v4.13.0), "iOS now toggles video off in the background to prevent frozen frames" (v4.13.0), "Fixed Expo background-task lookup that prevented some background work from running" (v4.14.0). The pattern over three years of the changelog is many small fixes — backgrounding under iOS / Android constraints is the most fragile surface in the app.

The honest read: **Keet's "no servers" property holds for messaging content but is mediated by a Holepunch-operated push relay for mobile wake-up.** This is a structurally unavoidable concession on iOS — APNs is non-negotiable. Myrhiza will face the same constraint and should plan for the same shape: a thin push relay run by the runtime operator (or by individual app operators), forwarding content-free wakes only, with all message content fetched device-to-device after wake. The fact that Keet hasn't solved this without a relay is not a Keet failure — it's the platform reality.

## Adoption — honest numbers

This is where the marketing diverges most sharply from observable reality. As of May 2026:

| Source | Metric | Value |
|---|---|---|
| App Store (US storefront) `itunes.apple.com/lookup?id=6443880549` | Total user ratings | 99 |
| App Store | Average rating | 4.59 / 5 |
| AppBrain (Google Play data) | Total Android downloads (lifetime) | ~690,000 |
| AppBrain | Last-30-day Android downloads | ~110,000 |
| AppBrain | Android ratings | ~1,000 (4.31 / 5) |

The "millions of times downloaded across mobile and desktop" line that appears in some Holepunch press materials is unsubstantiated by App Store / Play Store telemetry. The realistic order of magnitude is **a few hundred thousand all-time installs, low-tens-of-thousands of monthly active users, with a Q1 2026 spike** (per AppBrain's "150%+ download spike Q1 2026" datapoint) likely driven by Tether-adjacent press cycles.

This is, however, **one of the larger verified consumer mobile P2P deployments on a custom stack** — Briar (Tor-routed, Android-only, larger total install base but similarly-modest MAU), Volla on Holochain (smaller still), Delta Chat (large but the network is email-over-SMTP with iroh layered on for realtime; see [`../iroh/apps.md`](../iroh/apps.md)). The Keet number is small compared to mainstream messengers (Signal ~70M MAU, WhatsApp ~3B MAU) but it is the existence proof that the underlying stack ships and has run for 3+ years on iOS and Android with no central message store.

For Myrhiza: a successful P2P-app runtime does not need to dethrone WhatsApp. The bar is shipping a real app to the App Stores and keeping it running. Keet clears that bar. Volla on Holochain clears it less convincingly (smaller scale, narrower platform).

## Implications for Myrhiza

1. **A flagship app sets the direction of the runtime.** Keet has driven much of what Pear runtime looks like — `pear-wakeups` (link wakeups), `keet-appling` (the desktop shell pattern), `blind-pairing-core` (the invite primitive), `keet-identity-key` (HD identity). When the runtime is built around a real app, the runtime's design surfaces real problems instead of speculative ones. Myrhiza should pick a flagship app *target* (even a hypothetical one) early and use it as the forcing function for capability design — see CLAUDE.md "Capabilities are the only host surface."

2. **Closed flagship on open substrate is a viable model.** Keet is proprietary and Hypercore is MIT. This has not blocked third-party Pear-app developers because the substrate gives them everything they need to build their own apps. Myrhiza could end up here — a closed flagship by us or a partner sitting on a fully open kernel + capability layer + state-machine kit. *Choice is preserved as long as the kernel and APIs stay open-source and self-hostable.* Document this option-space in [`./commercial.md`](commercial.md).

3. **Mobile push is a runtime-operator concern, not an app-author concern.** Keet's Pear runtime exposes push wakeups to apps but the actual APNs / FCM relay is run by Holepunch. This is the correct division of labour: apps cannot be expected to set up APNs certificates and FCM projects individually, and individual users cannot be expected to either. The runtime operator (whoever ships the kernel binary to the device) is the natural party. Myrhiza should plan for a `runtime-push` capability that lets apps subscribe to wake events without touching APNs / FCM directly — this becomes part of the kernel's I/O surface.

4. **State-apply determinism maps directly onto Autobase linearisation.** Autobase's deterministic linearisation across multiple writer Hypercores is structurally the same problem as Myrhiza's `(prior state, event) → next state` purity requirement. Look at how Autobase handles deterministic ordering across forking writers — that's a directly transferable design pattern. See [`./hypercore-stack.md`](hypercore-stack.md) for detail.

5. **WebRTC + Hyperswarm beats inventing a media stack.** Keet ships voice/video by reusing the WebRTC media plane and replacing only the signalling layer with Hyperswarm-mediated discovery. Building a P2P media plane from scratch is a multi-year effort; building a P2P signalling layer is achievable in months. If Myrhiza wants voice/video apps on the runtime, the same shape applies — expose enough capability that an app author can run WebRTC and reach peers, don't build a media stack inside the kernel.

6. **Honest scale matters more than ambitious scale.** Keet at low-tens-of-thousands MAU class is real production. Most of the "P2P-runtime" projects with bigger marketing claims have less actual deployment than Keet. Don't oversell. See [`./critiques.md`](critiques.md) for the matching honest-read of "no servers" marketing reality.

## See also

- [`./pear-runtime.md`](pear-runtime.md) — the Pear runtime that hosts Keet and other Pear apps
- [`./bare-runtime.md`](bare-runtime.md) — the embedded JavaScript runtime that runs the Keet mobile bundle
- [`./hypercore-stack.md`](hypercore-stack.md) — Hypercore, Hyperbee, Hyperdrive, Autobase: the data layer
- [`./hyperswarm.md`](hyperswarm.md) — DHT + NAT hole punching + Noise transport
- [`./apps.md`](apps.md) — the broader Pear-app ecosystem beyond Keet
- [`./commercial.md`](commercial.md) — Holepunch the company; Tether funding; revenue model
- [`./lessons.md`](lessons.md) — distilled lessons for Myrhiza
- [`../holochain/`](../holochain/) — Volla as the closest mobile-P2P comparison
- [`../iroh/apps.md`](../iroh/apps.md) — Delta Chat as a transport-only iroh adopter at much larger scale
- [`../spritely-ocapn/apps.md`](../spritely-ocapn/apps.md) — the no-flagship-app contrast case
