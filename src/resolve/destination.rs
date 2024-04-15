use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    resolved::{Destination, DestinationKind, ExprKind, Type, TypedExpr},
    source_file_cache::SourceFileCache,
};

pub fn resolve_expr_to_destination(
    source_file_cache: &SourceFileCache,
    typed_expr: TypedExpr,
) -> Result<Destination, ResolveError> {
    let source = typed_expr.expr.source;

    Ok(Destination::new(
        match typed_expr.expr.kind {
            ExprKind::Variable(variable) => DestinationKind::Variable(variable),
            ExprKind::GlobalVariable(global) => DestinationKind::GlobalVariable(global),
            ExprKind::Member {
                subject,
                structure_ref,
                index,
                field_type,
                memory_management,
            } => {
                match subject.resolved_type {
                    Type::PlainOldData(..) => (),
                    Type::Unsync(..) => (),
                    Type::ManagedStructure(..) => {
                        return Err(ResolveError::new(
                            source_file_cache,
                            source,
                            ResolveErrorKind::CannotMutateNonUnsync {
                                bad_type: subject.resolved_type.to_string(),
                            },
                        ))
                    }
                    _ => {
                        return Err(ResolveError::new(
                            source_file_cache,
                            source,
                            ResolveErrorKind::CannotMutate {
                                bad_type: subject.resolved_type.to_string(),
                            },
                        ))
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
                return Err(ResolveError::new(
                    source_file_cache,
                    source,
                    ResolveErrorKind::CannotMutate {
                        bad_type: typed_expr.resolved_type.to_string(),
                    },
                ))
            }
        },
        typed_expr.resolved_type,
        typed_expr.expr.source,
    ))
}
