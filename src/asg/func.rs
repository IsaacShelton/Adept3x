use crate::{asg::*, name::ResolvedName, source_files::Source, tag::Tag};
use std::{collections::HashSet, fmt::Display};

#[derive(Clone, Debug)]
pub struct CurrentConstraints {
    constraints: HashMap<String, HashSet<Constraint>>,
}

impl<'a> CurrentConstraints {
    pub fn new(constraints: HashMap<String, HashSet<Constraint>>) -> Self {
        Self { constraints }
    }

    pub fn new_empty() -> Self {
        Self {
            constraints: Default::default(),
        }
    }

    pub fn satisfies(&self, ty: &Type, constraint: &Constraint) -> bool {
        match constraint {
            Constraint::PrimitiveAdd => match &ty.kind {
                TypeKind::Integer(..) | TypeKind::CInteger(..) | TypeKind::Floating(..) => true,
                TypeKind::Polymorph(name, constraints) => {
                    constraints.contains(constraint)
                        || self
                            .constraints
                            .get(name)
                            .map_or(false, |in_scope| in_scope.contains(constraint))
                }
                _ => false,
            },
            Constraint::Trait(name, _trait_ref, _trait_arguments) => match &ty.kind {
                TypeKind::Polymorph(name, constraints) => {
                    constraints.contains(constraint)
                        || self
                            .constraints
                            .get(name)
                            .map_or(false, |in_scope| in_scope.contains(constraint))
                }
                _ => {
                    todo!("test if user-defined trait '{}' is satisfied", name)
                }
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: ResolvedName,
    pub named_type_args: Vec<String>,
    pub params: Params,
    pub return_type: Type,
    pub stmts: Vec<Stmt>,
    pub is_foreign: bool,
    pub is_generic: bool,
    pub vars: VariableStorage,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub constraints: CurrentConstraints,
    pub impl_params: ImplParams,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Params {
    pub required: Vec<Param>,
    pub is_cstyle_vararg: bool,
}

impl Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, param) in self.required.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }

        if self.is_cstyle_vararg {
            if !self.required.is_empty() {
                write!(f, ", ")?;
            }

            write!(f, "...")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Hash, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

impl Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.ty)
    }
}

impl PartialEq for Param {
    fn eq(&self, other: &Self) -> bool {
        self.ty.eq(&other.ty)
    }
}
