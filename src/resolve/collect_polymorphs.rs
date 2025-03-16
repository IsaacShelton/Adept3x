use crate::asg::{Type, TypeKind};
use indexmap::IndexSet;

pub fn collect_polymorphs(map: &mut IndexSet<String>, ty: &Type) {
    match &ty.kind {
        TypeKind::Unresolved => panic!(),
        TypeKind::Boolean
        | TypeKind::Integer(_, _)
        | TypeKind::CInteger(_, _)
        | TypeKind::SizeInteger(_)
        | TypeKind::IntegerLiteral(_)
        | TypeKind::FloatLiteral(_)
        | TypeKind::Floating(_) => (),
        TypeKind::Ptr(inner) => collect_polymorphs(map, inner.as_ref()),
        TypeKind::Void => (),
        TypeKind::Never => (),
        TypeKind::AnonymousStruct() => todo!(),
        TypeKind::AnonymousUnion() => todo!(),
        TypeKind::AnonymousEnum(_) => (),
        TypeKind::FixedArray(fixed_array) => collect_polymorphs(map, &fixed_array.inner),
        TypeKind::FuncPtr(_) => todo!(),
        TypeKind::Enum(_, _) => (),
        TypeKind::Structure(_, _, params) | TypeKind::TypeAlias(_, _, params) => {
            for parameter in params {
                collect_polymorphs(map, parameter);
            }
        }
        TypeKind::Polymorph(name) => {
            if !map.contains(name) {
                map.insert(name.into());
            }
        }
        TypeKind::Trait(_, _, parameters) => {
            for parameter in parameters {
                collect_polymorphs(map, parameter);
            }
        }
    }
}
