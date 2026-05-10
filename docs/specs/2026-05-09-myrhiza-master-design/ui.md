**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — UI

# UI: framework, not app substrate

## 13. UI: framework, not app substrate

PR #636 framed the UI surface as "another app." Master spec adopts
the framing with explicit honesty about its caveats.

### 13.1 UI as app

The default UI app is shipped in-tree (initially `myrhiza-ui-leptos`
for Leptos-based browser/native UI). It exports `ui:*` interfaces
(panel, list, message, form, menu, button, input, ...) that other
apps' interaction components import.

Other UI apps may be authored:

- `myrhiza-ui-tui` — terminal, ratatui rendering.
- `myrhiza-ui-mcp` — agent host, structured-data rendering for an
  LLM.
- `myrhiza-ui-mobile-native` — Compose/SwiftUI shell, future.
- `myrhiza-ui-dioxus` — when Dioxus Blitz matures.

### 13.2 UI app capability surface

A UI app must bind a broad capability surface (DOM, focus, IME,
clipboard, file pickers, navigation, viewport, push, IndexedDB,
service workers, drag-and-drop on web; equivalent on native). The
master spec acknowledges:

- The default UI app is privileged. It is in the TCB for its own
  chrome and DOM.
- The default UI app is **not** in the TCB for arbitrary callers'
  intents — but only because per-call gating (§7.3) protects against
  caller social engineering at the **kernel** boundary, NOT inside
  the UI app's render path.

The "UI is just another app" framing is honest only when the UI
app's privilege is bounded by the runtime AND specific privileged
operations bypass the UI app entirely.

### 13.2.1 Kernel-controlled UI surface

For high-value-op approval prompts (clipboard write, file picker,
top-level navigation, push registration, AEAD seal/open with
sensitive keys, HTTP egress with origin filter), the **kernel renders
the prompt directly**, not via the UI app. This is required because:

- The UI app cannot be trusted to faithfully render a prompt for
  privileged operations. A compromised UI app could fake an approval.
- Per-call gating's defense in depth requires that the user response
  is genuinely from the user, not synthesized by the UI app.

**v1 kernel-controlled surface implementations**:

- **Native**: kernel renders prompts via OS-native modal dialog
  primitives (Cocoa, GTK, WinUI). The UI app cannot draw over OS-
  native modals. Engineering effort: per-platform; budget for v1.
- **Browser**: the **UI app itself runs in a sandboxed iframe whose
  parent is the kernel-controlled origin** (NOT the other way around).
  The kernel renders chrome (toolbar, install prompts, high-value-op
  approvals) in the parent context; the UI app inhabits the child
  iframe with `sandbox="allow-scripts"` (no `allow-same-origin`,
  no `allow-top-navigation`, no `allow-popups`). The UI app cannot
  reach the parent's DOM, cannot postMessage into kernel-controlled
  surfaces unless the parent explicitly opens a postMessage channel,
  cannot manipulate z-index of parent's chrome.

  **Why parent = kernel, not child**: z-index alone is not a security
  boundary. A child iframe is an OS-enforced isolation: scripts in
  the child cannot reach the parent's window object, cannot fake
  approval clicks for parent-rendered controls, cannot adjust their
  own z-index above the parent. This is the standard pattern used
  by browser extensions for protected UI; we adopt it.

  Concrete: kernel ships as an HTTPS-served origin (e.g.
  `https://kernel.localhost`). The UI app loads as
  `<iframe sandbox="allow-scripts" src="https://app-{hash}.kernel.localhost">`.
  High-value-op approval prompts render in the parent context; the
  iframe cannot draw over them.

The kernel-controlled surface is **kernel TCB**, not part of any UI
app. App authors do not customize it; the kernel ships a fixed
prompt format with the visual hash icon (§10.5) for author identity.

**`host.user-prompt(prompt) -> response`** for non-privileged intent
prompts MAY use the UI app's surface. The UI app is in the TCB for
those prompts; the kernel doesn't enforce kernel-rendered chrome
for non-privileged prompts (the cost would be prohibitive for normal
UX flows like "Are you sure you want to send this message?").

### 13.3 Custom-pixel surfaces

Whiteboards, code editors, network-graph visualizers, 3D voice
rooms, custom physics — these need custom-pixel control beyond
what `ui:*` interfaces express. Solution:

- On web: sandboxed iframe with postMessage protocol (kernel-mediated
  capability).
- On native: platform-specific equivalent.
- On TUI / MCP: rendered as "unavailable on this surface."

The escape hatch is web-shaped on purpose. GPU-driven UI substrates
(e.g. Bevy as a surface plugin once its web tooling matures) compose
here, not as replacements for the default UI app.

### 13.4 ui:* WIT contract

The `ui:*` interfaces are an interaction contract for app-to-UI
integration, not a portable UI substrate. They define how an
interaction component declares the views it wants rendered, the
commands it accepts, and the contextual integration points it offers
— not how those views are painted.

Each UI app implements the contract in its own idiom. Reusing one
set of interaction components across UIs is the goal; UI apps that
do not export an interface (e.g. a TUI without `ui:rich-card`) cause
graceful degradation, not breakage.

The exact `ui:*` contract is a child-spec concern.


