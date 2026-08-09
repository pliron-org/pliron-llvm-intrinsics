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

// `llvm_intrinsics.fabs` parsing/printing test.
#[test]
fn test_fabs_op_parse_print_verify() {
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
            } !7
        }"#]]
    .assert_eq(&module_op.disp(ctx).to_string());
}

// `llvm_intrinsics.fabs` lowers to `llvm.call_intrinsic @llvm.fabs.*`.
#[test]
fn test_fabs_op_to_llvm_conversion() {
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
            }
        }
        "#;

    let parsed_op = parse_module(ctx, input_ir);
    let module_op = Operation::get_op::<ModuleOp>(parsed_op, ctx).unwrap();
    verify_op(&module_op, ctx).expect_ok(ctx);

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
          %res64_v6 = call double @llvm.fabs.f64(double %0)
          %res32_v7 = call float @llvm.fabs.f32(float %1)
          %res32_ext_v4 = fpext float %res32_v7 to double
          %res_v5 = fadd double %res64_v6, %res32_ext_v4
          ret double %res_v5
        }

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare double @llvm.fabs.f64(double) #0

        ; Function Attrs: nocallback nocreateundeforpoison nofree nosync nounwind speculatable willreturn memory(none)
        declare float @llvm.fabs.f32(float) #0

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
}
