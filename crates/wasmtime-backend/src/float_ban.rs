//! Byte-level float-ban lint per determinism.md §5.2.
//!
//! Components that import or use float instructions in any function
//! body fail this lint and are rejected at instantiation. Includes
//! SIMD-float ops (cross-platform divergence vectors per §5.2).

use wasmparser::{Operator, Parser, Payload};

/// Scan a core wasm module's function bodies for any float instruction.
///
/// Returns `Err` naming the first banned instruction encountered.
///
/// # Errors
///
/// Returns `Err` if the bytes are not a valid wasm core module, or if any
/// instruction in any function body is a banned float / SIMD-float op.
pub fn scan_core_module_for_floats(bytes: &[u8]) -> Result<(), String> {
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|e| format!("wasm parse error: {e}"))?;
        if let Payload::CodeSectionEntry(body) = payload {
            let mut reader = body
                .get_operators_reader()
                .map_err(|e| format!("operators reader error: {e}"))?;
            while !reader.eof() {
                let op = reader.read().map_err(|e| format!("read op error: {e}"))?;
                if is_float_op(&op) {
                    return Err(format!("banned float op: {}", op_name(&op)));
                }
            }
        }
    }
    Ok(())
}

/// Scan an entire component's embedded core modules for float
/// instructions in any function body. Each embedded core module is
/// passed through [`scan_core_module_for_floats`].
///
/// # Errors
///
/// Returns `Err` if the bytes are not a valid wasm component, if a
/// `ModuleSection` byte range is out of bounds, or if any embedded core
/// module fails [`scan_core_module_for_floats`].
pub fn scan_component_for_floats(component_bytes: &[u8]) -> Result<(), String> {
    for payload in Parser::new(0).parse_all(component_bytes) {
        let payload = payload.map_err(|e| format!("component parse error: {e}"))?;
        if let Payload::ModuleSection {
            unchecked_range, ..
        } = payload
        {
            let module_bytes = component_bytes
                .get(unchecked_range.clone())
                .ok_or_else(|| {
                    format!("component module section range {unchecked_range:?} out of bounds")
                })?;
            scan_core_module_for_floats(module_bytes)?;
        }
    }
    Ok(())
}

fn is_float_op(op: &Operator<'_>) -> bool {
    use Operator::{
        F32Abs, F32Add, F32Ceil, F32Const, F32ConvertI32S, F32ConvertI32U, F32ConvertI64S,
        F32ConvertI64U, F32Copysign, F32DemoteF64, F32Div, F32Eq, F32Floor, F32Ge, F32Gt, F32Le,
        F32Load, F32Lt, F32Max, F32Min, F32Mul, F32Ne, F32Nearest, F32Neg, F32ReinterpretI32,
        F32Sqrt, F32Store, F32Sub, F32Trunc, F32x4Add, F32x4Div, F32x4ExtractLane, F32x4Mul,
        F32x4ReplaceLane, F32x4Splat, F32x4Sub, F64Abs, F64Add, F64Ceil, F64Const, F64ConvertI32S,
        F64ConvertI32U, F64ConvertI64S, F64ConvertI64U, F64Copysign, F64Div, F64Eq, F64Floor,
        F64Ge, F64Gt, F64Le, F64Load, F64Lt, F64Max, F64Min, F64Mul, F64Ne, F64Nearest, F64Neg,
        F64PromoteF32, F64ReinterpretI64, F64Sqrt, F64Store, F64Sub, F64Trunc, F64x2Add, F64x2Div,
        F64x2ExtractLane, F64x2Mul, F64x2ReplaceLane, F64x2Splat, F64x2Sub, I32ReinterpretF32,
        I32TruncF32S, I32TruncF32U, I32TruncF64S, I32TruncF64U, I64ReinterpretF64, I64TruncF32S,
        I64TruncF32U, I64TruncF64S, I64TruncF64U, V128Load, V128Store,
    };
    matches!(
        op,
        F32Load { .. }
            | F64Load { .. }
            | F32Store { .. }
            | F64Store { .. }
            | F32Const { .. }
            | F64Const { .. }
            | F32Eq
            | F32Ne
            | F32Lt
            | F32Gt
            | F32Le
            | F32Ge
            | F64Eq
            | F64Ne
            | F64Lt
            | F64Gt
            | F64Le
            | F64Ge
            | F32Abs
            | F32Neg
            | F32Ceil
            | F32Floor
            | F32Trunc
            | F32Nearest
            | F32Sqrt
            | F32Add
            | F32Sub
            | F32Mul
            | F32Div
            | F32Min
            | F32Max
            | F32Copysign
            | F64Abs
            | F64Neg
            | F64Ceil
            | F64Floor
            | F64Trunc
            | F64Nearest
            | F64Sqrt
            | F64Add
            | F64Sub
            | F64Mul
            | F64Div
            | F64Min
            | F64Max
            | F64Copysign
            | I32TruncF32S
            | I32TruncF32U
            | I32TruncF64S
            | I32TruncF64U
            | I64TruncF32S
            | I64TruncF32U
            | I64TruncF64S
            | I64TruncF64U
            | F32ConvertI32S
            | F32ConvertI32U
            | F32ConvertI64S
            | F32ConvertI64U
            | F64ConvertI32S
            | F64ConvertI32U
            | F64ConvertI64S
            | F64ConvertI64U
            | F32DemoteF64
            | F64PromoteF32
            | I32ReinterpretF32
            | I64ReinterpretF64
            | F32ReinterpretI32
            | F64ReinterpretI64
            // SIMD-float ops are also banned per §5.2.
            | V128Load { .. }
            | V128Store { .. }
            | F32x4Splat
            | F64x2Splat
            | F32x4ExtractLane { .. }
            | F32x4ReplaceLane { .. }
            | F64x2ExtractLane { .. }
            | F64x2ReplaceLane { .. }
            | F32x4Add
            | F32x4Sub
            | F32x4Mul
            | F32x4Div
            | F64x2Add
            | F64x2Sub
            | F64x2Mul
            | F64x2Div
    )
}

fn op_name(op: &Operator<'_>) -> &'static str {
    // wasmparser's Display would allocate; we just name the obvious
    // banned cases. Anything else returns "float op".
    use Operator::{
        F32Add, F32Const, F32Div, F32Mul, F32Sub, F64Add, F64Const, F64Div, F64Mul, F64Sub,
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
        _ => "float op",
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
            err.contains("f32") || err.contains("float"),
            "error should name a float op: {err}"
        );
    }

    #[test]
    fn accepts_integer_only_core_module() {
        let bytes = integer_module_bytes();
        scan_core_module_for_floats(&bytes).expect("integer-only module must pass");
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
            err.contains("f32") || err.contains("float"),
            "error should name a float op: {err}"
        );
    }
}
