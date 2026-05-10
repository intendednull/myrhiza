//! Byte-level float-ban lint per determinism.md §5.2.
//!
//! Components that import or use float instructions in any function
//! body fail this lint and are rejected at instantiation. Includes
//! SIMD-float ops (cross-platform divergence vectors per §5.2).
//!
//! ## Approach
//!
//! Whitelist, not blacklist. The set of safe operators is bounded and
//! well-understood (integer arith, control flow, memory, CM glue);
//! the set of unsafe operators grows with every wasmparser release as
//! new SIMD / relaxed-SIMD / GC / threads ops land. A blacklist drifts
//! and silently lets new ops through; a whitelist holds the line.
//!
//! Any operator NOT in the whitelist is rejected. This is intentional:
//! v1 components are deterministic state-apply functions. They have no
//! reason to use float, SIMD, atomics, or GC. If a future proposal
//! adds an operator we want to allow, it must be reviewed and added
//! explicitly to the whitelist.
//!
//! ## Defense in depth
//!
//! `WasmtimeBackend::new` already calls `Config::wasm_simd(false)`,
//! `wasm_threads(false)`, etc., so the engine itself rejects SIMD
//! components. This lint runs at install time, before the engine sees
//! the bytes, and defends independently — Wasmtime defaults move
//! across LTS bumps, and a forgotten config flag should not become a
//! convergence-divergence vector.

use wasmparser::{Operator, Parser, Payload};

/// Scan a core wasm module's function bodies for any banned instruction.
///
/// Returns `Err` naming the first banned instruction encountered.
///
/// # Errors
///
/// Returns `Err` if the bytes are not a valid wasm core module, or if any
/// instruction in any function body is not in the deterministic whitelist
/// (i.e. is a float, SIMD, atomic, GC, or otherwise non-determinism-safe op).
pub fn scan_core_module_for_floats(bytes: &[u8]) -> Result<(), String> {
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|e| format!("wasm parse error: {e}"))?;
        if let Payload::CodeSectionEntry(body) = payload {
            let mut reader = body
                .get_operators_reader()
                .map_err(|e| format!("operators reader error: {e}"))?;
            while !reader.eof() {
                let op = reader.read().map_err(|e| format!("read op error: {e}"))?;
                if !is_allowed_op(&op) {
                    return Err(format!("banned op: {}", op_name(&op)));
                }
            }
        }
    }
    Ok(())
}

/// Scan an entire component's embedded core modules and nested
/// components recursively for banned instructions in any function body.
///
/// `Parser::new(0).parse_all(bytes)` only yields `Payload::ModuleSection`
/// / `Payload::ComponentSection` for *direct* children of the component
/// being parsed. To cover nested components (a component embedded in
/// another component), this function recurses into each
/// `ComponentSection` byte range with a fresh parser.
///
/// # Errors
///
/// Returns `Err` if the bytes are not a valid wasm component, if a
/// `ModuleSection` or `ComponentSection` byte range is out of bounds,
/// or if any embedded core module contains a banned op.
pub fn scan_component_for_floats(component_bytes: &[u8]) -> Result<(), String> {
    for payload in Parser::new(0).parse_all(component_bytes) {
        let payload = payload.map_err(|e| format!("component parse error: {e}"))?;
        match payload {
            Payload::ModuleSection {
                unchecked_range, ..
            } => {
                let module_bytes =
                    component_bytes
                        .get(unchecked_range.clone())
                        .ok_or_else(|| {
                            format!(
                                "component module section range {unchecked_range:?} out of bounds"
                            )
                        })?;
                scan_core_module_for_floats(module_bytes)?;
            }
            Payload::ComponentSection {
                unchecked_range, ..
            } => {
                let inner_bytes =
                    component_bytes.get(unchecked_range.clone()).ok_or_else(|| {
                        format!(
                            "component nested-component section range {unchecked_range:?} out of bounds"
                        )
                    })?;
                scan_component_for_floats(inner_bytes)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Whitelist of deterministic core-wasm operators allowed in v1
/// state-apply / state-propose function bodies.
///
/// Everything not listed here is rejected. The categories below mirror
/// wasmparser's proposal grouping:
///
/// - MVP integer arith / cmp / conv (i32, i64): allowed
/// - MVP control flow (block / loop / if / else / br / call / ...): allowed
/// - MVP locals / globals: allowed
/// - MVP integer load / store: allowed
/// - MVP memory.size / memory.grow: allowed
/// - sign-extension (`i32.extend8_s`, ...): allowed
/// - non-trapping float-to-int (`I32TruncSatF*`): DENIED — touches float
/// - bulk-memory (memory.init/copy/fill, table.init/copy, *.drop): allowed
/// - reference-types (ref.null, ref.func, table.get/set/grow/size/fill, typed-select): allowed
/// - tail-call (`return_call`, `return_call_indirect`): allowed
/// - threads (atomics): DENIED — `wasm_threads(false)` in engine, but defend in depth
/// - SIMD (v128.*, *x*.*): DENIED — divergence vector
/// - relaxed-SIMD (`Relaxed*`): DENIED — explicitly non-deterministic
/// - GC (struct.*, array.*, ref.test/cast, ...): DENIED — not in v1 ABI
/// - exceptions (`try_table`, throw, ...): DENIED — non-determinism vector
/// - shared-everything threads: DENIED
/// - memory-control (memory.discard): DENIED — visibility hint, not deterministic
/// - wide-arith (`I64MulWideS/U`): allowed (integer-only, deterministic)
//
// One large `matches!` arm per allowed op is intentional — splitting
// into per-category helpers would obscure the audit surface, which is
// the whole point of using a whitelist over a blacklist.
#[allow(clippy::too_many_lines)]
fn is_allowed_op(op: &Operator<'_>) -> bool {
    use Operator::{
        // MVP control flow
        Block,
        Br,
        BrIf,
        BrTable,
        Call,
        CallIndirect,
        // bulk-memory
        DataDrop,
        // MVP integer compare / arith / conv (i32)
        Drop,
        ElemDrop,
        Else,
        End,
        // MVP global access
        GlobalGet,
        GlobalSet,
        I32Add,
        I32And,
        I32Clz,
        I32Const,
        I32Ctz,
        I32DivS,
        I32DivU,
        I32Eq,
        I32Eqz,
        I32Extend8S,
        // sign-extension
        I32Extend16S,
        I32GeS,
        I32GeU,
        I32GtS,
        I32GtU,
        I32LeS,
        I32LeU,
        I32Load,
        I32Load8S,
        I32Load8U,
        I32Load16S,
        I32Load16U,
        I32LtS,
        I32LtU,
        I32Mul,
        I32Ne,
        I32Or,
        I32Popcnt,
        I32RemS,
        I32RemU,
        I32Rotl,
        I32Rotr,
        I32Shl,
        I32ShrS,
        I32ShrU,
        I32Store,
        I32Store8,
        I32Store16,
        I32Sub,
        I32WrapI64,
        I32Xor,
        I64Add,
        I64And,
        I64Clz,
        I64Const,
        I64Ctz,
        I64DivS,
        I64DivU,
        I64Eq,
        I64Eqz,
        I64Extend8S,
        I64Extend16S,
        I64Extend32S,
        I64ExtendI32S,
        I64ExtendI32U,
        I64GeS,
        I64GeU,
        I64GtS,
        I64GtU,
        I64LeS,
        I64LeU,
        I64Load,
        I64Load8S,
        I64Load8U,
        I64Load16S,
        I64Load16U,
        I64Load32S,
        I64Load32U,
        I64LtS,
        I64LtU,
        I64Mul,
        I64Ne,
        I64Or,
        I64Popcnt,
        I64RemS,
        I64RemU,
        I64Rotl,
        I64Rotr,
        I64Shl,
        I64ShrS,
        I64ShrU,
        I64Store,
        I64Store8,
        I64Store16,
        I64Store32,
        I64Sub,
        I64Xor,
        If,
        // MVP locals
        LocalGet,
        LocalSet,
        LocalTee,
        Loop,
        MemoryCopy,
        MemoryFill,
        MemoryGrow,
        // bulk-memory: memory ops
        MemoryInit,
        // MVP memory ops
        MemorySize,
        Nop,
        // reference-types
        RefFunc,
        RefIsNull,
        RefNull,
        Return,
        // tail-call
        ReturnCall,
        ReturnCallIndirect,
        Select,
        TableCopy,
        TableFill,
        TableGet,
        TableGrow,
        // bulk-memory + reference-types: table ops
        TableInit,
        TableSet,
        TableSize,
        TypedSelect,
        TypedSelectMulti,
        Unreachable,
    };
    matches!(
        op,
        // MVP control flow
        Unreachable
            | Nop
            | Block { .. }
            | Loop { .. }
            | If { .. }
            | Else
            | End
            | Br { .. }
            | BrIf { .. }
            | BrTable { .. }
            | Return
            | Call { .. }
            | CallIndirect { .. }
            | Drop
            | Select
            | LocalGet { .. }
            | LocalSet { .. }
            | LocalTee { .. }
            | GlobalGet { .. }
            | GlobalSet { .. }
            // MVP integer load / store
            | I32Load { .. }
            | I64Load { .. }
            | I32Load8S { .. }
            | I32Load8U { .. }
            | I32Load16S { .. }
            | I32Load16U { .. }
            | I64Load8S { .. }
            | I64Load8U { .. }
            | I64Load16S { .. }
            | I64Load16U { .. }
            | I64Load32S { .. }
            | I64Load32U { .. }
            | I32Store { .. }
            | I64Store { .. }
            | I32Store8 { .. }
            | I32Store16 { .. }
            | I64Store8 { .. }
            | I64Store16 { .. }
            | I64Store32 { .. }
            | MemorySize { .. }
            | MemoryGrow { .. }
            // MVP integer constants
            | I32Const { .. }
            | I64Const { .. }
            // MVP integer compares
            | I32Eqz
            | I32Eq
            | I32Ne
            | I32LtS
            | I32LtU
            | I32GtS
            | I32GtU
            | I32LeS
            | I32LeU
            | I32GeS
            | I32GeU
            | I64Eqz
            | I64Eq
            | I64Ne
            | I64LtS
            | I64LtU
            | I64GtS
            | I64GtU
            | I64LeS
            | I64LeU
            | I64GeS
            | I64GeU
            // MVP integer arith / bitwise / shift / rotate
            | I32Clz
            | I32Ctz
            | I32Popcnt
            | I32Add
            | I32Sub
            | I32Mul
            | I32DivS
            | I32DivU
            | I32RemS
            | I32RemU
            | I32And
            | I32Or
            | I32Xor
            | I32Shl
            | I32ShrS
            | I32ShrU
            | I32Rotl
            | I32Rotr
            | I64Clz
            | I64Ctz
            | I64Popcnt
            | I64Add
            | I64Sub
            | I64Mul
            | I64DivS
            | I64DivU
            | I64RemS
            | I64RemU
            | I64And
            | I64Or
            | I64Xor
            | I64Shl
            | I64ShrS
            | I64ShrU
            | I64Rotl
            | I64Rotr
            // MVP integer width conversions
            | I32WrapI64
            | I64ExtendI32S
            | I64ExtendI32U
            // sign-extension proposal
            | I32Extend8S
            | I32Extend16S
            | I64Extend8S
            | I64Extend16S
            | I64Extend32S
            // bulk-memory
            | MemoryInit { .. }
            | DataDrop { .. }
            | MemoryCopy { .. }
            | MemoryFill { .. }
            | TableInit { .. }
            | ElemDrop { .. }
            | TableCopy { .. }
            // reference-types
            | TypedSelect { .. }
            | TypedSelectMulti { .. }
            | RefNull { .. }
            | RefIsNull
            | RefFunc { .. }
            | TableFill { .. }
            | TableGet { .. }
            | TableSet { .. }
            | TableGrow { .. }
            | TableSize { .. }
            // tail-call
            | ReturnCall { .. }
            | ReturnCallIndirect { .. }
    )
}

/// Best-effort name for a banned operator's error message. Returns a
/// generic label for ops we don't bother to spell out — the goal is
/// debug breadcrumbs, not a stable wire format.
fn op_name(op: &Operator<'_>) -> &'static str {
    use Operator::{
        F32Add, F32Const, F32Div, F32Mul, F32Sub, F32x4Add, F32x4PMin, F32x4RelaxedMadd,
        F32x4Splat, F64Add, F64Const, F64Div, F64Mul, F64Sub, F64x2Add, V128Const, V128Load,
        V128Store,
    };
    match op {
        F32Add => "f32.add",
        F32Sub => "f32.sub",
        F32Mul => "f32.mul",
        F32Div => "f32.div",
        F64Add => "f64.add",
        F64Sub => "f64.sub",
        F64Mul => "f64.mul",
        F64Div => "f64.div",
        F32Const { .. } => "f32.const",
        F64Const { .. } => "f64.const",
        V128Load { .. } => "v128.load",
        V128Store { .. } => "v128.store",
        V128Const { .. } => "v128.const",
        F32x4Splat => "f32x4.splat",
        F32x4Add => "f32x4.add",
        F32x4PMin => "f32x4.pmin",
        F64x2Add => "f64x2.add",
        F32x4RelaxedMadd => "f32x4.relaxed_madd",
        _ => "non-deterministic op",
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::{scan_component_for_floats, scan_core_module_for_floats};

    /// Core wasm module: `(func (export "f") (result f32) f32.const 1.0 f32.const 2.0 f32.add)`.
    fn float_module_bytes() -> Vec<u8> {
        wat::parse_str(
            r#"
            (module
                (func (export "f") (result f32)
                    f32.const 1.0
                    f32.const 2.0
                    f32.add))
            "#,
        )
        .expect("valid WAT")
    }

    /// Core wasm module: `(func (export "f") (result i32) i32.const 1 i32.const 2 i32.add)`.
    fn integer_module_bytes() -> Vec<u8> {
        wat::parse_str(
            r#"
            (module
                (func (export "f") (result i32)
                    i32.const 1
                    i32.const 2
                    i32.add))
            "#,
        )
        .expect("valid WAT")
    }

    #[test]
    fn rejects_core_module_with_float_op() {
        let bytes = float_module_bytes();
        let err =
            scan_core_module_for_floats(&bytes).expect_err("module with f32.add must be rejected");
        assert!(
            err.contains("f32") || err.contains("non-deterministic"),
            "error should name a banned op: {err}"
        );
    }

    #[test]
    fn accepts_integer_only_core_module() {
        let bytes = integer_module_bytes();
        scan_core_module_for_floats(&bytes).expect("integer-only module must pass");
    }

    /// Module exercising the broader integer-op surface to catch any
    /// over-eager rejection in the whitelist (control flow, locals,
    /// memory, sign-extension).
    #[test]
    fn accepts_integer_module_with_control_flow_and_memory() {
        let bytes = wat::parse_str(
            r#"
            (module
                (memory 1)
                (func (export "f") (param i32 i32) (result i32)
                    (local i32)
                    block (result i32)
                        local.get 0
                        local.get 1
                        i32.add
                        local.tee 2
                        i32.const 0
                        i32.store
                        local.get 2
                        i32.extend8_s
                        if (result i32)
                            i32.const 1
                        else
                            i32.const 0
                        end
                    end))
            "#,
        )
        .expect("valid WAT");
        scan_core_module_for_floats(&bytes)
            .expect("integer module with control flow + memory + sign-ext must pass");
    }

    /// Component embedding a single core module that uses `f32.add`.
    /// Verifies `scan_component_for_floats` walks into nested core
    /// modules rather than only considering the component shell.
    fn float_component_bytes() -> Vec<u8> {
        wat::parse_str(
            r#"
            (component
                (core module
                    (func (export "f") (result f32)
                        f32.const 1.0
                        f32.const 2.0
                        f32.add)))
            "#,
        )
        .expect("valid component WAT")
    }

    #[test]
    fn rejects_component_with_float_in_core_module() {
        let bytes = float_component_bytes();
        let err = scan_component_for_floats(&bytes)
            .expect_err("component embedding f32.add must be rejected");
        assert!(
            err.contains("f32") || err.contains("non-deterministic"),
            "error should name a banned op: {err}"
        );
    }

    /// Component nested inside a component, with a float-using core
    /// module at the inner shell. The naive "walk top-level only"
    /// scanner would miss this; the recursive scanner must descend.
    #[test]
    fn detects_float_in_nested_component() {
        let bytes = wat::parse_str(
            r#"
            (component
                (component
                    (core module
                        (func (export "f") (result f32)
                            f32.const 1.0))))
            "#,
        )
        .expect("valid nested-component WAT");
        let err =
            scan_component_for_floats(&bytes).expect_err("nested-component float must be rejected");
        assert!(
            err.contains("f32") || err.contains("non-deterministic"),
            "error should name a banned op: {err}"
        );
    }

    /// SIMD-float `f32x4.pmin` in a core module must be rejected. The
    /// previous blacklist did not enumerate `pmin`/`pmax`/`min`/`max`
    /// for the float lanes; the whitelist catches it because the op
    /// is simply not in the allowed set.
    #[test]
    fn detects_simd_float_pmin() {
        let bytes = wat::parse_str(
            r#"
            (module
                (func (export "f") (param v128 v128) (result v128)
                    local.get 0
                    local.get 1
                    f32x4.pmin))
            "#,
        )
        .expect("valid SIMD WAT");
        let err = scan_core_module_for_floats(&bytes).expect_err("f32x4.pmin must be rejected");
        assert!(
            err.contains("f32x4") || err.contains("non-deterministic"),
            "error should name a SIMD-float op: {err}"
        );
    }

    /// Relaxed-SIMD `f32x4.relaxed_madd` is *explicitly* non-deterministic
    /// per the proposal — different platforms may fuse the multiply-add
    /// or not. Must be rejected even with a relaxed-SIMD-aware engine.
    #[test]
    fn detects_relaxed_simd_madd() {
        let bytes = wat::parse_str(
            r#"
            (module
                (func (export "f") (param v128 v128 v128) (result v128)
                    local.get 0
                    local.get 1
                    local.get 2
                    f32x4.relaxed_madd))
            "#,
        )
        .expect("valid relaxed-SIMD WAT");
        let err =
            scan_core_module_for_floats(&bytes).expect_err("f32x4.relaxed_madd must be rejected");
        assert!(
            err.contains("relaxed") || err.contains("f32x4") || err.contains("non-deterministic"),
            "error should name a relaxed-SIMD op: {err}"
        );
    }

    /// Plain `v128.const` (no float lane semantics) must still be
    /// rejected — SIMD as a whole is denied to keep determinism rules
    /// simple. (Even if a bytecode used only integer SIMD ops, the
    /// engine has SIMD off; the lint defends in depth.)
    #[test]
    fn detects_simd_v128_const() {
        let bytes = wat::parse_str(
            r#"
            (module
                (func (export "f") (result v128)
                    v128.const i32x4 0 0 0 0))
            "#,
        )
        .expect("valid SIMD WAT");
        scan_core_module_for_floats(&bytes).expect_err("v128.const must be rejected");
    }
}
