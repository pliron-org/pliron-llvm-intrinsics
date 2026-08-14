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

use crate::{mangling::llvm_mangled_ty, ops::FAbsOp};

#[op_interface_impl]
impl ToLLVMDialect for FAbsOp {
    fn rewrite(
        &self,
        ctx: &mut Context,
        rewriter: &mut DialectConversionRewriter,
        _operands_info: &OperandsInfo,
    ) -> Result<()> {
        let arg = self.get_operand(ctx);
        let arg_ty = self.operand_type(ctx);

        let suffix = llvm_mangled_ty(ctx, arg_ty, self.loc(ctx))?;
        let intrinsic_name: StringAttr = format!("llvm.fabs.{suffix}").into();

        let func_ty = FuncType::get(ctx, arg_ty, vec![arg_ty], false);

        let call_op = CallIntrinsicOp::new(ctx, intrinsic_name, func_ty, vec![arg]);
        rewriter.insert_op(ctx, &call_op);
        rewriter.replace_operation(ctx, self.get_operation(), call_op.get_operation());
        Ok(())
    }
}

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
