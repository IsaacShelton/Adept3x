use crate::{
    module_graph::ModulePartHandle,
    repr::{DeclHead, Type},
};

#[derive(Debug)]
pub struct Link<'env> {
    pub name: &'env str,
    pub handle: ModulePartHandle<'env>,
    pub constraints: LookupConstraints<'env>,
    pub binding: DeclHead<'env>,
}

impl<'env> Link<'env> {
    pub fn new(
        name: &'env str,
        handle: ModulePartHandle<'env>,
        constraints: LookupConstraints<'env>,
        binding: DeclHead<'env>,
    ) -> Self {
        Self {
            name,
            handle,
            binding,
            constraints,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LookupConstraints<'env> {
    TypeLike(TypeLikeLookupConstraints),
    FuncLike(FuncLikeLookupConstraints<'env>),
    ValueLike,
}

impl<'env> LookupConstraints<'env> {
    pub fn is_match(&self, head: &DeclHead<'env>) -> bool {
        match self {
            LookupConstraints::TypeLike(constraints) => constraints.is_match(head),
            LookupConstraints::FuncLike(constraints) => constraints.is_match(head),
            LookupConstraints::ValueLike => match head {
                DeclHead::FuncLike(..) | DeclHead::TypeLike(..) => false,
                DeclHead::ValueLike(_) => true,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeLikeLookupConstraints {
    arity: usize,
}

impl TypeLikeLookupConstraints {
    pub fn is_match<'env>(&self, head: &DeclHead<'env>) -> bool {
        match head {
            DeclHead::FuncLike(..) | DeclHead::ValueLike(..) => false,
            DeclHead::TypeLike(type_head) => self.arity == type_head.arity,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FuncLikeLookupConstraints<'env> {
    unaliased_arg_types: &'env [&'env Type<'env>],
}

impl<'env> FuncLikeLookupConstraints<'env> {
    pub fn is_match(&self, head: &DeclHead<'env>) -> bool {
        match head {
            DeclHead::TypeLike(..) | DeclHead::ValueLike(..) => false,
            DeclHead::FuncLike(func_head) => {
                // We will check that min arity matches, and types are compatible
                // User-defined implicit conversions will probably not be allowed?
                // They should use traits instead...
                todo!("does function match?")
            }
        }
    }
}
