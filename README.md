# pliron-llvm-intrinsics

An [LLVM intrinsics](https://llvm.org/docs/LangRef.html#intrinsic-functions)
dialect for [pliron](https://github.com/pliron-org/pliron).

`pliron-llvm` uses a generic `llvm.call_intrinsic` `Op` for all intrinsics.
This repository provides strongly-typed specialized `Op`s for intrinsics.

`Op`s in this dialect lower (via `DialectConversion`) to `CallIntrinsicOp`
in the LLVM dialect.

## Supported intrinsics

Unary intrinsics taking and returning a floating point type (or a vector
thereof). Each is overloaded on its operand type, so `llvm_intrinsics.sqrt`
on `builtin.fp32` lowers to `llvm.sqrt.f32`, and on a
`llvm.vector <Fixed x 2 x builtin.fp64>` to `llvm.sqrt.v2f64`.

| category | intrinsics |
|-----|-------|
| general | `fabs`, `sqrt`, `canonicalize` |
| trigonometric | `sin`, `cos`, `tan`, `asin`, `acos`, `atan` |
| hyperbolic | `sinh`, `cosh`, `tanh` |
| exponential | `exp`, `exp2`, `exp10` |
| logarithmic | `log`, `log2`, `log10` |
| rounding | `floor`, `ceil`, `trunc`, `rint`, `nearbyint`, `round`, `roundeven` |

Note that `llvm.tan`, the inverse trigonometric and the hyperbolic intrinsics
require LLVM 19 or later, and `llvm.exp10` requires LLVM 18 or later.
