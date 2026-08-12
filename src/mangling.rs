use pliron::arg_err;
use pliron::builtin::types::{FP16Type, FP32Type, FP64Type, IntegerType};
use pliron::context::Context;
use pliron::derive::{type_interface, type_interface_impl};
use pliron::location::Location;
use pliron::printable::Printable;
use pliron::result::Result;
use pliron::r#type::{Type, TypeHandle, type_cast};
use pliron_llvm::types::VectorType;
use thiserror::Error;

#[type_interface]
pub trait LlvmTypeToMangledOverload {
    fn verify(_op: &dyn Type, _ctx: &Context) -> Result<()>
    where
        Self: Sized,
    {
        Ok(())
    }
    fn to_mangled_string(&self, ctx: &Context, loc: Location) -> Result<String>;
}

macro_rules! impl_llvm_type_to_mangled_overload {
    ($src:ty, $self:ident => $text:expr) => {
        #[type_interface_impl]
        impl LlvmTypeToMangledOverload for $src {
            fn to_mangled_string(&$self, _ctx: &Context, _loc: Location) -> Result<String> {
                Ok($text.to_string())
            }
        }
    };
}

#[type_interface_impl]
impl LlvmTypeToMangledOverload for IntegerType {
    fn to_mangled_string(&self, _ctx: &Context, _loc: Location) -> Result<String> {
        Ok(format!("i{}", self.width()))
    }
}

impl_llvm_type_to_mangled_overload!(FP16Type, self => "f16");
impl_llvm_type_to_mangled_overload!(FP32Type, self => "f32");
impl_llvm_type_to_mangled_overload!(FP64Type, self => "f64");

#[type_interface_impl]
impl LlvmTypeToMangledOverload for VectorType {
    fn to_mangled_string(&self, ctx: &Context, loc: Location) -> Result<String> {
        let prefix = if self.is_scalable() { "nx" } else { "" };
        let (n, elem) = (self.num_elements(), self.elem_type());
        Ok(format!("{prefix}v{n}{}", llvm_mangled_ty(ctx, elem, loc)?))
    }
}

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
        let ty = ty.disp(ctx).to_string();
        let error = MangleTypeError { ty };
        arg_err!(loc, error)
    }
}
