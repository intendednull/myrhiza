**Date:** 2026-05-22
**Status:** active
**Subject:** OTP behaviours — gen_server, gen_statem, supervisor, application — the design-principles canon

# OTP behaviours

A **behaviour** in OTP is what other languages call an interface or trait: a contract of callback functions a module implements, plus a generic implementation that drives them. The OTP standard library ships a small set of these that have remained stable across decades. Knowing the four below is most of what makes someone "OTP-fluent."

## `gen_server`

Generic server. Callback module exports:

```erlang
-callback init(Args)        -> {ok, State} | {stop, Reason}.
-callback handle_call(Req, From, State) -> {reply, Reply, NewState} | ...
-callback handle_cast(Msg, State)       -> {noreply, NewState} | ...
-callback handle_info(Info, State)      -> {noreply, NewState} | ...
-callback terminate(Reason, State)      -> term().
-callback code_change(OldVsn, State, Extra) -> {ok, NewState}.
```

`handle_call` is request/reply with the calling process blocked until reply; `handle_cast` is fire-and-forget; `handle_info` is for non-OTP messages (timeouts, links, monitors, raw `!`-sends). `code_change/3` is the **hot-upgrade hook**; see [`hot-code-loading.md`](hot-code-loading.md).

`gen_server` is the workhorse. The vast majority of OTP processes are gen_servers. Production apps measure their codebase in gen_servers-per-thousand-LOC.

## `gen_statem`

Generic state machine. **Replaced `gen_fsm`** in OTP 19 (2016); `gen_fsm` deprecated in OTP 20 (2017) and still ships as legacy. Both terms still appear in older codebases.

Two callback modes:

- `state_functions` — one callback function per state. State name is the function name. Closer to `gen_fsm`'s shape; easier to read for small machines.
- `handle_event_function` — single callback handles all (State, Event) pairs. More flexible; needed for state-data-driven dispatch.

Key features `gen_statem` added vs `gen_fsm`:

- **State enter calls** — callback can run on state entry. Eliminates a common bug class where "do X on transition to state S" was reimplemented at every call site.
- **State-data separation** — state name and state data are independent; state name is a symbolic atom or a richer term.
- **Event postponing** — `{next_state, NextState, NewData, [postpone]}` defers an event until after a state change. Replicates `gen_fsm`'s pattern of "I don't know how to handle this here; try again after the next transition" without re-implementing it.
- **Generic timers** with named identities, cancellable independently. Eliminated the bespoke timer-management each old `gen_fsm` had to write.

**`code_change/4`** for `gen_statem` (note the arity, not `/3`): `code_change(OldVsn, OldState, OldData, Extra) -> {ok, NewState, NewData} | Reason`. Subtle but important — when upgrading a `gen_statem`, both the state name *and* the state data may need to migrate, and the callback takes both.

**Implications for Myrhiza:** `gen_statem`'s `(state_name, state_data)` split + state-enter calls + postpone semantics is a coherent FSM contract worth studying when designing the `state-apply` component's contract. But note the determinism subtlety: BEAM `gen_statem` is single-threaded and not deterministically replayable; Myrhiza's `state-apply` is purer (function of `(prior state, event)`). The behaviour-shape borrows; the timing semantics don't. See [`lessons.md`](lessons.md).

## `supervisor`

Already covered in [`architecture.md`](architecture.md). Restart strategies, child specs, restart intensity. The thing to add here: supervisors are themselves OTP behaviours implementing the `supervisor` callback contract, so a supervisor *module* exports `init/1` returning the children list + strategy. Static children are common; dynamic children use `supervisor:start_child/2` or the newer `:dynamic` (Elixir `DynamicSupervisor`).

**Restart-intensity defaults:** `{MaxR=3, MaxT=5}` since OTP 19 — three restarts in five seconds before the supervisor itself dies. Most production apps tune these per-tier. Aggressive auto-restart at the leaf is fine; an aggressive auto-restart of an *application* tier is a recipe for cascading failure under bad input.

## `application`

The unit of release packaging. An application is:

- A `.app` resource file declaring name, version, modules, dependencies, environment vars, and the root supervisor module.
- A root supervisor that the `application` behaviour starts on `application:start/1`.
- Optionally, a callback module implementing `start/2` and `stop/1`.

An OTP **release** is a curated set of applications + a boot script + their compiled modules + the ERTS (Erlang Runtime System) binary. `release_handler` (covered in [`hot-code-loading.md`](hot-code-loading.md)) operates over releases.

**Implications for Myrhiza:** the `application` boundary in OTP is the closest existing analog to Myrhiza's **app bundle** — a self-contained, versioned, dependency-declaring unit of code. The OTP `.app` file's `applications` dependency list maps cleanly onto a hypothetical Myrhiza `app.toml`'s component dependency list. The boot-script + start-order semantics are also worth studying as a model for "in what order should a Myrhiza app's components come up?"

## Lesser-used behaviours (mention only)

- `gen_event` — event manager / event handler pattern. Largely fallen out of favour in modern code; most projects roll their own pub/sub or use Elixir's `Registry` / `Phoenix.PubSub`.
- `proc_lib` + `sys` — low-level process tooling for "I want OTP-compatible debug/upgrade hooks without the full behaviour shell." Used by library authors to implement custom behaviours.

## Behaviour ergonomics that didn't survive

- `gen_fsm` — replaced by `gen_statem`. Still in tree as legacy.
- `gen_event` — alive but unfashionable.
- `gen_tcp_acceptor` / `gen_udp_acceptor` (never officially OTP) — community attempts to standardise socket-accept loops; never converged.

## What the canon tells us about API longevity

The four behaviours above (`gen_server`, `gen_statem`, `supervisor`, `application`) have **callback signatures that have remained stable for 15+ years**. `code_change/3`'s shape is identical between OTP 12 (2008) and OTP 29 (2026). This is a real testament to having gotten the abstraction right early. The price was steep: it took until OTP 19 (2016) to replace `gen_fsm`'s known-bad ergonomics, because backward-compat across decades was treated as load-bearing.

**Implications for Myrhiza:** when designing the kernel ABI (the `host`-imports any component can call), assume the ABI's first stable cut will outlive multiple Myrhiza generations. OTP's API longevity is a model worth borrowing the *attitude* from, even if the surface area differs. See [`lessons.md`](lessons.md) "Borrow."

## Sources

- gen_server: <https://www.erlang.org/doc/apps/stdlib/gen_server.html>
- gen_statem: <https://www.erlang.org/doc/apps/stdlib/gen_statem.html>
- gen_statem rewrite from gen_fsm guide: <https://www.erlang.org/doc/apps/stdlib/gen_fsm.html>
- supervisor: <https://www.erlang.org/doc/apps/stdlib/supervisor.html>
- application: <https://www.erlang.org/doc/apps/kernel/application.html>
- *Designing for Scalability with Erlang/OTP* (Cesarini & Vinoski, O'Reilly, 2016)
