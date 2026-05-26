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
                state_propose: $crate::__opt_lit!($($sp)?),
                interaction: $crate::__opt_lit!($($ix)?),
                behavior: $crate::__opt_lit!($($bh)?),
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
