use crate::asg::{self, Constraint};
use std::collections::{HashMap, HashSet};

pub fn collect_constraints(
    parameters: &asg::Params,
    return_type: &asg::Type,
) -> HashMap<String, HashSet<Constraint>> {
    let mut map = HashMap::default();

    for param in parameters.required.iter() {
        collect_constraints_into(&mut map, &param.ty);
    }

    collect_constraints_into(&mut map, &return_type);
    map
}

pub fn collect_constraints_into(map: &mut HashMap<String, HashSet<Constraint>>, ty: &asg::Type) {
    match &ty.kind {
        asg::TypeKind::Unresolved => panic!(),
        asg::TypeKind::Boolean
        | asg::TypeKind::Integer(_, _)
        | asg::TypeKind::CInteger(_, _)
        | asg::TypeKind::IntegerLiteral(_)
        | asg::TypeKind::FloatLiteral(_)
        | asg::TypeKind::Floating(_) => (),
        asg::TypeKind::Ptr(inner) => collect_constraints_into(map, inner.as_ref()),
        asg::TypeKind::Void => (),
        asg::TypeKind::Never => (),
        asg::TypeKind::AnonymousStruct() => todo!(),
        asg::TypeKind::AnonymousUnion() => todo!(),
        asg::TypeKind::AnonymousEnum() => todo!(),
        asg::TypeKind::FixedArray(fixed_array) => collect_constraints_into(map, &fixed_array.inner),
        asg::TypeKind::FuncPtr(_) => todo!(),
        asg::TypeKind::Enum(_, _) => (),
        asg::TypeKind::Structure(_, _, parameters) => {
            for parameter in parameters {
                collect_constraints_into(map, parameter);
            }
        }
        asg::TypeKind::TypeAlias(_, _) => (),
        asg::TypeKind::Polymorph(name, constraints) => {
            let set = map.entry(name.to_string()).or_default();
            for constraint in constraints {
                set.insert(constraint.clone());
            }
        }
        asg::TypeKind::Trait(_, _, parameters) => {
            for parameter in parameters {
                collect_constraints_into(map, parameter);
            }
        }
    }
}
