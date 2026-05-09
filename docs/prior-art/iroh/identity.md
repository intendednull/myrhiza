**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — identity model

## NodeID = Ed25519 public key

An iroh endpoint is identified by the public half of an Ed25519 keypair. The docs ([endpoints concept](https://docs.iroh.computer/concepts/endpoints)) state plainly: "Each endpoint in iroh has a unique identifier (`EndpointID`) created as a cryptographic key. … the public half of an Ed25519 keypair." The corresponding private key is held only on the endpoint and used to sign and decrypt at the QUIC layer; every connection is end-to-end encrypted with no certificates to manage.

Note the recent rename: `NodeId` → `EndpointId`. The 0.94 release ("The Endpoint Takeover") shifted the public type names; the 32-byte raw identifier semantics did not change. Older docs, blog posts, and the Hacker News thread referenced below still say "NodeId" — they describe the same bytes.

## Encoding — z-base-32 in DNS, base32-lowercase in tickets, hex in configs

Three encodings show up depending on context, and you have to know which you're looking at:

- **z-base-32 (z32)** for DNS discovery records. The DNS label form is `_iroh.<z32-node-id>.<origin-domain> TXT`, where `<z32-node-id>` is "the [z32](https://crates.io/crates/z32) encoding of the 32-byte long `NodeId` (which is a string of 52 characters)" ([Iroh DNS post](https://www.iroh.computer/blog/iroh-dns)). z32 is human-typeable, case-insensitive, and dodges the `0/O`, `1/l/I` collisions of stock base32.
- **Base32-lowercase, postcard-serialized** for *tickets* (the connection-bootstrap blobs). Tickets carry the EndpointID *plus* relay URL plus direct addresses plus optional app data; a ticket starts with an ASCII type prefix (`endpoint`, `blob`, `doc`) and then base32-lowercase of postcard bytes ([tickets concept](https://docs.iroh.computer/concepts/tickets)). Tickets are what dumbpipe and sendme paste around.
- **Plain hex** for relay allowlist configs and a few other Rust-API surfaces. The "EndpointID format in allowlist" discussion ([#3900](https://github.com/n0-computer/iroh/discussions/3900)) is the canonical "I pasted the wrong thing" thread — the `endpointXXX…` ticket-style string is *not* what relay configs want; they want raw hex of the public key.

There is no DID layer. Iroh is **bare-pubkey identity**. The 32 bytes are the identity. Anything resembling DID, naming, or human-readable handles is the application's problem.

## Where private keys live

Iroh's defaults are intentionally minimal: `SecretKey::generate()` to mint, `SecretKey::from_str()` to parse. Persistence is *the application's responsibility*. There is no built-in keyring integration, no OS-keychain wiring, no encrypted-at-rest store. Common patterns observed in n0's own examples:

- Environment variable holding the hex secret (workshop / demo path).
- A `.secret` / `.pub` file pair under `~/.ssh` or an app config dir (iroh-ssh's `--persist` flag does this).
- Whatever the host application already does for secrets (Spacedrive ties device pairing to BIP39 mnemonics; Delta Chat embeds the key in its existing account-state).

If your threat model wants encrypted-at-rest custody, you build it. Compare with Holochain's lair keystore (out-of-process, libsodium-backed, passphrase-encrypted) — iroh has no equivalent. See [`../holochain/identity.md`](../holochain/identity.md) for the contrast.

## Rotation and backup — there is no built-in story

Iroh has no key-rotation primitive. The pubkey *is* the identity; replacing the keypair means becoming a new endpoint as far as every peer is concerned. This is the same "stranded if compromised" failure mode Holochain's removed DPKI was meant to fix.

n0 has published one credible direction here: the [FROST threshold signatures post](https://www.iroh.computer/blog/frost-threshold-signatures) demonstrates splitting an Ed25519 key into shares (e.g. device + server + offline backup) such that signing requires a threshold but no share holds the full key. FROST signatures are bit-compatible with vanilla Ed25519, so pkarr/DNS publishing keeps working unchanged. This is a research demo, not a shipped feature; no `iroh-frost` crate exists in the main release.

Backup, similarly, is "you handle it." Persist the 32 secret bytes wherever your app already persists secrets.

## Discovery — DNS, pkarr, and (opt-in) mainline DHT

Discovery is how a peer translates an EndpointID into "who do I dial." Iroh ships three mechanisms, layered:

- **DNS discovery (default).** Each node publishes a [pkarr](https://crates.io/crates/pkarr)-signed DNS packet to a Pkarr relay; clients resolve `_iroh.<z32-node-id>.dns.iroh.link TXT` to recover the node's home relay URL and direct addresses. The default origin domain `dns.iroh.link` is run by n0 ([Iroh DNS post](https://www.iroh.computer/blog/iroh-dns)). Self-hosting is supported via `EndpointBuilder::discovery_dns(origin)`.
- **mDNS (default for LAN).** Local-network discovery via multicast DNS. No relay involved; same-LAN peers find each other directly.
- **Mainline DHT (opt-in).** Behind the `discovery-pkarr-dht` Cargo feature ([DHT discovery doc](https://docs.iroh.computer/connecting/dht-discovery)). The same pkarr-signed packets get republished to BitTorrent's mainline DHT, giving a fully decentralized lookup path. Off by default — n0 explicitly cites resource cost and that most users don't need it.

All three publish-paths are *signed* — a discovery record is only valid if its signature matches the EndpointID it claims to describe. So a malicious DNS server can withhold records but cannot impersonate an EndpointID.

## Implications for Myrhiza

- **The 32-byte pubkey is a fine kernel-level peer identity.** Maps cleanly to whatever Myrhiza wants for `peer_id` in the per-peer instance identity used by behavior components. No translation layer needed if we stay raw-bytes internally.
- **But it is not enough as a user identity.** Bare pubkey gives us "this device" not "this human." Multi-device, key rotation, recovery — none of these are iroh's job. Myrhiza must decide *where* that lives: in a state-apply component (DPKI-style on-network identity registry, with the warning sign of Holochain's seven-year DPKI failure), in a separate identity service, or punted to the app author. Pick before v1; half-shipped identity is worse than missing identity.
- **Discovery is a kernel capability, not an app one.** Apps should never see DNS, mDNS, or DHT directly — the kernel resolves EndpointID → connection and hands the app a stream. This keeps determinism (state-apply never observes "did discovery succeed in 200ms") and keeps host-import surface narrow.
- **Encoding choice matters for sturdyrefs.** If Myrhiza adds OCapN-style sturdyrefs ([`../spritely-ocapn/`](../spritely-ocapn/)), the netlayer hint should embed the raw 32 bytes plus a relay/address bundle, mirroring iroh's ticket format. Don't invent a new encoding; ride iroh's z32-for-DNS / base32-for-tickets convention so QR codes and copy-paste keep working.
- **No key custody = a host-import opportunity.** Myrhiza can ship encrypted-at-rest secret-key custody as a kernel capability that apps cannot bypass. This is a clean win over iroh's "you figure it out" default.

## Sources

- [Endpoints concept](https://docs.iroh.computer/concepts/endpoints) — `EndpointID` defined as Ed25519 pubkey.
- [Tickets concept](https://docs.iroh.computer/concepts/tickets) — ticket format, base32-lowercase + postcard.
- [Iroh DNS post](https://www.iroh.computer/blog/iroh-dns) — z32 encoding, `dns.iroh.link`, pkarr packets.
- [DHT discovery doc](https://docs.iroh.computer/connecting/dht-discovery) — mainline DHT, opt-in feature flag.
- [pkarr crate](https://crates.io/crates/pkarr) — Public-Key Addressable Resource Records.
- [z32 crate](https://crates.io/crates/z32) — z-base-32 implementation.
- [EndpointID format discussion #3900](https://github.com/n0-computer/iroh/discussions/3900) — hex vs ticket-encoded confusion.
- [FROST threshold signatures post](https://www.iroh.computer/blog/frost-threshold-signatures) — proposed (not shipped) recovery story.
- [iroh 0.94 release](https://www.iroh.computer/blog/iroh-0-94-0-the-endpoint-takeover) — `NodeId` → `EndpointId` rename.
- [iroh-ssh tutorial](https://ovelny.sh/chaos/tutorials/iroh-ssh-and-termux-setup/) — example file-based key persistence with `--persist`.
