**Date:** 2026-05-09
**Status:** active
**Subject:** Croquet/Multisynq developer-facing programming model — Model/View split, pub/sub, future calls, snapshots

Sibling notes: [`glossary.md`](./glossary.md) · [`architecture.md`](./architecture.md) · [`determinism.md`](./determinism.md) · [`governance.md`](./governance.md) · [`comparisons.md`](./comparisons.md) · [`lessons.md`](./lessons.md) · [`multisynq-platform.md`](./multisynq-platform.md)

This note captures what an application author writes when targeting Croquet/Multisynq. It is the surface that maps most directly onto Myrhiza's component-profile distinction between deterministic state-apply code and per-peer interaction code.

## 1. The two-class model

Every Croquet/Multisynq application is structured as two parallel class hierarchies:

```js
class Game extends Multisynq.Model {  // synchronized, deterministic
    init(options, persisted) { /* ... */ }
}
Game.register("Game");

class Display extends Multisynq.View { // per-client, non-deterministic
    constructor(model) { super(model); /* DOM, canvas, input */ }
    update(time) { /* called once per animation frame */ }
}
```

The Model holds shared simulation state — physics, rules, persistent data. The View handles input, rendering, and any per-user concern. The framework guarantees Model state is identical across every peer in a session; View state is per-peer and free to diverge.

## 2. Publish / subscribe

All cross-class communication is event-based:

```js
this.publish(scope, event, data);          // anywhere
this.subscribe(scope, event, handler);     // anywhere
```

Routing rules (from the Multisynq tutorial *Model-View-Synchronizer*):

- **View → Model** events are reflected through the synchronizer to *all* peers' Models — this is how user input enters the simulation.
- **Model → Model** events run locally in the deterministic VM (still consistent because every peer runs the same code).
- **Model → View** events are delivered locally on each peer.
- **View → View** events are local-only.

`scope` is an arbitrary string used to namespace events; `viewId` and `model.id` are the canonical scopes. The view *can* read directly from the Model object graph, but must never write — all writes flow through published events.

## 3. Model creation: `Model.create()`, never `new`

Models are constructed via the static factory:

```js
const ship = Ship.create({ viewId });
```

Direct `new` is forbidden because the framework allocates and tracks deterministic IDs, registers the instance with the snapshot system, and routes `init(options, persisted)` instead of a constructor. Constructors run again on snapshot resume; `init` runs only once at first creation. Implementing logic in a constructor is explicitly called out as a bug pattern in Multisynq's own examples.

Every Model class must call `MyModel.register("MyModel")` at module load so the class can be looked up by name when a snapshot is rehydrated.

## 4. Object identity and references

Each Model has a deterministic `this.id` assigned by the framework. Code references other Models either by the live JS reference (which the serializer rewrites to an ID on snapshot) or by `wellKnownModel(name)` for global anchors — `multiblaster` reaches the root via `this.wellKnownModel("modelRoot")`. View↔Model coupling uses `viewId`, an auto-assigned per-peer identifier scoped to the session. This indirection is what lets snapshots round-trip through a JSON-shaped wire format without losing the object graph.

## 5. Future calls — virtual time scheduling

The Model has no access to `setTimeout` or `Date.now()`. Instead:

```js
this.future(50).mainLoop();   // call this.mainLoop() in 50ms of simulation time
```

`future` returns a proxy; the next method call on the proxy is queued at `now + ms` of *virtual* time, which advances under the synchronizer's control. Pending future calls are part of the snapshot, so a client joining mid-game restores both the data state and the schedule of pending callbacks. `this.now()` returns current simulation time. Animation loops, physics steps, and timeouts are all expressed this way.

## 6. Session join

```js
Multisynq.Session.join({
    apiKey: "…from multisynq.io/coder…",
    appId: "io.multisynq.multiblaster-tutorial",
    name:  "my-session",        // optional; random if omitted
    password: "…",              // gates session access; also the E2EE key
    model: Game,
    view:  Display,
    tps:   20,                  // ticks per second; default 20
});
```

`Session.join` returns a Promise. The synchronizer routes the joiner either to a fresh session (run `init`) or to an existing one (load latest snapshot, replay any messages since). The session ID is derived from `appId + code-hash + Constants` so any model-code change forks a new session — old peers cannot accidentally talk to new code.

## 7. Serialization & the Prime Directive

> *"Your Multisynq Model must be completely self-contained."* — `multisynq-client` README.

Concretely, the Model:

- Stores only data — no captured closures, no function-valued properties (JS cannot introspect functions for serialization).
- Cannot use `async`/`await` or Promises.
- Must not read global mutable state. Constants used by the simulation go on `Multisynq.Constants` so they hash into the session ID.
- Math.random() and Date are *patched* inside Model code — `Math.random()` becomes a deterministic seeded RNG; clock reads return simulation time.

Custom classes are serialized by registering them; for non-Model classes that appear inside Model state, an author supplies a `static types()` declaration that maps a class name to write/read functions for the snapshot. (The exact API surface lives in the JSDoc reference at `multisynq.github.io/multisynq-client`.)

Snapshots are produced by the synchronizer on a schedule and on demand; they live in encrypted storage so the joiner can resume without replaying from genesis.

## 8. Worked example — `multiblaster`

`github.com/multisynq/multiblaster` is an asteroids homage in ~600 lines. The shape is canonical:

```js
class Game extends Multisynq.Model {
    init(_, persisted) {
        this.highscores = persisted?.highscores ?? {};
        this.ships = new Map();
        this.subscribe(this.sessionId, "view-join", this.viewJoined);
        this.subscribe(this.sessionId, "view-exit", this.viewExited);
        Asteroid.create({});
        this.mainLoop();
    }
    viewJoined(viewId)  { this.ships.set(viewId, Ship.create({ viewId })); }
    viewExited(viewId)  { this.ships.get(viewId).destroy(); this.ships.delete(viewId); }
    mainLoop() {
        for (const ship of this.ships.values()) ship.move();
        /* … */
        this.future(50).mainLoop();         // 20Hz physics tick
    }
}
Game.register("Game");
```

Per-player input is scoped to that player's `viewId`, so each `Ship` subscribes only to its owner's events:

```js
this.subscribe(viewId, "left-thruster",   this.leftThruster);
this.subscribe(viewId, "fire-blaster",    this.fireBlaster);
```

The `Display` View attaches DOM listeners and publishes input events; in `update()` it reads directly from the Model and renders a canvas with smoothing/interpolation between physics ticks.

## 9. What the developer cannot do (in Model code)

| Forbidden | Replace with |
|---|---|
| `setTimeout`, `setInterval` | `this.future(ms).method()` |
| `Date.now()`, `performance.now()` | `this.now()` (simulation time) |
| `Math.random()` (raw) | `Math.random()` is monkey-patched inside Model to a seeded deterministic RNG |
| `async` / `await`, Promises | synchronous code only; use future calls for delays |
| Closures stored on `this` | plain data fields; methods on the class |
| Direct DOM / `fetch` / `localStorage` | move to View; publish results in via events |
| Global mutable state | `Multisynq.Constants` (hashed into session ID) |

These constraints are exactly the price of determinism. The error messages and developer ergonomics around them are one of the more polished aspects of the platform.

## 10. Implications for Myrhiza

- **The Model/View split is the same shape as Myrhiza's `state-apply` (deterministic) vs `interaction` (non-deterministic UI) component profiles.** Treat Croquet's Prime Directive as empirical evidence that the split is implementable, teachable, and survives real apps.
- **The "auto-reflected View→Model event" is Myrhiza's intent → event submission path.** Croquet shows that authors can stay in one mental model — publish — without thinking about wire format. Myrhiza should aim for the same ergonomics on top of `state-propose` / `state-apply`.
- **Future calls are the model for deterministic timers in a WASM state-apply component.** Anything in Myrhiza that wants to fire later (timeouts, animation, retry) needs an analogous virtual-time API; do not let `state-apply` see wall-clock.
- **Serialization-by-construction beats ad-hoc CBOR.** Croquet's `register("Name")` + `static types()` pattern is much smaller than a full schema language and keeps the snapshot tightly coupled to the code that produced it. Myrhiza's component model already gives us a typed interface; the snapshot story can lean on that rather than reinventing it.
- **Constants-into-session-ID is a cheap version-pinning trick.** Myrhiza's analogue is the component hash; ensure any non-component-resident constants (feature flags, network parameters) hash into session/state identity the same way, or apps will desync silently when a constant changes underfoot.

## Sources

- `@multisynq/client` README — github.com/multisynq/multisynq-client (Apache-2.0, 1.1.0, 2025-07-24)
- API Reference index — `docs.multisynq.io/api-reference`
- *Model-View-Synchronizer* tutorial — `docs.multisynq.io/tutorials/model-view-synchronizer`
- `multiblaster/index.html` — github.com/multisynq/multiblaster (Apache-2.0)
- JSDoc reference — `multisynq.github.io/multisynq-client/` (Model, View, Session class pages)
- npm registry: `@multisynq/client@1.1.0` (Apache-2.0); `@croquet/croquet@2.0.4` (Apache-2.0, last published 2025-06-09)
