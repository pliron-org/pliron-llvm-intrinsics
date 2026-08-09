# pliron-llvm-intrinsics

An [LLVM intrinsics](https://llvm.org/docs/LangRef.html#intrinsic-functions)
dialect for [pliron](https://github.com/pliron-org/pliron).

Each LLVM intrinsic is represented by its own strongly-typed `Op`, as opposed
to pliron-llvm's generic `llvm.call_intrinsic`. Ops in this dialect lower (via
`DialectConversion`) to `CallIntrinsicOp` in the LLVM dialect.
