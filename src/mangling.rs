use pliron::printable::Printable;
use thiserror::Error;

use pliron::builtin::types::{FP16Type, FP32Type, FP64Type, IntegerType};
use pliron::context::Context;
use pliron::derive::{type_interface, type_interface_impl};
use pliron::location::Location;
use pliron::result::Result;
use pliron::r#type::{TypeHandle, type_cast};
use pliron::verify_err;
use pliron_llvm::types::VectorType;

#[type_interface]
pub trait LlvmTypeToMangledOverload {
    fn verify(
        _op: &dyn pliron::r#type::Type,
        _ctx: &pliron::context::Context,
    ) -> pliron::result::Result<()>
    where
        Self: Sized,
    {
        Ok(())
    }
    fn to_mangled_string(&self, ctx: &Context, loc: Location) -> Result<String>;
}

macro_rules! impl_llvm_type_to_mangled_overload {
    ($src:ty, $self:ident => $body:expr) => {
        #[type_interface_impl]
        impl LlvmTypeToMangledOverload for $src {
            fn to_mangled_string(&$self, ctx: &Context, loc: Location) -> Result<String> {
                Ok($body)
            }
        }
    };
    ($src:ty, $self:ident, $ctx:ident, $loc:ident => $body:expr) => {
        #[type_interface_impl]
        impl LlvmTypeToMangledOverload for $src {
            fn to_mangled_string(&$self, $ctx: &Context, $loc: Location) -> Result<String> {
                Ok($body)
            }
        }
    };
}

impl_llvm_type_to_mangled_overload!(IntegerType, self => format!("i{}", self.width()));
impl_llvm_type_to_mangled_overload!(FP16Type, self => "f16".to_string());
impl_llvm_type_to_mangled_overload!(FP32Type, self => "f32".to_string());
impl_llvm_type_to_mangled_overload!(FP64Type, self => "f64".to_string());
impl_llvm_type_to_mangled_overload!(VectorType, self, ctx, loc => {
    let prefix = if self.is_scalable() {
        "nx"
    } else {
        ""
    };
    let (n, elem) = (self.num_elements(), self.elem_type());
    format!("{prefix}v{n}{}", llvm_mangled_ty(ctx, elem, Location::Unknown)?)
});

#[derive(Error, Debug)]
#[error("Unsupported mangled type: {ty}")]
pub struct MangleTypeError {
    ty: String,
}

/// Convert a llvm type to the mangled string for intrinsic overload
pub fn llvm_mangled_ty(ctx: &Context, ty: TypeHandle, loc: Location) -> Result<String> {
    if let Some(ty) = type_cast::<dyn LlvmTypeToMangledOverload>(&*ty.deref(ctx)) {
        ty.to_mangled_string(ctx, loc)
    } else {
        verify_err!(
            loc,
            MangleTypeError {
                ty: ty.disp(ctx).to_string()
            }
        )
    }
}
