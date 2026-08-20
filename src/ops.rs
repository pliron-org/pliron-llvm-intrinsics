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

#[derive(Error, Debug)]
pub enum UnaryFloatIntrinsicVerifyErr {
    #[error("Operand of {0} must be a floating point type")]
    OperandNotFloat(&'static str),
}

/// Define an [Op] for a unary LLVM intrinsic whose operand and result are both
/// a floating point type (or a vector thereof).
///
/// Every such intrinsic has the same shape - `<fptype> @llvm.<base>(<fptype>)` - so
/// they share a single set of interfaces, constructor and verifier. Intrinsics that
/// deviate (extra `immarg` operands, integer operands or results, aggregate results)
/// are not covered by this macro and need their own definitions.
///
/// `rustfmt` re-indents multi-line attribute arguments inside a `macro_rules!` body on
/// every run without ever reaching a fixed point, so this definition is skipped.
#[rustfmt::skip]
macro_rules! unary_float_intrinsic {
    ($op:ident, $op_name:literal, $base:literal, $desc:literal) => {
        #[doc = $desc]
        ///
        /// Operates on a floating point value, or a vector of floating point values.
        ///
        #[doc = concat!(
            "Equivalent to LLVM's [`llvm.", $base,
            "`](https://llvm.org/docs/LangRef.html#llvm-", $base,
            "-intrinsic) intrinsic."
        )]
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
            name = $op_name,
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
        pub struct $op;

        impl $op {
            #[doc = concat!("Create a new [", stringify!($op), "].")]
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

        impl Verify for $op {
            fn verify(&self, ctx: &Context) -> Result<()> {
                let opd_ty = self.operand_type(ctx);
                if !is_float_or_vector_of_float(opd_ty, ctx) {
                    return verify_err!(
                        self.loc(ctx),
                        UnaryFloatIntrinsicVerifyErr::OperandNotFloat($op_name)
                    );
                }
                Ok(())
            }
        }
    };
}

unary_float_intrinsic!(FAbsOp, "llvm_intrinsics.fabs", "fabs", "Absolute value.");
unary_float_intrinsic!(SqrtOp, "llvm_intrinsics.sqrt", "sqrt", "Square root.");

// Trigonometric.
unary_float_intrinsic!(SinOp, "llvm_intrinsics.sin", "sin", "Sine.");
unary_float_intrinsic!(CosOp, "llvm_intrinsics.cos", "cos", "Cosine.");
unary_float_intrinsic!(TanOp, "llvm_intrinsics.tan", "tan", "Tangent.");
unary_float_intrinsic!(AsinOp, "llvm_intrinsics.asin", "asin", "Arcsine.");
unary_float_intrinsic!(AcosOp, "llvm_intrinsics.acos", "acos", "Arccosine.");
unary_float_intrinsic!(AtanOp, "llvm_intrinsics.atan", "atan", "Arctangent.");

// Hyperbolic.
unary_float_intrinsic!(SinhOp, "llvm_intrinsics.sinh", "sinh", "Hyperbolic sine.");
unary_float_intrinsic!(CoshOp, "llvm_intrinsics.cosh", "cosh", "Hyperbolic cosine.");
unary_float_intrinsic!(
    TanhOp,
    "llvm_intrinsics.tanh",
    "tanh",
    "Hyperbolic tangent."
);

// Exponential and logarithmic.
unary_float_intrinsic!(ExpOp, "llvm_intrinsics.exp", "exp", "Base-e exponential.");
unary_float_intrinsic!(
    Exp2Op,
    "llvm_intrinsics.exp2",
    "exp2",
    "Base-2 exponential."
);
unary_float_intrinsic!(
    Exp10Op,
    "llvm_intrinsics.exp10",
    "exp10",
    "Base-10 exponential."
);
unary_float_intrinsic!(LogOp, "llvm_intrinsics.log", "log", "Base-e logarithm.");
unary_float_intrinsic!(Log2Op, "llvm_intrinsics.log2", "log2", "Base-2 logarithm.");
unary_float_intrinsic!(
    Log10Op,
    "llvm_intrinsics.log10",
    "log10",
    "Base-10 logarithm."
);

// Rounding.
unary_float_intrinsic!(
    FloorOp,
    "llvm_intrinsics.floor",
    "floor",
    "Round towards negative infinity."
);
unary_float_intrinsic!(
    CeilOp,
    "llvm_intrinsics.ceil",
    "ceil",
    "Round towards positive infinity."
);
unary_float_intrinsic!(
    TruncOp,
    "llvm_intrinsics.trunc",
    "trunc",
    "Round towards zero."
);
unary_float_intrinsic!(
    RintOp,
    "llvm_intrinsics.rint",
    "rint",
    "Round to an integer using the current rounding mode, raising `inexact`."
);
unary_float_intrinsic!(
    NearbyIntOp,
    "llvm_intrinsics.nearbyint",
    "nearbyint",
    "Round to an integer using the current rounding mode, without raising `inexact`."
);
unary_float_intrinsic!(
    RoundOp,
    "llvm_intrinsics.round",
    "round",
    "Round to the nearest integer, with ties away from zero."
);
unary_float_intrinsic!(
    RoundEvenOp,
    "llvm_intrinsics.roundeven",
    "roundeven",
    "Round to the nearest integer, with ties to even."
);

// Misc.
unary_float_intrinsic!(
    CanonicalizeOp,
    "llvm_intrinsics.canonicalize",
    "canonicalize",
    "Canonical IEEE-754 representation."
);
