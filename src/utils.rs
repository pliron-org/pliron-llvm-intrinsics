use pliron::{
    builtin::type_interfaces::FloatTypeInterface,
    context::Context,
    r#type::{TypeHandle, type_impls},
};
use pliron_llvm::types::VectorType;

pub fn is_float_or_vector_of_float(ty: TypeHandle, ctx: &Context) -> bool {
    let mut ty = ty.deref(ctx);
    if let Some(vec_ty) = ty.downcast_ref::<VectorType>() {
        ty = vec_ty.elem_type().deref(ctx);
    }
    type_impls::<dyn FloatTypeInterface>(&*ty)
}
