use crate::asg::{self};
use std::collections::HashSet;

pub fn collect_polymorphs(map: &mut HashSet<String>, ty: &asg::Type) {
    match &ty.kind {
        asg::TypeKind::Unresolved => panic!(),
        asg::TypeKind::Boolean
        | asg::TypeKind::Integer(_, _)
        | asg::TypeKind::CInteger(_, _)
        | asg::TypeKind::IntegerLiteral(_)
        | asg::TypeKind::FloatLiteral(_)
        | asg::TypeKind::Floating(_) => (),
        asg::TypeKind::Ptr(inner) => collect_polymorphs(map, inner.as_ref()),
        asg::TypeKind::Void => (),
        asg::TypeKind::Never => (),
        asg::TypeKind::AnonymousStruct() => todo!(),
        asg::TypeKind::AnonymousUnion() => todo!(),
        asg::TypeKind::AnonymousEnum(_) => (),
        asg::TypeKind::FixedArray(fixed_array) => collect_polymorphs(map, &fixed_array.inner),
        asg::TypeKind::FuncPtr(_) => todo!(),
        asg::TypeKind::Enum(_, _) => (),
        asg::TypeKind::Structure(_, _, params) | asg::TypeKind::TypeAlias(_, _, params) => {
            for parameter in params {
                collect_polymorphs(map, parameter);
            }
        }
        asg::TypeKind::Polymorph(name) => {
            if !map.contains(name) {
                map.insert(name.into());
            }
        }
        asg::TypeKind::Trait(_, _, parameters) => {
            for parameter in parameters {
                collect_polymorphs(map, parameter);
            }
        }
    }
}
