use super::error::{ResolveError, ResolveErrorKind};
use asg::{Destination, DestinationKind, ExprKind, Member, TypedExpr};

pub fn resolve_expr_to_destination(typed_expr: TypedExpr) -> Result<Destination, ResolveError> {
    let source = typed_expr.expr.source;

    Ok(Destination::new(
        match typed_expr.expr.kind {
            ExprKind::Variable(variable) => DestinationKind::Variable(*variable),
            ExprKind::GlobalVariable(global) => DestinationKind::GlobalVariable(*global),
            ExprKind::Member(member) => {
                let Member {
                    subject,
                    struct_ref,
                    index,
                    field_type,
                } = *member;

                DestinationKind::Member {
                    subject: Box::new(subject),
                    struct_ref,
                    index,
                    field_type,
                }
            }
            ExprKind::ArrayAccess(array_access) => DestinationKind::ArrayAccess(array_access),
            ExprKind::Dereference(subject) => DestinationKind::Dereference(subject.expr),
            _ => {
                return Err(ResolveErrorKind::CannotMutate {
                    bad_type: typed_expr.ty.to_string(),
                }
                .at(source));
            }
        },
        typed_expr.ty,
        typed_expr.expr.source,
    ))
}
