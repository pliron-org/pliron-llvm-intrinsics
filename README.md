# pliron-llvm-intrinsics

An [LLVM intrinsics](https://llvm.org/docs/LangRef.html#intrinsic-functions)
dialect for [pliron](https://github.com/pliron-org/pliron).

`pliron-llvm` uses a generic `llvm.call_intrinsic` `Op` for all intrinsics.
This repository provides strongly-typed specialized `Op`s for intrinsics.

`Op`s in this dialect lower (via `DialectConversion`) to `CallIntrinsicOp`
in the LLVM dialect.
