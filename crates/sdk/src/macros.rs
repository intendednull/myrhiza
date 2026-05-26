//! Macros for app authors.
//!
//! See `myrhiza_sdk::prelude::manifest` for the declarative macro
//! that builds a canonicalized `Manifest` struct from Rust syntax.

/// Build a canonicalized [`Manifest`](crate::manifest::Manifest) from
/// Rust syntax.
///
/// Per spec §2.6 + §3.2. App authors write their manifest as Rust code;
/// schema validation surfaces at compile time via the macro's expansion
/// against the [`Manifest`](crate::manifest::Manifest) struct shape in
/// `crates/manifest/src/schema.rs`. The macro calls
/// [`Manifest::canonicalize`](crate::manifest::Manifest::canonicalize)
/// after construction so callers receive normalized output.
///
/// # Example
///
/// ```
/// use myrhiza_sdk::prelude::*;
///
/// let m = myrhiza_sdk::manifest! {
///     app {
///         name: "counter",
///         version: "0.1.0",
///         description: "shared counter",
///         author_class: third_party,
///     }
///     abi {
///         kernel_major: 1,
///         kernel_minor_min: 0,
///         state_digest_format: bincode13,
///     }
///     capabilities {
///         deterministic_helpers: ["host.hash"],
///     }
///     components {
///         state_apply: "components/state-apply.wasm",
///     }
/// };
/// assert_eq!(m.app.name, "counter");
/// ```
#[macro_export]
macro_rules! manifest {
    (
        app {
            name: $name:literal,
            version: $version:literal,
            description: $description:literal,
            author_class: $class:ident,
        }
        abi {
            kernel_major: $kmaj:literal,
            kernel_minor_min: $kmin:literal,
            state_digest_format: $sdf:ident,
        }
        capabilities {
            $( deterministic_helpers: [ $($helper:literal),* $(,)? ] , )?
            $( host_imports: [ $($hi:literal),* $(,)? ] , )?
        }
        components {
            $( state_apply: $sa:literal , )?
            $( state_propose: $sp:literal , )?
            $( interaction: $ix:literal , )?
            $( behavior: $bh:literal , )?
        }
    ) => {{
        use $crate::manifest::*;
        let mut m = Manifest {
            app: AppSection {
                name: $name.into(),
                version: $version.into(),
                description: $description.into(),
                author_pubkey: String::new(), // filled at signing time
                author_identity_class: $crate::__author_class!($class),
            },
            abi: AbiSection {
                kernel_major: $kmaj,
                kernel_minor_min: $kmin,
                state_digest_format: $crate::__sdf!($sdf),
            },
            capabilities: CapabilitiesSection {
                deterministic_helpers: {
                    #[allow(unused_mut)]
                    let mut h = ::std::collections::BTreeMap::new();
                    $( $( h.insert($helper.into(), true); )* )?
                    h
                },
                host_imports: {
                    #[allow(unused_mut)]
                    let mut h = ::std::collections::BTreeMap::new();
                    $( $( h.insert($hi.into(), true); )* )?
                    h
                },
                ui_surfaces: ::std::collections::BTreeMap::new(),
                high_value_ops: HighValueOps::default(),
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection { interval_events: 1024 },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: $crate::__opt_lit!($($sa)?),
                state_apply_hash: None,
                state_propose: $crate::__opt_lit!($($sp)?),
                state_propose_hash: None,
                interaction: $crate::__opt_lit!($($ix)?),
                interaction_hash: None,
                behavior: $crate::__opt_lit!($($bh)?),
                behavior_hash: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        };
        m.canonicalize();
        m
    }};
}

/// Helper sub-macro: translate `ident` → `AuthorIdentityClass` variant.
///
/// Hidden from the public surface; invoked only by [`manifest!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __author_class {
    (third_party) => {
        $crate::manifest::AuthorIdentityClass::ThirdParty
    };
    (myrhiza_official) => {
        $crate::manifest::AuthorIdentityClass::MyrhizaOfficial
    };
}

/// Helper sub-macro: translate `ident` → `StateDigestFormat` variant.
///
/// Hidden from the public surface; invoked only by [`manifest!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __sdf {
    (bincode13) => {
        $crate::manifest::StateDigestFormat::Bincode13
    };
}

/// Helper sub-macro: optional literal → `Option<String>`.
///
/// Hidden from the public surface; invoked only by [`manifest!`].
#[doc(hidden)]
#[macro_export]
macro_rules! __opt_lit {
    () => {
        None
    };
    ($lit:literal) => {
        Some($lit.into())
    };
}

/// Application initializer macro — replaces the bulk of per-component
/// boilerplate with one invocation.
///
/// Per docs/specs/2026-05-26-b-8-sdk-design.md §2.1 + §3.1 + §3.2.
///
/// Four arms map to the four component profiles. `state_apply`,
/// `state_propose`, and `interaction` emit the bump allocator wiring,
/// the `#[panic_handler]`, the `wit_bindgen::generate!` invocation
/// (pointing at the consumer's `./wit` directory via wit-bindgen's
/// default `CARGO_MANIFEST_DIR` resolution), and the `export!` line.
/// The `behavior` arm is reserved for v1.1 and surfaces a
/// `compile_error!`.
///
/// # Inner attributes — consumer responsibility
///
/// `macro_rules!` macros cannot emit inner attributes (`#![no_std]`,
/// `#![no_main]`, `#![allow(unsafe_op_in_unsafe_fn)]`) that take
/// effect on the surrounding module — Rust's macro expansion rules
/// forbid it. Consumers must place these at the top of each
/// component source file themselves:
///
/// ```ignore
/// #![no_std]
/// #![no_main]
/// #![allow(unsafe_op_in_unsafe_fn)]
///
/// myrhiza_sdk::myrhiza_app!(state_apply, Component);
///
/// // ... app-author code: use alloc::vec::Vec; impl Guest for Component { ... }
/// ```
///
/// The macro emits `extern crate alloc;` itself; `Vec`, `String`,
/// etc. become reachable via `alloc::` paths after that. The
/// `unsafe_op_in_unsafe_fn` allow is for wit-bindgen 0.30's
/// generated unsafe blocks.
///
/// # Example
///
/// ```ignore
/// // In a wasm32-unknown-unknown bin or cdylib crate root:
/// #![no_std]
/// #![no_main]
/// #![allow(unsafe_op_in_unsafe_fn)]
///
/// myrhiza_sdk::myrhiza_app!(state_apply, Component);
///
/// use alloc::vec::Vec;
///
/// impl Guest for Component {
///     fn apply(prior_state: Vec<u8>, event: Vec<u8>) -> (Verdict, Vec<u8>) {
///         // ...
///         # todo!()
///     }
///     fn state_digest(state: Vec<u8>) -> Vec<u8> {
///         // ...
///         # todo!()
///     }
/// }
/// ```
#[macro_export]
macro_rules! myrhiza_app {
    (state_apply, $component:ident) => {
        $crate::__myrhiza_app_impl!("state-apply", $component);
    };
    (state_propose, $component:ident) => {
        $crate::__myrhiza_app_impl!("state-propose", $component);
    };
    (interaction, $component:ident) => {
        $crate::__myrhiza_app_impl!("interaction", $component);
    };
    (behavior, $component:ident) => {
        compile_error!("behavior profile is reserved for v1.1; not yet wired in B-8");
    };
}

/// Helper sub-macro: emit the boilerplate shared across the three
/// non-`behavior` arms of [`myrhiza_app!`] (bump-allocator install,
/// `#[panic_handler]`, `wit_bindgen::generate!` invocation,
/// `struct + export!` pair). Hidden from the public surface; invoked
/// only by [`myrhiza_app!`].
///
/// Splitting the common body out avoids the ~20 LOC × 3-profile copy
/// that would otherwise duplicate verbatim — and lets the boilerplate
/// evolve in one place when wit-bindgen's API shifts.
#[doc(hidden)]
#[macro_export]
macro_rules! __myrhiza_app_impl {
    ($world:literal, $component:ident) => {
        extern crate alloc;

        #[global_allocator]
        static GLOBAL: $crate::__boilerplate::BumpAlloc = $crate::__boilerplate::BumpAlloc;

        #[panic_handler]
        fn panic(_info: &core::panic::PanicInfo) -> ! {
            loop {}
        }

        wit_bindgen::generate!({
            world: $world,
        });

        struct $component;
        export!($component);
    };
}

/// Resolve the consumer crate's `wit/` directory as a `&'static str`.
///
/// Expands to `concat!(env!("CARGO_MANIFEST_DIR"), "/wit")` — the macro
/// lives in the SDK but emits the **consumer's** path. `CARGO_MANIFEST_DIR`
/// is set by Cargo to the caller's manifest dir at macro-expansion time,
/// so the resolved path points at `examples/<app>/wit/` (kept in sync with
/// `crates/sdk/wit/` via `just sync-wit`, asserted by the in-sync test).
///
/// The `local_` prefix encodes the "lives-in-SDK, emits-consumer-path"
/// semantic asymmetry that would otherwise surprise readers expecting
/// `myrhiza_sdk::local_wit_dir!()` to return the SDK's own dir.
///
/// # Note on `wit_bindgen::generate!`
///
/// `wit_bindgen::generate!` is a `proc_macro` that parses its `path:`
/// argument as a `syn::LitStr` (string literal) and does **not**
/// expand `macro_rules!` invocations inside. Wiring
/// `path: $crate::local_wit_dir!()` does not work — the proc-macro
/// sees the unexpanded token tree. The `myrhiza_app!` macro omits
/// `path:` entirely, relying on wit-bindgen's default
/// (`./wit` relative to the consumer's `CARGO_MANIFEST_DIR`), which
/// resolves to the same location. This helper is exported for use
/// in non-proc-macro contexts (e.g., a hand-written `include_str!`
/// of a file in `wit/`).
#[macro_export]
macro_rules! local_wit_dir {
    () => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/wit")
    };
}
