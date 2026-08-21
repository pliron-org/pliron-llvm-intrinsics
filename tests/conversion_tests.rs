// SPDX-License-Identifier: Apache-2.0
// Copyright (c) The pliron-llvm-intrinsics contributors

//! Op and conversion tests

use expect_test::expect;
use pliron::{
    builtin::ops::ModuleOp,
    context::Context,
    init_env_logger_for_tests, input_error_noloc,
    irbuild::dialect_conversion::apply_dialect_conversion,
    irfmt::parsers::spaced,
    op::{Op, verify_op},
    operation::{Operation, verify_operation},
    parsable::parse_from_str,
    printable::Printable,
    result::{ExpectOk, Result},
};
use pliron_llvm::llvm_sys::{
    core::LLVMContext,
    lljit::{JitSymbol, SimpleJIT},
};
use pliron_llvm_intrinsics::to_llvm::LLVMIntrinsicsToLLVM;

fn parse_verify_module(ctx: &mut Context, input_ir: &str) -> Result<ModuleOp> {
    let opr = parse_from_str(spaced(Operation::top_level_parser()), ctx, input_ir)?;
    verify_operation(opr, ctx)?;
    Operation::get_op::<ModuleOp>(opr, ctx).ok_or_else(|| input_error_noloc!("Not a ModuleOp"))
}

fn assert_close_f64(actual: f64, expected: f64, what: &str) {
    let tol = 1e-12 * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: got {actual}, expected {expected}"
    );
}

fn assert_close_f32(actual: f32, expected: f32, what: &str) {
    let tol = 1e-5 * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: got {actual}, expected {expected}"
    );
}

/// Test a unary float intrinsic end to end: parse it on `fp64`, `fp32` and a vector of
/// `fp64`, lower it, check each use picked up the right overload suffix, hand the module
/// to LLVM, and JIT it to confirm the lowered call computes what `$reference` computes.
macro_rules! test_unary_float_intrinsic {
    ($test:ident, $op:literal, $base:literal, $x:expr, $a:expr, $b:expr, $reference:expr) => {
        #[test]
        fn $test() {
            init_env_logger_for_tests!();
            let ctx = &mut Context::new();

            let input_ir = format!(
            r#"
                builtin.module @test_module {{
                ^entry():
                    llvm.func @test_f64: llvm.func <builtin.fp64(builtin.fp64) variadic = false> [] {{
                        ^entry(x: builtin.fp64):
                            res = {op} x : builtin.fp64;
                            llvm.return res
                    }};
                    llvm.func @test_f32: llvm.func <builtin.fp32(builtin.fp32) variadic = false> [] {{
                        ^entry(x: builtin.fp32):
                            res = {op} x : builtin.fp32;
                            llvm.return res
                    }};
                    llvm.func @test_vec: llvm.func <builtin.fp64(builtin.fp64, builtin.fp64) variadic = false> [] {{
                        ^entry(a: builtin.fp64, b: builtin.fp64):
                            i0 = llvm.constant <builtin.integer <0: i32>> : builtin.integer i32;
                            i1 = llvm.constant <builtin.integer <1: i32>> : builtin.integer i32;
                            vec_undef = llvm.undef : llvm.vector <Fixed x 2 x builtin.fp64>;
                            vec_a = llvm.insert_element vec_undef, a, i0 : llvm.vector <Fixed x 2 x builtin.fp64>;
                            vec_ab = llvm.insert_element vec_a, b, i1 : llvm.vector <Fixed x 2 x builtin.fp64>;
                            vec_res = {op} vec_ab : llvm.vector <Fixed x 2 x builtin.fp64>;
                            res_a = llvm.extract_element vec_res, i0 : builtin.fp64;
                            res_b = llvm.extract_element vec_res, i1 : builtin.fp64;
                            res = llvm.fadd <> res_a, res_b : builtin.fp64;
                            llvm.return res
                    }}
                }}
                "#,
                op = $op
            );

            let module_op = parse_verify_module(ctx, &input_ir).expect_ok(ctx);

            // Every `llvm_intrinsics.*` op is gone, replaced by `llvm.call_intrinsic`.
            apply_dialect_conversion(ctx, &mut LLVMIntrinsicsToLLVM, module_op.get_operation())
                .expect_ok(ctx);
            verify_op(&module_op, ctx).expect_ok(ctx);
            let printed = module_op.disp(ctx).to_string();
            assert!(
                !printed.contains($op),
                "{} survived dialect conversion:\n{printed}",
                $op
            );

            let llvm_ctx = LLVMContext::default();
            let llvm_ir =
                pliron_llvm::to_llvm_ir::convert_module(ctx, &llvm_ctx, module_op).expect_ok(ctx);
            llvm_ir
                .verify()
                .inspect_err(|e| println!("LLVM-IR verification failed: {}", e))
                .unwrap();

            // Each use is overloaded on its own operand type.
            let ir = llvm_ir.to_string();
            for suffix in ["f64", "f32", "v2f64"] {
                let mangled = format!(concat!("@llvm.", $base, ".{}("), suffix);
                assert!(ir.contains(&mangled), "missing {mangled} in:\n{ir}");
            }

            let reference: fn(f64) -> f64 = $reference;
            let jit = SimpleJIT::new(llvm_ctx, llvm_ir).expect("SimpleJIT creation failed");

            let f64_fn: JitSymbol<fn(f64) -> f64> = unsafe {
                jit.lookup_symbol("test_f64")
                    .expect("Couldn't lookup symbol")
            };
            assert_close_f64(f64_fn($x), reference($x), concat!($op, " on fp64"));

            let f32_fn: JitSymbol<fn(f32) -> f32> = unsafe {
                jit.lookup_symbol("test_f32")
                    .expect("Couldn't lookup symbol")
            };
            assert_close_f32(
                f32_fn($x as f32),
                reference($x) as f32,
                concat!($op, " on fp32"),
            );

            let vec_fn: JitSymbol<fn(f64, f64) -> f64> = unsafe {
                jit.lookup_symbol("test_vec")
                    .expect("Couldn't lookup symbol")
            };
            assert_close_f64(
                vec_fn($a, $b),
                reference($a) + reference($b),
                concat!($op, " on a vector of fp64"),
            );
        }
    };
}

test_unary_float_intrinsic!(
    test_fabs,
    "llvm_intrinsics.fabs",
    "fabs",
    -0.5,
    -0.25,
    0.75,
    |x| x.abs()
);
test_unary_float_intrinsic!(
    test_sqrt,
    "llvm_intrinsics.sqrt",
    "sqrt",
    0.5,
    0.25,
    0.75,
    f64::sqrt
);

// Trigonometric.
test_unary_float_intrinsic!(
    test_sin,
    "llvm_intrinsics.sin",
    "sin",
    0.5,
    0.25,
    0.75,
    f64::sin
);
test_unary_float_intrinsic!(
    test_cos,
    "llvm_intrinsics.cos",
    "cos",
    0.5,
    0.25,
    0.75,
    f64::cos
);
test_unary_float_intrinsic!(
    test_tan,
    "llvm_intrinsics.tan",
    "tan",
    0.5,
    0.25,
    0.75,
    f64::tan
);
test_unary_float_intrinsic!(
    test_asin,
    "llvm_intrinsics.asin",
    "asin",
    0.5,
    0.25,
    0.75,
    f64::asin
);
test_unary_float_intrinsic!(
    test_acos,
    "llvm_intrinsics.acos",
    "acos",
    0.5,
    0.25,
    0.75,
    f64::acos
);
test_unary_float_intrinsic!(
    test_atan,
    "llvm_intrinsics.atan",
    "atan",
    0.5,
    0.25,
    0.75,
    f64::atan
);

// Hyperbolic.
test_unary_float_intrinsic!(
    test_sinh,
    "llvm_intrinsics.sinh",
    "sinh",
    0.5,
    0.25,
    0.75,
    f64::sinh
);
test_unary_float_intrinsic!(
    test_cosh,
    "llvm_intrinsics.cosh",
    "cosh",
    0.5,
    0.25,
    0.75,
    f64::cosh
);
test_unary_float_intrinsic!(
    test_tanh,
    "llvm_intrinsics.tanh",
    "tanh",
    0.5,
    0.25,
    0.75,
    f64::tanh
);

// Exponential and logarithmic.
test_unary_float_intrinsic!(
    test_exp,
    "llvm_intrinsics.exp",
    "exp",
    0.5,
    0.25,
    0.75,
    f64::exp
);
test_unary_float_intrinsic!(
    test_exp2,
    "llvm_intrinsics.exp2",
    "exp2",
    0.5,
    0.25,
    0.75,
    f64::exp2
);
test_unary_float_intrinsic!(
    test_exp10,
    "llvm_intrinsics.exp10",
    "exp10",
    0.5,
    0.25,
    0.75,
    |x| 10f64.powf(x)
);
test_unary_float_intrinsic!(
    test_log,
    "llvm_intrinsics.log",
    "log",
    0.5,
    0.25,
    0.75,
    f64::ln
);
test_unary_float_intrinsic!(
    test_log2,
    "llvm_intrinsics.log2",
    "log2",
    0.5,
    0.25,
    0.75,
    f64::log2
);
test_unary_float_intrinsic!(
    test_log10,
    "llvm_intrinsics.log10",
    "log10",
    0.5,
    0.25,
    0.75,
    f64::log10
);

// Rounding. The inputs are chosen so that ties-away-from-zero (`round`) and
// ties-to-even (`roundeven`, `rint`, `nearbyint`) disagree.
test_unary_float_intrinsic!(
    test_floor,
    "llvm_intrinsics.floor",
    "floor",
    2.5,
    -3.75,
    0.5,
    f64::floor
);
test_unary_float_intrinsic!(
    test_ceil,
    "llvm_intrinsics.ceil",
    "ceil",
    2.5,
    -3.75,
    0.5,
    f64::ceil
);
test_unary_float_intrinsic!(
    test_trunc,
    "llvm_intrinsics.trunc",
    "trunc",
    2.5,
    -3.75,
    0.5,
    f64::trunc
);
test_unary_float_intrinsic!(
    test_rint,
    "llvm_intrinsics.rint",
    "rint",
    2.5,
    -3.75,
    0.5,
    f64::round_ties_even
);
test_unary_float_intrinsic!(
    test_nearbyint,
    "llvm_intrinsics.nearbyint",
    "nearbyint",
    2.5,
    -3.75,
    0.5,
    f64::round_ties_even
);
test_unary_float_intrinsic!(
    test_round,
    "llvm_intrinsics.round",
    "round",
    2.5,
    -3.75,
    0.5,
    f64::round
);
test_unary_float_intrinsic!(
    test_roundeven,
    "llvm_intrinsics.roundeven",
    "roundeven",
    2.5,
    -3.75,
    0.5,
    f64::round_ties_even
);

// Misc. `canonicalize` is the identity on the normal values used here.
test_unary_float_intrinsic!(
    test_canonicalize,
    "llvm_intrinsics.canonicalize",
    "canonicalize",
    0.5,
    0.25,
    0.75,
    |x| x
);

#[test]
fn test_all_intrinsics_lowering_snapshot() {
    init_env_logger_for_tests!();
    let ctx = &mut Context::new();

    let input_ir = r#"
        builtin.module @test_module {
          ^entry():
            llvm.func @all_f32: llvm.func <builtin.fp32(builtin.fp32) variadic = false> [] {
                ^entry(x: builtin.fp32):
                    r0 = llvm_intrinsics.fabs x : builtin.fp32;
                    r1 = llvm_intrinsics.sqrt r0 : builtin.fp32;
                    r2 = llvm_intrinsics.sin r1 : builtin.fp32;
                    r3 = llvm_intrinsics.cos r2 : builtin.fp32;
                    r4 = llvm_intrinsics.tan r3 : builtin.fp32;
                    r5 = llvm_intrinsics.asin r4 : builtin.fp32;
                    r6 = llvm_intrinsics.acos r5 : builtin.fp32;
                    r7 = llvm_intrinsics.atan r6 : builtin.fp32;
                    r8 = llvm_intrinsics.sinh r7 : builtin.fp32;
                    r9 = llvm_intrinsics.cosh r8 : builtin.fp32;
                    r10 = llvm_intrinsics.tanh r9 : builtin.fp32;
                    r11 = llvm_intrinsics.exp r10 : builtin.fp32;
                    r12 = llvm_intrinsics.exp2 r11 : builtin.fp32;
                    r13 = llvm_intrinsics.exp10 r12 : builtin.fp32;
                    r14 = llvm_intrinsics.log r13 : builtin.fp32;
                    r15 = llvm_intrinsics.log2 r14 : builtin.fp32;
                    r16 = llvm_intrinsics.log10 r15 : builtin.fp32;
                    r17 = llvm_intrinsics.floor r16 : builtin.fp32;
                    r18 = llvm_intrinsics.ceil r17 : builtin.fp32;
                    r19 = llvm_intrinsics.trunc r18 : builtin.fp32;
                    r20 = llvm_intrinsics.rint r19 : builtin.fp32;
                    r21 = llvm_intrinsics.nearbyint r20 : builtin.fp32;
                    r22 = llvm_intrinsics.round r21 : builtin.fp32;
                    r23 = llvm_intrinsics.roundeven r22 : builtin.fp32;
                    r24 = llvm_intrinsics.canonicalize r23 : builtin.fp32;
                    llvm.return r24
            };
            llvm.func @overloads: llvm.func <builtin.fp64(builtin.fp64) variadic = false> [] {
                ^entry(x: builtin.fp64):
                    i0 = llvm.constant <builtin.integer <0: i32>> : builtin.integer i32;
                    s64 = llvm_intrinsics.sqrt x : builtin.fp64;
                    vec_undef = llvm.undef : llvm.vector <Fixed x 2 x builtin.fp64>;
                    vec_x = llvm.insert_element vec_undef, s64, i0 : llvm.vector <Fixed x 2 x builtin.fp64>;
                    vec_s = llvm_intrinsics.sqrt vec_x : llvm.vector <Fixed x 2 x builtin.fp64>;
                    res = llvm.extract_element vec_s, i0 : builtin.fp64;
                    llvm.return res
            }
        }
        "#;

    let module_op = parse_verify_module(ctx, input_ir).expect_ok(ctx);

    apply_dialect_conversion(ctx, &mut LLVMIntrinsicsToLLVM, module_op.get_operation())
        .expect_ok(ctx);
    verify_op(&module_op, ctx).expect_ok(ctx);

    expect![[r#"
        builtin.module @test_module 
        {
          ^entry_block1v1() !0:
            llvm.func @all_f32: llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false>
              [] 
            {
              ^entry_block2v1(x_v0: builtin.fp32 ) !1:
                r0_v33 = llvm.call_intrinsic @"llvm.fabs.f32" (x_v0) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !2;
                r1_v34 = llvm.call_intrinsic @"llvm.sqrt.f32" (r0_v33) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !3;
                r2_v35 = llvm.call_intrinsic @"llvm.sin.f32" (r1_v34) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !4;
                r3_v36 = llvm.call_intrinsic @"llvm.cos.f32" (r2_v35) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !5;
                r4_v37 = llvm.call_intrinsic @"llvm.tan.f32" (r3_v36) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !6;
                r5_v38 = llvm.call_intrinsic @"llvm.asin.f32" (r4_v37) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !7;
                r6_v39 = llvm.call_intrinsic @"llvm.acos.f32" (r5_v38) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !8;
                r7_v40 = llvm.call_intrinsic @"llvm.atan.f32" (r6_v39) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !9;
                r8_v41 = llvm.call_intrinsic @"llvm.sinh.f32" (r7_v40) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !10;
                r9_v42 = llvm.call_intrinsic @"llvm.cosh.f32" (r8_v41) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !11;
                r10_v43 = llvm.call_intrinsic @"llvm.tanh.f32" (r9_v42) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !12;
                r11_v44 = llvm.call_intrinsic @"llvm.exp.f32" (r10_v43) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !13;
                r12_v45 = llvm.call_intrinsic @"llvm.exp2.f32" (r11_v44) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !14;
                r13_v46 = llvm.call_intrinsic @"llvm.exp10.f32" (r12_v45) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !15;
                r14_v47 = llvm.call_intrinsic @"llvm.log.f32" (r13_v46) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !16;
                r15_v48 = llvm.call_intrinsic @"llvm.log2.f32" (r14_v47) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !17;
                r16_v49 = llvm.call_intrinsic @"llvm.log10.f32" (r15_v48) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !18;
                r17_v50 = llvm.call_intrinsic @"llvm.floor.f32" (r16_v49) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !19;
                r18_v51 = llvm.call_intrinsic @"llvm.ceil.f32" (r17_v50) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !20;
                r19_v52 = llvm.call_intrinsic @"llvm.trunc.f32" (r18_v51) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !21;
                r20_v53 = llvm.call_intrinsic @"llvm.rint.f32" (r19_v52) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !22;
                r21_v54 = llvm.call_intrinsic @"llvm.nearbyint.f32" (r20_v53) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !23;
                r22_v55 = llvm.call_intrinsic @"llvm.round.f32" (r21_v54) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !24;
                r23_v56 = llvm.call_intrinsic @"llvm.roundeven.f32" (r22_v55) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !25;
                r24_v57 = llvm.call_intrinsic @"llvm.canonicalize.f32" (r23_v56) : llvm.func <builtin.fp32 (builtin.fp32 ) variadic = false> !26;
                llvm.return r24_v57 !27
            } !28;
            llvm.func @overloads: llvm.func <builtin.fp64 (builtin.fp64 ) variadic = false>
              [] 
            {
              ^entry_block3v1(x_v26: builtin.fp64 ) !29:
                i0_v27 = llvm.constant <builtin.integer <0: i32>> : builtin.integer i32 !30;
                s64_v58 = llvm.call_intrinsic @"llvm.sqrt.f64" (x_v26) : llvm.func <builtin.fp64 (builtin.fp64 ) variadic = false> !31;
                vec_undef_v29 = llvm.undef : llvm.vector <Fixed x 2 x builtin.fp64 > !32;
                vec_x_v30 = llvm.insert_element vec_undef_v29, s64_v58, i0_v27 : llvm.vector <Fixed x 2 x builtin.fp64 > !33;
                vec_s_v59 = llvm.call_intrinsic @"llvm.sqrt.v2f64" (vec_x_v30) : llvm.func <llvm.vector <Fixed x 2 x builtin.fp64 >(llvm.vector <Fixed x 2 x builtin.fp64 >) variadic = false> !34;
                res_v32 = llvm.extract_element vec_s_v59, i0_v27 : builtin.fp64  !35;
                llvm.return res_v32 !36
            } !37
        }"#]].assert_eq(&module_op.disp(ctx).to_string());

    let llvm_ctx = LLVMContext::default();
    let llvm_ir = pliron_llvm::to_llvm_ir::convert_module(ctx, &llvm_ctx, module_op).expect_ok(ctx);
    llvm_ir
        .verify()
        .inspect_err(|e| println!("LLVM-IR verification failed: {}", e))
        .unwrap();

    expect![[r#"
        ; ModuleID = 'test_module'
        source_filename = "test_module"

        define float @all_f32(float %0) {
        entry_block2v1:
          %r0_v33 = call float @llvm.fabs.f32(float %0)
          %r1_v34 = call float @llvm.sqrt.f32(float %r0_v33)
          %r2_v35 = call float @llvm.sin.f32(float %r1_v34)
          %r3_v36 = call float @llvm.cos.f32(float %r2_v35)
          %r4_v37 = call float @llvm.tan.f32(float %r3_v36)
          %r5_v38 = call float @llvm.asin.f32(float %r4_v37)
          %r6_v39 = call float @llvm.acos.f32(float %r5_v38)
          %r7_v40 = call float @llvm.atan.f32(float %r6_v39)
          %r8_v41 = call float @llvm.sinh.f32(float %r7_v40)
          %r9_v42 = call float @llvm.cosh.f32(float %r8_v41)
          %r10_v43 = call float @llvm.tanh.f32(float %r9_v42)
          %r11_v44 = call float @llvm.exp.f32(float %r10_v43)
          %r12_v45 = call float @llvm.exp2.f32(float %r11_v44)
          %r13_v46 = call float @llvm.exp10.f32(float %r12_v45)
          %r14_v47 = call float @llvm.log.f32(float %r13_v46)
          %r15_v48 = call float @llvm.log2.f32(float %r14_v47)
          %r16_v49 = call float @llvm.log10.f32(float %r15_v48)
          %r17_v50 = call float @llvm.floor.f32(float %r16_v49)
          %r18_v51 = call float @llvm.ceil.f32(float %r17_v50)
          %r19_v52 = call float @llvm.trunc.f32(float %r18_v51)
          %r20_v53 = call float @llvm.rint.f32(float %r19_v52)
          %r21_v54 = call float @llvm.nearbyint.f32(float %r20_v53)
          %r22_v55 = call float @llvm.round.f32(float %r21_v54)
          %r23_v56 = call float @llvm.roundeven.f32(float %r22_v55)
          %r24_v57 = call float @llvm.canonicalize.f32(float %r23_v56)
          ret float %r24_v57
        }

        define double @overloads(double %0) {
        entry_block3v1:
          %s64_v58 = call double @llvm.sqrt.f64(double %0)
          %vec_x_v30 = insertelement <2 x double> undef, double %s64_v58, i32 0
          %vec_s_v59 = call <2 x double> @llvm.sqrt.v2f64(<2 x double> %vec_x_v30)
          %res_v32 = extractelement <2 x double> %vec_s_v59, i32 0
          ret double %res_v32
        }

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.fabs.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.sqrt.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.sin.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.cos.f32(float) #0

        ; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.tan.f32(float) #1

        ; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.asin.f32(float) #1

        ; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.acos.f32(float) #1

        ; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.atan.f32(float) #1

        ; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.sinh.f32(float) #1

        ; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.cosh.f32(float) #1

        ; Function Attrs: nocallback nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.tanh.f32(float) #1

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.exp.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.exp2.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.exp10.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.log.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.log2.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.log10.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.floor.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.ceil.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.trunc.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.rint.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.nearbyint.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.round.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.roundeven.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.canonicalize.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare double @llvm.sqrt.f64(double) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare <2 x double> @llvm.sqrt.v2f64(<2 x double>) #0

        attributes #0 = { nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }
        attributes #1 = { nocallback nofree nosync nounwind speculatable willreturn memory(none) }
    "#]].assert_eq(&llvm_ir.to_string());
}
