// SPDX-License-Identifier: Apache-2.0
// Copyright (c) The pliron-llvm-intrinsics contributors

//! Convert the LLVM intrinsics dialect to the LLVM dialect.

use pliron::{
    builtin::{attributes::StringAttr, op_interfaces::OneOpdInterface},
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

/// Lower a unary float intrinsic [Op] to a [`CallIntrinsicOp`] calling `llvm.<base>`,
/// overloaded on the operand type.
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
                let arg = self.get_operand(ctx);
                let arg_ty = self.operand_type(ctx);

                let suffix = llvm_mangled_ty(ctx, arg_ty, self.loc(ctx))?;
                let intrinsic_name: StringAttr =
                    format!(concat!("llvm.", $base, ".{}"), suffix).into();

                let func_ty = FuncType::get(ctx, arg_ty, vec![arg_ty], false);

                let call_op = CallIntrinsicOp::new(ctx, intrinsic_name, func_ty, vec![arg]);
                rewriter.insert_op(ctx, &call_op);
                rewriter.replace_operation(ctx, self.get_operation(), call_op.get_operation());
                Ok(())
            }
        }
    };
}

lower_unary_float_intrinsic!(FAbsOp, "fabs");
lower_unary_float_intrinsic!(SqrtOp, "sqrt");

lower_unary_float_intrinsic!(SinOp, "sin");
lower_unary_float_intrinsic!(CosOp, "cos");
lower_unary_float_intrinsic!(TanOp, "tan");
lower_unary_float_intrinsic!(AsinOp, "asin");
lower_unary_float_intrinsic!(AcosOp, "acos");
lower_unary_float_intrinsic!(AtanOp, "atan");

lower_unary_float_intrinsic!(SinhOp, "sinh");
lower_unary_float_intrinsic!(CoshOp, "cosh");
lower_unary_float_intrinsic!(TanhOp, "tanh");

lower_unary_float_intrinsic!(ExpOp, "exp");
lower_unary_float_intrinsic!(Exp2Op, "exp2");
lower_unary_float_intrinsic!(Exp10Op, "exp10");
lower_unary_float_intrinsic!(LogOp, "log");
lower_unary_float_intrinsic!(Log2Op, "log2");
lower_unary_float_intrinsic!(Log10Op, "log10");

lower_unary_float_intrinsic!(FloorOp, "floor");
lower_unary_float_intrinsic!(CeilOp, "ceil");
lower_unary_float_intrinsic!(TruncOp, "trunc");
lower_unary_float_intrinsic!(RintOp, "rint");
lower_unary_float_intrinsic!(NearbyIntOp, "nearbyint");
lower_unary_float_intrinsic!(RoundOp, "round");
lower_unary_float_intrinsic!(RoundEvenOp, "roundeven");

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
