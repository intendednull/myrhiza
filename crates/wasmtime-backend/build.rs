//! Build script: emit rerun-if-changed for the WIT package so a WIT
//! edit triggers re-compilation of bindgen output (added in Task 25).

fn main() {
    println!("cargo:rerun-if-changed=../../wit/myrhiza-kernel/wit");
}
