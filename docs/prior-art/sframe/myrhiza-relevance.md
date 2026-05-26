**Date:** 2026-05-22
**Status:** active
**Subject:** When and how SFrame becomes load-bearing for Myrhiza — kernel/capability surface, runner-up paradigms, decision points.

# SFrame's relevance to Myrhiza

SFrame is **speculative until Myrhiza ships A/V**, and **load-bearing the moment a Myrhiza capability needs to carry media frames.** This file says what "load-bearing" means concretely.

## 1. The trigger condition

The capability surface flips from "MLS is sufficient" to "we need SFrame (or equivalent)" the first time a Myrhiza app wants to send audio or video frames to a group. Specifically:

- **Voice chat in a channel.** A capability whose stream carries Opus frames.
- **Video calls / screenshare.** A capability whose stream carries VP8/VP9/AV1 frames.
- **Live A/V broadcast within a channel.** Same as above but one-to-many.
- **Real-time interactive media** (gaming voice, collaborative music, etc.).

For each of these, raw MLS ciphertext won't do — MLS is designed for occasional commits + relatively small message volumes, not 50+ frames per second per sender per stream. The MLS commit machinery is not the right cadence for media; the MLS *exporter* is.

This is exactly the gap that motivates the citation at [`prior-art/mls/open-problems.md:45-47`](../mls/open-problems.md).

## 2. The decision: RFC 9605 verbatim, DAVE-style custom, or something else

Three candidate paradigms, with runner-up framing.

### Candidate A: RFC 9605 verbatim

Take MLS, derive `base_key` per epoch via the SFrame label, use one of the five RFC ciphersuites (likely `AES_128_GCM_SHA256_128`), wrap codec frames per RFC 9605 §4.

**Pros:**
- Standards-aligned. Future interop is possible.
- Security analysis already done by the WG.
- Webex is the existence proof at scale.

**Cons:**
- Reference-grade implementation maturity is weak (see [`open-problems.md` §9](open-problems.md)).
- Codec-awareness gap: SFUs/relays handling RFC 9605 frames cannot read codec metadata without spec extension.
- No interop benefit if Myrhiza is the only ecosystem speaking it (and we'd be alone or near-alone outside Webex).

### Candidate B: DAVE-style custom transform

Take MLS, derive a base secret per (epoch, sender) with a Myrhiza-specific exporter label, use AES-128-GCM with truncated tags, define a Myrhiza wire format with codec-aware unencrypted ranges.

**Pros:**
- Codec-awareness ships with the transform; SFUs/relays can do their job.
- DAVE proves the pattern works at scale (Discord).
- We control the wire format and can evolve it without IETF cycles.

**Cons:**
- Owns the security analysis ourselves.
- No interop with Webex or future SFrame ecosystems.
- "Yet another snowflake protocol" tax: future contributors need to learn Myrhiza-specifics rather than industry-standard SFrame.

### Candidate C: pure-peer media without MLS-derived keys

Skip the MLS-derived approach. Use direct Noise/MLS pairwise channels between peers in a mesh topology; each pairwise leg encrypts its own media. No group key for media at all.

**Pros:**
- Conceptually clean — every media stream is end-to-end between two peers.
- No SFU possible (which is also a security positive: no third party in the data plane).

**Cons:**
- Mesh topology bandwidth: N×(N-1) streams. Doesn't scale beyond ~6 participants.
- No "broadcaster to many viewers" pattern.
- Diverges sharply from MLS-channels-for-everything story.

### Runner-up framing

If Myrhiza's first A/V use case is small voice rooms (≤8 participants), Candidate C is genuinely competitive — mesh works. If we ever want larger (≥20-person voice meetings, large video calls, broadcast streams), only A or B remain.

Between A and B, the decision hinges on:
- **Codec-awareness need.** If our relays/SFUs need codec metadata for forwarding (almost certainly yes for any non-trivial deployment), B is forced.
- **Interop ambition.** If we want a future where Myrhiza apps can join Webex-style RFC-9605 meetings, A is forced. If interop is not a goal, B's flexibility wins.
- **Implementation cost.** A means wrapping `libsframe` (FFI risk, maturity question); B means writing ~600 lines of crypto + frame-handling code ourselves.

**Default assumption (rebuttable):** if and when Myrhiza adds A/V, Candidate B (DAVE-style) is most likely the right answer for the same reasons Discord chose it — codec-awareness is non-negotiable at scale, interop with Webex is not a near-term goal, and we already own the MLS layer. **This is a hypothesis to revisit when the actual A/V spec lands**, not a commitment.

## 3. Kernel / capability surface implications

Whichever path we pick, the kernel must:

1. **Expose an MLS-exporter capability** to the media stack. Some Myrhiza component (the kernel, or a designated capability provider) holds the MLS group state and produces `base_key` on demand keyed by `(group, epoch, label)`. Apps don't get raw MLS material — they get `base_key` for one specific purpose.

2. **Brokers per-sender KIDs**. The kernel knows which LeafIndex belongs to which peer; KIDs map onto LeafIndex. Apps don't choose KIDs.

3. **Triggers commits on leave / kick**. The kernel must be able to force an MLS commit when a member is removed, so the leaver-window is bounded. Applications cannot block this.

4. **Counter management**. The kernel tracks per-stream frame counters to prevent reuse. Applications get a "next-frame" RPC, not a raw counter.

5. **Ciphersuite policy**. The kernel picks the SFrame (or SFrame-equivalent) ciphersuite based on group MLS ciphersuite, not the application.

This is consistent with the Myrhiza determinism + capabilities principles: apps don't touch keys, the kernel does, and the API surface is capability-shaped.

## 4. Determinism considerations

SFrame itself is deterministic given (key material, counter, plaintext, AAD). The MLS exporter is deterministic given group state. So in principle, a `state-apply` component could include SFrame-encrypted media in its output deterministically — but **media frames are not state-apply territory.** They are streaming data on `interaction` or `behavior` capabilities, where determinism is loose.

The right modeling is:
- **MLS group state lives in `state-apply` territory.** Joins, leaves, commits are events that the state-apply component materializes.
- **MLS exporter is a deterministic *query* over state-apply output.** Anyone holding the same group state can derive the same `base_key`.
- **SFrame encryption is downstream of state-apply** and happens in the `interaction` / `behavior` profile where the actual media frames originate.

This separation matches the Myrhiza profile structure cleanly. SFrame does not introduce determinism risk because it lives in the streaming-media layer, not in the consensus layer.

## 5. WASM Component Model integration

Like MLS implementations (`prior-art/mls/open-problems.md` §11), no SFrame implementation ships as a Component Model artifact today. Wrapping options:

- **Host-side capability.** The kernel runs `libsframe` (or a Rust port) natively and exposes encrypt/decrypt RPCs to apps via a `host-sframe` import. Apps get ciphertext frames and submit ciphertext frames.
- **Sandboxed crypto.** Compile a pure-Rust SFrame implementation to WASM, expose it as an in-app helper component. Loses the "kernel owns keys" property; doesn't match the capability story.

Host-side is the obvious answer, matching how the kernel handles all long-lived key material.

## 6. Failure mode honesty

If Myrhiza adopts SFrame (or DAVE-style) and the security analysis later finds a flaw in the chosen frame transform or the MLS exporter integration, we cannot just swap a library. The wire format is on the wire; clients in old versions exist; downgrade attacks are a thing.

This is the same risk every E2EE messenger inherits, and is managed by:
- Versioning the frame format (DAVE does this; their v0 / v1 marker is visible in the exporter label).
- Pinning the ciphersuite in MLS group state (so a kicked client cannot "negotiate down").
- Documenting an emergency rotation procedure (force-rekey, force-version-bump).

We should design this in from the start, not bolt it on.

## 7. When to commit

Don't commit to SFrame-vs-DAVE-vs-mesh until:

1. The first concrete A/V capability has a spec under `docs/specs/`.
2. The capability spec includes a threat model (who's the adversary, what's the leaver-window tolerance, what's the metadata-leakage tolerance).
3. The capability spec includes a topology (mesh, SFU, hybrid).
4. We've verified `libsframe` (or chosen alternative) maturity by building a prototype that runs.

Until then, this folder is preparation. It exists so the spec author at that point doesn't start from zero.

## 8. Sources

- [RFC 9605 §5.2 — MLS integration](https://www.rfc-editor.org/rfc/rfc9605.html#section-5.2)
- [Discord DAVE whitepaper](https://daveprotocol.com)
- [`prior-art/mls/open-problems.md` §10 — Voice/video and large media](../mls/open-problems.md)
- [`prior-art/mls/open-problems.md` §11 — WASM Component Model integration](../mls/open-problems.md)
- [`docs/specs/`](../../specs/) — Myrhiza spec corpus (the future home of A/V capability specs)
