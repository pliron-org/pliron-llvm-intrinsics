// SPDX-License-Identifier: Apache-2.0
// Copyright (c) The pliron-llvm-intrinsics contributors

//! Convert the LLVM intrinsics dialect to the LLVM dialect.

use pliron::{
    builtin::attributes::StringAttr,
    context::{Context, Ptr},
    derive::op_interface_impl,
    irbuild::{
        dialect_conversion::{DialectConversion, DialectConversionRewriter, OperandsInfo},
        inserter::Inserter,
        rewriter::Rewriter,
    },
    op::{Op, op_cast, op_impls},
    operation::Operation,
    result::Result,
};
use pliron_llvm::{ToLLVMDialect, ops::CallIntrinsicOp, types::FuncType};

use crate::{mangling::llvm_mangled_ty, ops::*};

/// Lower a unary float intrinsic [Op] to a [`CallIntrinsicOp`] calling `llvm.<opname>`,
/// overloaded on the operand type.
fn rewrite_unary_float_intrinsic(
    op: &dyn UnaryFloatIntrinsicInterface,
    ctx: &mut Context,
    rewriter: &mut DialectConversionRewriter,
) -> Result<()> {
    let arg = op.get_operand(ctx);
    let arg_ty = op.operand_type(ctx);

    let base_name = op.get_opid().name;
    let suffix = llvm_mangled_ty(ctx, arg_ty, op.loc(ctx))?;
    let intrinsic_name: StringAttr = format!("llvm.{}.{}", base_name, suffix).into();

    let func_ty = FuncType::get(ctx, arg_ty, vec![arg_ty], false);
    let call_op = CallIntrinsicOp::new(ctx, intrinsic_name, func_ty, vec![arg]);
    rewriter.insert_op(ctx, &call_op);
    rewriter.replace_operation(ctx, op.get_operation(), call_op.get_operation());
    Ok(())
}

/// Implement ToLLVMDialect for a unary float intrinsic,
/// wrapping around a call to `rewrite_unary_float_intrinsic`
macro_rules! lower_unary_float_intrinsic {
    ($op:ty, $base:literal) => {
        #[op_interface_impl]
        impl ToLLVMDialect for $op {
            fn rewrite(
                &self,
                ctx: &mut Context,
                rewriter: &mut DialectConversionRewriter,
                _operands_info: &OperandsInfo,
            ) -> Result<()> {
                rewrite_unary_float_intrinsic(self, ctx, rewriter)
            }
        }
    };
}

lower_unary_float_intrinsic!(FAbsOp, "fabs");
lower_unary_float_intrinsic!(SqrtOp, "sqrt");

// Trigonometric.
lower_unary_float_intrinsic!(SinOp, "sin");
lower_unary_float_intrinsic!(CosOp, "cos");
lower_unary_float_intrinsic!(TanOp, "tan");
lower_unary_float_intrinsic!(AsinOp, "asin");
lower_unary_float_intrinsic!(AcosOp, "acos");
lower_unary_float_intrinsic!(AtanOp, "atan");

// Hyperbolic.
lower_unary_float_intrinsic!(SinhOp, "sinh");
lower_unary_float_intrinsic!(CoshOp, "cosh");
lower_unary_float_intrinsic!(TanhOp, "tanh");

// Exponential and logarithmic.
lower_unary_float_intrinsic!(ExpOp, "exp");
lower_unary_float_intrinsic!(Exp2Op, "exp2");
lower_unary_float_intrinsic!(Exp10Op, "exp10");
lower_unary_float_intrinsic!(LogOp, "log");
lower_unary_float_intrinsic!(Log2Op, "log2");
lower_unary_float_intrinsic!(Log10Op, "log10");

// Rounding.
lower_unary_float_intrinsic!(FloorOp, "floor");
lower_unary_float_intrinsic!(CeilOp, "ceil");
lower_unary_float_intrinsic!(TruncOp, "trunc");
lower_unary_float_intrinsic!(RintOp, "rint");
lower_unary_float_intrinsic!(NearbyIntOp, "nearbyint");
lower_unary_float_intrinsic!(RoundOp, "round");
lower_unary_float_intrinsic!(RoundEvenOp, "roundeven");

// Misc.
lower_unary_float_intrinsic!(CanonicalizeOp, "canonicalize");

/// [DialectConversion] that lowers every [Op] in the LLVM intrinsics dialect
/// to its [`CallIntrinsicOp`] equivalent in the LLVM dialect.
pub struct LLVMIntrinsicsToLLVM;

impl DialectConversion for LLVMIntrinsicsToLLVM {
    fn can_convert_op(&self, ctx: &Context, op: Ptr<Operation>) -> bool {
        op_impls::<dyn ToLLVMDialect>(&*Operation::get_op_dyn(op, ctx))
    }

    fn rewrite(
        &mut self,
        ctx: &mut Context,
        rewriter: &mut DialectConversionRewriter,
        op: Ptr<Operation>,
        operands_info: &OperandsInfo,
    ) -> Result<()> {
        let op_dyn = Operation::get_op_dyn(op, ctx);
        let to_llvm_op = op_cast::<dyn ToLLVMDialect>(&*op_dyn)
            .expect("Matched Op must implement ToLLVMDialect");
        to_llvm_op.rewrite(ctx, rewriter, operands_info)
    }
}
