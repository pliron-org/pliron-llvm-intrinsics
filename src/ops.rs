// SPDX-License-Identifier: Apache-2.0
// Copyright (c) The pliron-llvm-intrinsics contributors

//! Ops in the LLVM intrinsics dialect.

use pliron::{
    builtin::op_interfaces::{
        NOpdsInterface, NResultsInterface, OneOpdInterface, OneResultInterface,
        SameOperandsAndResultType, SameOperandsType, SameResultsType,
    },
    common_traits::Verify,
    context::Context,
    derive::pliron_op,
    op::Op,
    operation::Operation,
    result::Result,
    r#type::Typed,
    value::Value,
    verify_err,
};
use thiserror::Error;

use crate::utils::is_float_or_vector_of_float;

/// Absolute value of a floating point operand.
/// Equivalent to LLVM's [`llvm.fabs`](https://llvm.org/docs/LangRef.html#llvm-fabs-intrinsic) intrinsic.
///
/// ## Operand(s):
///
/// | operand | description |
/// |-----|-------|
/// | `arg` | floating point type |
///
/// ## Result(s):
///
/// | result | description |
/// |-----|-------|
/// | `res` | floating point type, same as `arg` |
#[pliron_op(
    name = "llvm_intrinsics.fabs",
    interfaces = [
        NOpdsInterface<1>,
        OneOpdInterface,
        NResultsInterface<1>,
        OneResultInterface,
        SameOperandsType,
        SameResultsType,
        SameOperandsAndResultType,
    ],
    format = "$0 ` : ` type($0)",
)]
pub struct FAbsOp;

impl FAbsOp {
    /// Create a new [FAbsOp].
    pub fn new(ctx: &mut Context, arg: Value) -> Self {
        let res_ty = arg.get_type(ctx);
        let op = Operation::new(
            ctx,
            Self::get_concrete_op_info(),
            vec![res_ty],
            vec![arg],
            vec![],
            0,
        );
        Self { op }
    }
}

#[derive(Error, Debug)]
pub enum FAbsOpVerifyErr {
    #[error("Operand must be a floating point type")]
    OperandNotFloat,
}

impl Verify for FAbsOp {
    fn verify(&self, ctx: &Context) -> Result<()> {
        let opd_ty = self.operand_type(ctx);
        if !is_float_or_vector_of_float(opd_ty, ctx) {
            return verify_err!(self.loc(ctx), FAbsOpVerifyErr::OperandNotFloat);
        }
        Ok(())
    }
}
