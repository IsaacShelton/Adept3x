use crate::{name::ResolvedName, resolved::*, source_files::Source, tag::Tag};
use std::{collections::HashSet, fmt::Display};

#[derive(Clone, Debug)]
pub struct CurrentConstraints<'a> {
    pub constraints: HashMap<String, HashSet<Constraint>>,
    pub implementations: &'a Implementations,
}

#[derive(Clone, Debug, Default)]
pub struct Implementations {
    targeting_trait: HashMap<TraitRef, ()>,
}

impl Implementations {
    pub fn new() -> Self {
        Self {
            targeting_trait: Default::default(),
        }
    }
}

impl<'a> CurrentConstraints<'a> {
    pub fn new_empty(implementations: &'a Implementations) -> Self {
        Self {
            constraints: Default::default(),
            implementations,
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
            Constraint::Trait(name, _trait_ref) => match &ty.kind {
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
pub struct Function<'a> {
    pub name: ResolvedName,
    pub parameters: Parameters,
    pub return_type: Type,
    pub stmts: Vec<Stmt>,
    pub is_foreign: bool,
    pub is_generic: bool,
    pub variables: VariableStorage,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub constraints: CurrentConstraints<'a>,
}

#[derive(Clone, Debug, Default)]
pub struct Parameters {
    pub required: Vec<Parameter>,
    pub is_cstyle_vararg: bool,
}

impl Display for Parameters {
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
pub struct Parameter {
    pub name: String,
    pub resolved_type: Type,
}

impl Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.resolved_type)
    }
}

impl PartialEq for Parameter {
    fn eq(&self, other: &Self) -> bool {
        self.resolved_type.eq(&other.resolved_type)
    }
}
