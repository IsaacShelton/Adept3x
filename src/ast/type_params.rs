use super::TypeArg;
use crate::source_files::Source;
use indexmap::IndexSet;

#[derive(Clone, Debug, Default)]
pub struct TypeParams {
    pub params: IndexSet<String>,
}

impl TypeParams {
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.params.iter()
    }

    pub fn len(&self) -> usize {
        self.params.len()
    }

    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }
}

impl From<IndexSet<String>> for TypeParams {
    fn from(params: IndexSet<String>) -> Self {
        Self { params }
    }
}

impl TryFrom<Vec<TypeArg>> for TypeParams {
    type Error = (String, Source);

    fn try_from(mut args: Vec<TypeArg>) -> Result<Self, Self::Error> {
        let mut params = IndexSet::<String>::new();

        for arg in args.drain(..) {
            match arg {
                TypeArg::Type(ty) => match ty.kind {
                    super::TypeKind::Polymorph(name) => {
                        params.insert(name);
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
