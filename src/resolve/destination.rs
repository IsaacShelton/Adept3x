use super::error::{ResolveError, ResolveErrorKind};
use crate::resolved::{Destination, DestinationKind, ExprKind, Member, TypeKind, TypedExpr};

pub fn resolve_expr_to_destination(typed_expr: TypedExpr) -> Result<Destination, ResolveError> {
    let source = typed_expr.expr.source;

    Ok(Destination::new(
        match typed_expr.expr.kind {
            ExprKind::Variable(variable) => DestinationKind::Variable(*variable),
            ExprKind::GlobalVariable(global) => DestinationKind::GlobalVariable(*global),
            ExprKind::Member(member) => {
                let Member {
                    subject,
                    structure_ref,
                    index,
                    field_type,
                    memory_management,
                } = *member;

                match &subject.resolved_type.kind {
                    TypeKind::PlainOldData(..) => (),
                    TypeKind::ManagedStructure(..) => (),
                    _ => {
                        return Err(ResolveErrorKind::CannotMutate {
                            bad_type: subject.resolved_type.to_string(),
                        }
                        .at(source))
                    }
                }

                DestinationKind::Member {
                    subject: Box::new(subject),
                    structure_ref,
                    index,
                    field_type,
                    memory_management,
                }
            }
            ExprKind::ArrayAccess(array_access) => DestinationKind::ArrayAccess(array_access),
            _ => {
                return Err(ResolveErrorKind::CannotMutate {
                    bad_type: typed_expr.resolved_type.to_string(),
                }
                .at(source))
            }
        },
        typed_expr.resolved_type,
        typed_expr.expr.source,
    ))
}
