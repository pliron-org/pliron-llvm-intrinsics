// SPDX-License-Identifier: Apache-2.0
// Copyright (c) The pliron-llvm-intrinsics contributors

//! LLVM intrinsics dialect for [pliron](https://github.com/pliron-org/pliron).
//!
//! Each LLVM intrinsic is represented by its own strongly-typed [Op](pliron::op::Op),
//! which lowers to [`CallIntrinsicOp`](pliron_llvm::ops::CallIntrinsicOp) in the LLVM
//! dialect.

pub mod mangling;
pub mod ops;
pub mod to_llvm;
pub(crate) mod utils;
