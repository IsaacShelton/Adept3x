use super::Type;
use crate::resolved;
use derive_more::IsVariant;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct PolyRecipe {
    pub polymorphs: IndexMap<String, PolyValue>,
}

#[derive(Clone, Debug)]
pub struct PolyType {
    pub resolved_type: Type,
}

#[derive(Clone, Debug)]
pub struct PolyExpr {
    expr: resolved::Expr,
}

#[derive(Clone, Debug, IsVariant)]
pub enum PolyValue {
    PolyType(PolyType),
    PolyExpr(PolyExpr),
}

#[derive(Clone, Debug)]
pub struct PolyCatalog {
    polymorphs: IndexMap<String, PolyValue>,
}

#[derive(Clone, Debug)]
pub enum PolyCatalogInsertError {
    /// Cannot have a single polymorph that is both a type as well as an expression
    Incongruent,
}

impl PolyCatalog {
    pub fn new() -> Self {
        Self {
            polymorphs: IndexMap::default(),
        }
    }

    pub fn bake(self) -> PolyRecipe {
        PolyRecipe {
            polymorphs: self.polymorphs,
        }
    }

    pub fn match_type(
        &mut self,
        pattern_type: &Type,
        concrete_type: &Type,
    ) -> Result<(), Option<PolyCatalogInsertError>> {
        match &pattern_type.kind {
            resolved::TypeKind::Unresolved => panic!(),
            resolved::TypeKind::Boolean
            | resolved::TypeKind::Integer(_, _)
            | resolved::TypeKind::CInteger(_, _)
            | resolved::TypeKind::IntegerLiteral(_)
            | resolved::TypeKind::FloatLiteral(_)
            | resolved::TypeKind::Floating(_)
            | resolved::TypeKind::Void
            | resolved::TypeKind::Enum(_, _)
            | resolved::TypeKind::Structure(_, _)
            | resolved::TypeKind::TypeAlias(_, _) => {
                if *pattern_type == *concrete_type {
                    Ok(())
                } else {
                    Err(None)
                }
            }
            resolved::TypeKind::Pointer(pattern_inner) => match &concrete_type.kind {
                resolved::TypeKind::Pointer(concrete_inner) => {
                    self.match_type(pattern_inner, concrete_inner)
                }
                _ => Err(None),
            },
            resolved::TypeKind::AnonymousStruct() => todo!(),
            resolved::TypeKind::AnonymousUnion() => todo!(),
            resolved::TypeKind::AnonymousEnum(_) => todo!(),
            resolved::TypeKind::FixedArray(pattern_inner) => match &concrete_type.kind {
                resolved::TypeKind::FixedArray(concrete_inner) => {
                    self.match_type(&pattern_inner.inner, &concrete_inner.inner)
                }
                _ => Err(None),
            },
            resolved::TypeKind::FunctionPointer(_) => todo!(),
            resolved::TypeKind::Polymorph(name, _constraints) => {
                self.put_type(name, concrete_type).map_err(Some)
            }
        }
    }

    pub fn put_type(&mut self, name: &str, new_type: &Type) -> Result<(), PolyCatalogInsertError> {
        if let Some(existing) = self.polymorphs.get_mut(name) {
            match existing {
                PolyValue::PolyType(poly_type) => {
                    if poly_type.resolved_type != *new_type {
                        return Err(PolyCatalogInsertError::Incongruent);
                    }
                }
                PolyValue::PolyExpr(_) => return Err(PolyCatalogInsertError::Incongruent),
            }
        } else {
            self.polymorphs.insert(
                name.to_string(),
                PolyValue::PolyType(PolyType {
                    resolved_type: new_type.clone(),
                }),
            );
        }

        Ok(())
    }

    pub fn get(&mut self, name: &str) -> Option<&PolyValue> {
        self.polymorphs.get(name)
    }
}
