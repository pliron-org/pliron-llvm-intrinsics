// SPDX-License-Identifier: Apache-2.0
// Copyright (c) The pliron-llvm-intrinsics contributors

//! Op and conversion tests

use expect_test::expect;
use pliron::{
    builtin::ops::ModuleOp,
    combine::Parser,
    context::Context,
    init_env_logger_for_tests, input_error_noloc,
    irbuild::dialect_conversion::apply_dialect_conversion,
    irfmt::parsers::spaced,
    location,
    op::verify_op,
    operation::Operation,
    parsable::{self, state_stream_from_iterator},
    printable::Printable,
    result::ExpectOk,
};
use pliron_llvm::llvm_sys::{
    core::LLVMContext,
    lljit::{JitSymbol, SimpleJIT},
};
use pliron_llvm_intrinsics::to_llvm::LLVMIntrinsicsToLLVM;

fn parse_module(ctx: &mut Context, input_ir: &str) -> pliron::context::Ptr<Operation> {
    let state_stream = state_stream_from_iterator(
        input_ir.chars(),
        parsable::State::new(ctx, location::Source::InMemory),
    );
    let parsed = spaced(Operation::top_level_parser())
        .parse(state_stream)
        .map(|(op, _)| op)
        .map_err(|err| input_error_noloc!(err));
    parsed.expect_ok(ctx)
}

// `llvm_intrinsics.fabs` parsing/printing, lowering to LLVM and execution test.
#[test]
fn test_fabs_op() {
    init_env_logger_for_tests!();
    let ctx = &mut Context::new();

    let input_ir = r#"
        builtin.module @test_module {
          ^entry():
            llvm.func @test_fabs: llvm.func <builtin.fp64(builtin.fp64, builtin.fp32) variadic = false> [] {
                ^entry(x: builtin.fp64, y: builtin.fp32):
                    res64 = llvm_intrinsics.fabs x : builtin.fp64;
                    res32 = llvm_intrinsics.fabs y : builtin.fp32;
                    res32_ext = llvm.fpext <> res32 to builtin.fp64;
                    res = llvm.fadd <> res64, res32_ext : builtin.fp64;
                    llvm.return res
            };
            llvm.func @test_fabs_vec: llvm.func <builtin.fp64(builtin.fp64, builtin.fp64) variadic = false> [] {
                ^entry(a: builtin.fp64, b: builtin.fp64):
                    i0 = llvm.constant <builtin.integer <0: i32>> : builtin.integer i32;
                    i1 = llvm.constant <builtin.integer <1: i32>> : builtin.integer i32;
                    vec_undef = llvm.undef : llvm.vector <Fixed x 2 x builtin.fp64>;
                    vec_a = llvm.insert_element vec_undef, a, i0 : llvm.vector <Fixed x 2 x builtin.fp64>;
                    vec_ab = llvm.insert_element vec_a, b, i1 : llvm.vector <Fixed x 2 x builtin.fp64>;
                    vec_abs = llvm_intrinsics.fabs vec_ab : llvm.vector <Fixed x 2 x builtin.fp64>;
                    abs_a = llvm.extract_element vec_abs, i0 : builtin.fp64;
                    abs_b = llvm.extract_element vec_abs, i1 : builtin.fp64;
                    vec_res = llvm.fadd <> abs_a, abs_b : builtin.fp64;
                    llvm.return vec_res
            }
        }
        "#;

    let parsed_op = parse_module(ctx, input_ir);
    let module_op = Operation::get_op::<ModuleOp>(parsed_op, ctx).unwrap();
    verify_op(&module_op, ctx).expect_ok(ctx);

    expect![[r#"
        builtin.module @test_module 
        {
          ^entry_block1v1() !0:
            llvm.func @test_fabs: llvm.func <builtin.fp64 (builtin.fp64 , builtin.fp32 ) variadic = false>
              [] 
            {
              ^entry_block2v1(x_v0: builtin.fp64 , y_v1: builtin.fp32 ) !1:
                res64_v2 = llvm_intrinsics.fabs x_v0 : builtin.fp64  !2;
                res32_v3 = llvm_intrinsics.fabs y_v1 : builtin.fp32  !3;
                res32_ext_v4 = llvm.fpext <> res32_v3 to builtin.fp64  !4;
                res_v5 = llvm.fadd <> res64_v2, res32_ext_v4 : builtin.fp64  !5;
                llvm.return res_v5 !6
            } !7;
            llvm.func @test_fabs_vec: llvm.func <builtin.fp64 (builtin.fp64 , builtin.fp64 ) variadic = false>
              [] 
            {
              ^entry_block3v1(a_v6: builtin.fp64 , b_v7: builtin.fp64 ) !8:
                i0_v8 = llvm.constant <builtin.integer <0: i32>> : builtin.integer i32 !9;
                i1_v9 = llvm.constant <builtin.integer <1: i32>> : builtin.integer i32 !10;
                vec_undef_v10 = llvm.undef : llvm.vector <Fixed x 2 x builtin.fp64 > !11;
                vec_a_v11 = llvm.insert_element vec_undef_v10, a_v6, i0_v8 : llvm.vector <Fixed x 2 x builtin.fp64 > !12;
                vec_ab_v12 = llvm.insert_element vec_a_v11, b_v7, i1_v9 : llvm.vector <Fixed x 2 x builtin.fp64 > !13;
                vec_abs_v13 = llvm_intrinsics.fabs vec_ab_v12 : llvm.vector <Fixed x 2 x builtin.fp64 > !14;
                abs_a_v14 = llvm.extract_element vec_abs_v13, i0_v8 : builtin.fp64  !15;
                abs_b_v15 = llvm.extract_element vec_abs_v13, i1_v9 : builtin.fp64  !16;
                vec_res_v16 = llvm.fadd <> abs_a_v14, abs_b_v15 : builtin.fp64  !17;
                llvm.return vec_res_v16 !18
            } !19
        }"#]]
    .assert_eq(&module_op.disp(ctx).to_string());

    // `llvm_intrinsics.fabs` lowers to `llvm.call_intrinsic @llvm.fabs.*`.
    apply_dialect_conversion(ctx, &mut LLVMIntrinsicsToLLVM, parsed_op).expect_ok(ctx);
    verify_op(&module_op, ctx).expect_ok(ctx);

    let llvm_ctx = LLVMContext::default();
    let llvm_ir = pliron_llvm::to_llvm_ir::convert_module(ctx, &llvm_ctx, module_op).expect_ok(ctx);
    llvm_ir
        .verify()
        .inspect_err(|e| println!("LLVM-IR verification failed: {}", e))
        .unwrap();

    expect![[r#"
        ; ModuleID = 'test_module'
        source_filename = "test_module"

        define double @test_fabs(double %0, float %1) {
        entry_block2v1:
          %res64_v17 = call double @llvm.fabs.f64(double %0)
          %res32_v18 = call float @llvm.fabs.f32(float %1)
          %res32_ext_v4 = fpext float %res32_v18 to double
          %res_v5 = fadd double %res64_v17, %res32_ext_v4
          ret double %res_v5
        }

        define double @test_fabs_vec(double %0, double %1) {
        entry_block3v1:
          %vec_a_v11 = insertelement <2 x double> undef, double %0, i32 0
          %vec_ab_v12 = insertelement <2 x double> %vec_a_v11, double %1, i32 1
          %vec_abs_v19 = call <2 x double> @llvm.fabs.v2f64(<2 x double> %vec_ab_v12)
          %abs_a_v14 = extractelement <2 x double> %vec_abs_v19, i32 0
          %abs_b_v15 = extractelement <2 x double> %vec_abs_v19, i32 1
          %vec_res_v16 = fadd double %abs_a_v14, %abs_b_v15
          ret double %vec_res_v16
        }

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare double @llvm.fabs.f64(double) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.fabs.f32(float) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare <2 x double> @llvm.fabs.v2f64(<2 x double>) #0

        attributes #0 = { nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none) }
    "#]]
    .assert_eq(&llvm_ir.to_string());

    let jit = SimpleJIT::new(llvm_ctx, llvm_ir).expect("SimpleJIT creation failed");
    let f: JitSymbol<fn(f64, f32) -> f64> = unsafe {
        jit.lookup_symbol("test_fabs")
            .expect("Couldn't lookup symbol")
    };
    assert_eq!(f(-3.5, -1.5), 5.0);
    assert_eq!(f(3.5, 1.5), 5.0);
    assert_eq!(f(-3.5, 1.5), 5.0);

    let f_vec: JitSymbol<fn(f64, f64) -> f64> = unsafe {
        jit.lookup_symbol("test_fabs_vec")
            .expect("Couldn't lookup symbol")
    };
    assert_eq!(f_vec(-3.5, -1.5), 5.0);
    assert_eq!(f_vec(3.5, 1.5), 5.0);
    assert_eq!(f_vec(-3.5, 1.5), 5.0);
}
