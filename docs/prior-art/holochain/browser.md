# Browser viability

This is the area where Holochain's structural debt is most visible.

## No native browser conductor

The conductor is a Rust process. UIs are HTML+JS that connect to the local conductor via websocket using the [`@holochain/client`](https://github.com/holochain/holochain-client-js) package.

The standard distribution is the Holochain Launcher (Tauri-based desktop app) or, for mobile, a Tauri-Mobile wrapper (Kangaroo / [p2p Shipyard](https://blog.holochain.org/happs-spotlight-relay/)).

A "WASM conductor / light client" has been on the roadmap since at least 2019 ([WASM Conductor and Light Client groundwork](https://blog.holochain.org/the-groundwork-for-the-wasm-conductor-and-light-client/)) and is still listed as in-progress in the [2025 roadmap](https://www.holochain.org/roadmap/).

Holo Inc. operates a separate "hosting bridge" model where browsers connect to Holo-hosted nodes that act on behalf of the user — but this is a hosted service running real conductors, not a browser-resident conductor.

## Why this matters for Myrhiza

Holochain's runtime predates the Component Model. It compiles guest WASM with a Holochain-specific ABI (`hdk` macros + JSON-bincode wire format). That ABI was not designed for browser embedding and was not designed for guest-language pluralism.

Myrhiza's bet on Component Model + jco lets you ship the same components to a native iroh runtime and to a browser jco-compiled JS shim **without re-architecting**. Holochain is still bolting that on after 6+ years; you can have it as a first-class invariant.

## Lesson

**Browser viability is a load-bearing requirement, not a roadmap item.**

Holochain has been promising browser conductors since 2019. Six years later, still not shipped. The reason isn't lack of effort — it's that the original ABI wasn't designed for it. Once an ABI exists with N apps depending on it, retrofitting browser viability becomes a multi-year project that competes with every other improvement.

Myrhiza's choice to require jco transpile from day 0 closes off that failure mode. See [`lessons.md`](lessons.md).

## Sources

- [holochain-client-js](https://github.com/holochain/holochain-client-js)
- [The Groundwork for the WASM Conductor and Light Client](https://blog.holochain.org/the-groundwork-for-the-wasm-conductor-and-light-client/)
- [Holochain Roadmap](https://www.holochain.org/roadmap/)
- [hApps Spotlight: Relay](https://blog.holochain.org/happs-spotlight-relay/)
