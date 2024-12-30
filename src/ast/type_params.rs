use super::{Type, TypeArg};
use crate::source_files::Source;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct TypeParam {
    pub constraints: Vec<Type>,
}

impl TypeParam {
    pub fn new(constraints: Vec<Type>) -> Self {
        Self { constraints }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TypeParams {
    pub params: IndexMap<String, TypeParam>,
}

impl TypeParams {
    pub fn iter(&self) -> impl Iterator<Item = (&String, &TypeParam)> {
        self.params.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.params.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &TypeParam> {
        self.params.values()
    }
}

impl From<IndexMap<String, TypeParam>> for TypeParams {
    fn from(params: IndexMap<String, TypeParam>) -> Self {
        Self { params }
    }
}

impl TryFrom<Vec<TypeArg>> for TypeParams {
    type Error = (String, Source);

    fn try_from(mut args: Vec<TypeArg>) -> Result<Self, Self::Error> {
        let mut params = IndexMap::<String, TypeParam>::new();

        for arg in args.drain(..) {
            match arg {
                TypeArg::Type(ty) => match ty.kind {
                    super::TypeKind::Polymorph(name, constraints) => {
                        if let Some(existing) = params.get_mut(&name) {
                            existing.constraints.extend(constraints);
                        } else {
                            params.insert(name, TypeParam { constraints });
                        }
                    }
                    _ => {
                        return Err((
                            "Cannot use non-polymorph as type parameter".into(),
                            ty.source,
                        ))
                    }
                },
                TypeArg::Expr(expr) => {
                    return Err((
                        "Cannot use expression as type parameter".into(),
                        expr.source,
                    ))
                }
            }
        }

        Ok(params.into())
    }
}
