//! Build-time validation of the `myrhiza:kernel` WIT package.
//!
//! Runs `wit_parser::Resolve::push_dir` on every build of `myrhiza-types`
//! so a malformed `.wit` file fails CI rather than slipping through.
//! Panics are the legitimate "build-script invariant violation" path
//! documented in the workspace `Cargo.toml`.

fn main() {
    println!("cargo:rerun-if-changed=../../wit/myrhiza-kernel/wit");
    let path = std::path::Path::new("../../wit/myrhiza-kernel/wit");
    if !path.exists() {
        return;
    }
    let mut resolve = wit_parser::Resolve::new();
    if let Err(e) = resolve.push_dir(path) {
        // Build-script invariant violation — see workspace Cargo.toml
        // note on the `panic = "warn"` clippy lint.
        #[allow(clippy::panic)]
        {
            panic!("wit/myrhiza-kernel/wit failed to parse: {e}");
        }
    }
}
