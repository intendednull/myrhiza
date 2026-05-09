**Date:** 2026-05-08
**Status:** active
**Subject:** Relay — encrypted P2P messenger on Holochain (deployed on Volla Quintus)

# Relay — worked-example deep-dive

## 1. What it is

Relay (rebranded **Volla Messages** for the Volla ecosystem) is a peer-to-peer encrypted messenger built by [Terran Collective](https://www.terran.io/) and the Holochain Foundation. There are no central servers; every participant's device runs a Holochain conductor and "the data is gossiped around within a group — each member of the group is relaying messages on behalf of the other members" ([hApps Spotlight: Relay](https://blog.holochain.org/happs-spotlight-relay/)). It ships preinstalled on the [Volla Quintus](https://happeningscommunity.substack.com/p/introducing-the-volla-quintus-smartphone) phone (Volla OS 14 / Ubuntu Touch, €719, first deliveries October 2024) alongside another hApp, *Recover*, for encrypted backups.

## 2. Architecture overview

The repo `holochain-apps/relay` (also published as `holochain-apps/volla-messages`) ships a single hApp role named **`relay`**, with `clone_limit: 100` ([`workdir/happ.yaml`](https://github.com/holochain-apps/relay/blob/main/workdir/happ.yaml)). Every chat is a clone of that role, each with its own DNA hash (network).

The DNA bundles three integrity zomes paired with three coordinator zomes ([`dnas/relay/workdir/dna.yaml`](https://github.com/holochain-apps/relay/blob/main/dnas/relay/workdir/dna.yaml)):

- `relay_integrity` / `relay` — messages, conversation config, contacts.
- `profiles_integrity` / `profiles` — the standard Lightning Rod Labs profile zome.
- `file_storage_integrity` / `file_storage` — chunked attachments (used for image/file shares).

Entry types in `relay_integrity` are `Config { title, image }`, `Message { content, bucket, images }`, and `Contact { public_key, first_name, last_name, avatar }`. Link types include `AllMessages`, `MessageUpdates`, `ConfigUpdates`, `AllContacts`, `ContactUpdates`, `ContactToContacts` ([`integrity/relay/src/lib.rs`](https://github.com/holochain-apps/relay/blob/main/dnas/relay/zomes/integrity/relay/src/lib.rs)).

UI is **SvelteKit + Tailwind + Skeleton**, packaged with **Tauri Mobile** and the **[p2p Shipyard](https://darksoil.studio/p2p-shipyard/)** Tauri plugin (darksoil studio) which embeds a Holochain conductor inside the app on Android / Windows / macOS / Linux.

## 3. Identity model

A user's identity *is* a Holochain `AgentPubKey` — there are no phone numbers, emails, or accounts. The `profiles` zome attaches a display name and avatar to that key inside each clone. The blog post puts it plainly: users connect to each other "via public key, providing a decentralized and secure system of identifiers."

There is **no multi-device account** in the publicly available code. One Volla Quintus = one keypair = one identity per conversation. A `Contact` entry stored in the *provisioned* (root) cell lets a user attach friendly names to other agents' pubkeys locally; the comment in `RelayClient.ts` notes "Contacts are all stored in the original provisioned relay Cell."

## 4. Group / conversation model

Each conversation is its own **DNA clone** with a fresh random `network_seed` (a UUIDv4) — i.e., a separate DHT network. From [`ui/src/store/RelayClient.ts`](https://github.com/holochain-apps/relay/blob/main/ui/src/store/RelayClient.ts):

```ts
const modifiers = {
  network_seed: uuidv4(),
  properties: { created, privacy, progenitor: encodeHashToBase64(this.client.myPubKey) },
};
const cellInfo = await this.client.createCloneCell({ role_name: ROLE_NAME, ... });
```

The **progenitor pattern** controls membership. The creating agent's pubkey is baked into the DNA's `properties` and acts as the conversation's admin. For `Privacy::Private` conversations, joining requires a **membrane proof** — `MembraneProofData { conversation_id, for_agent, as_role }` signed by the progenitor. The integrity zome's `genesis_self_check` rejects any agent that can't present a valid signed envelope. The progenitor signs proofs via the `generate_membrane_proof` extern in the coordinator zome.

Invitations are bundled (`Invitation { networkSeed, created, privacy, progenitor, title, proof }`) and shared out-of-band — typically as a QR code (the repo ships an `example-qr-code-user.png`). The recipient calls `joinConversation(invitation)`, which spawns the matching clone with the supplied membrane proof.

`Privacy::Public` conversations skip the proof check, allowing open join via the network seed alone.

## 5. Message lifecycle

When Alice sends "hello" in a conversation cell:

1. UI calls `create_message(SendMessageInput { message, agents })` on the cloned cell.
2. The coordinator zome (`coordinator/relay/src/message.rs`) calls `create_entry(EntryTypes::Message(...))` — appending a `Create` action to Alice's source chain. This produces standard DHT ops: `StoreRecord`, `StoreEntry`, `RegisterAgentActivity` published to neighborhoods of the relevant hashes.
3. It then calls `create_link(messages_path(bucket).path_entry_hash(), message_hash, LinkTypes::AllMessages, ())` — anchoring the message under a time-bucketed `Path` (`"msg.{bucket}"`). This adds a `RegisterCreateLink` op so peers can enumerate recent messages without scanning chains.
4. Finally `send_remote_signal(MessageRecord { ... }, agents)` fires an ephemeral signal directly to the other members listed by the UI — this is what gives the "instant delivery" feel without waiting for DHT gossip to converge.
5. On Bob's device, `recv_remote_signal` receives the payload and calls `emit_signal(Signal::Message { action, message, from })`. The Svelte UI subscribes to these signals and renders the message immediately. Background DHT gossip and validation (the `validate` callback in the integrity zome) confirms persistence.

Edits use `update_entry` plus a `MessageUpdates` link from the original action; deletes use `delete_entry`, removing the `AllMessages` link first. Both broadcast a corresponding remote signal.

## 6. Encryption

Two layers, neither of which is implemented in the hApp Wasm itself:

- **On the wire / at rest in the DHT:** Holochain's transport (kitsune2/QUIC) encrypts node-to-node traffic, and a private DNA's data is only visible to agents who hold the membrane proof — every conversation gets its own DHT, so non-members literally cannot resolve the entries. This is what the [Volla announcement](https://blog.holochain.org/volla-partnership-announcement/) and [heise.de's coverage](https://www.heise.de/en/news/Messenger-alternative-Volla-Messages-with-big-promises-10352930.html) refer to as "256-bit encryption" and "encrypted networks per conversation."
- **At rest on device:** standard Holochain LMDB encryption for the source chain.

There is **no application-layer message encryption, no Signal-style ratchet, and no forward secrecy** in the public source — `grep` for `encrypt`, `nonce`, `ratchet`, `secretbox` in the relay zomes returns nothing. Heise specifically noted that "detailed information on the messenger's encryption was not found in the GitHub repository … the company has confirmed that it employs end-to-end encryption but has yet to release a comprehensive white paper." Per-conversation DNA isolation is the security model. Compromising a member's device exposes the full history.

## 7. Capability use

Capabilities are minimal. The coordinator zome's `init` grants exactly one **unrestricted** cap:

```rust
let mut fns = BTreeSet::new();
fns.insert((zome_info()?.name, "recv_remote_signal".into()));
create_cap_grant(CapGrantEntry {
    tag: "".into(),
    access: CapAccess::Unrestricted,
    functions: GrantedFunctions::Listed(fns),
})?;
```

This lets any peer in the same DNA call `recv_remote_signal` to deliver a message — the membrane proof check at the DNA boundary is what gates "any peer." All other zome functions are called locally by the SvelteKit UI through Tauri's app-websocket using its app-authentication token; no transferable or assigned caps are issued.

## 8. Distribution

Built and signed with Tauri Mobile + p2p Shipyard. The README lists CI signing for Android (`.jks`), Windows (Azure EV cert), and macOS (Apple Developer ID). Mobile builds run in **"zero-arc"** configuration — phones don't store any DHT slice, to "save on battery life and help the application meet app store requirements" ([Holochain blog](https://blog.holochain.org/happs-spotlight-relay/)). Volla Quintus is the exception: because Volla controls the OS, those devices "actually do hold full nodes," forming a self-hosting cloud of phones.

Preinstall mechanics on Quintus are simple — the `.apk` is bundled into Volla OS 14 alongside Recover. Desktop builds are distributed as standard installers via GitHub Releases (`vX.Y.Z` git tags trigger CI).

## 9. Production status

Beta on the Quintus from August 2024; first phone deliveries October 2024 ([Volla announcement](https://blog.holochain.org/volla-partnership-announcement/), [Volla Quintus review](https://happeningscommunity.substack.com/p/introducing-the-volla-quintus-smartphone)). Working features: 1:1 and group chat, file/image sharing, profile + avatar, edit/delete, QR-code invitations. Audio/video calls are listed as "planned" by [heise.de](https://www.heise.de/en/news/Messenger-alternative-Volla-Messages-with-big-promises-10352930.html). Known gaps: no multi-device, no message-history sync for late-joining members beyond what the DHT happens to hold, no published cryptographic spec, and the bootstrap/peer-discovery story still depends on Holochain's bootstrap servers in practice. The repo's [issue tracker](https://github.com/holochain-apps/relay/issues) shows ongoing work on connection stability and clone-cell management.

## 10. What Myrhiza can learn

- **Clone-per-context is the right Holochain idiom for groups.** Relay's "every conversation is its own DNA with its own seed and progenitor" is the cleanest membership boundary the platform offers — far simpler than per-entry ACLs. Myrhiza should consider clone-per-circle / clone-per-project for its own group constructs, with `clone_limit` set high.
- **Membrane-proof-as-invite is concise but brittle.** A signed `MembraneProofData { conversation_id, for_agent, as_role }` envelope works, but the signature is from a single `progenitor`. If the progenitor goes offline or rotates keys, no one new can join. Myrhiza needs **rotating admins / multi-signer membrane proofs** — likely a list of admins stored in a `Config` entry whose updates are themselves validated against the previous admin set.
- **Remote signals + DHT links is the right hybrid for "live" UX.** The `send_remote_signal` for instant delivery + `AllMessages` link under a time-bucketed `Path` for catch-up is a pattern Myrhiza can copy directly. The bucket integer (`messages_path(bucket: u32)`) is a nice way to keep link-base fan-out bounded.
- **Don't conflate transport encryption with application E2EE.** Relay leans entirely on per-DNA isolation and Holochain transport. That's defensible against passive network observers but exposes full history on any compromised member device, and the public docs admit the cryptographic story isn't written down. Myrhiza should pick one — either ship a real ratchet on top, or be loud and explicit that the threat model excludes compromised peers.

## Sources

- [hApps Spotlight: Relay — Holochain Blog](https://blog.holochain.org/happs-spotlight-relay/)
- [Volla Partnership Announcement — Holochain Blog](https://blog.holochain.org/volla-partnership-announcement/)
- [GitHub: holochain-apps/relay (a.k.a. volla-messages)](https://github.com/holochain-apps/relay)
- [`workdir/happ.yaml`](https://github.com/holochain-apps/relay/blob/main/workdir/happ.yaml), [`dnas/relay/workdir/dna.yaml`](https://github.com/holochain-apps/relay/blob/main/dnas/relay/workdir/dna.yaml)
- [`dnas/relay/zomes/integrity/relay/src/lib.rs`](https://github.com/holochain-apps/relay/blob/main/dnas/relay/zomes/integrity/relay/src/lib.rs)
- [`coordinator/relay/src/message.rs`](https://github.com/holochain-apps/relay/blob/main/dnas/relay/zomes/coordinator/relay/src/message.rs)
- [`ui/src/store/RelayClient.ts`](https://github.com/holochain-apps/relay/blob/main/ui/src/store/RelayClient.ts)
- [Introducing the Volla Quintus Smartphone — Happenings Community](https://happeningscommunity.substack.com/p/introducing-the-volla-quintus-smartphone)
- [Messenger alternative: Volla Messages with big promises — heise online](https://www.heise.de/en/news/Messenger-alternative-Volla-Messages-with-big-promises-10352930.html)
- [Mobile Holochain Applications Shipped! — Holochain Blog](https://blog.holochain.org/mobile-holochain-applications-shipped/)
- [Terran Collective](https://www.terran.io/)
- [p2p Shipyard — darksoil studio](https://darksoil.studio/p2p-shipyard/)
