use crate::error::UnaliasError;
use asg::{Asg, IntoPolyRecipeResolver, PolyRecipe, PolyValue, Type, TypeKind};
use indexmap::IndexMap;
use std::{borrow::Cow, collections::HashSet};

const MAX_UNALIAS_DEPTH: usize = 1024;

pub fn unalias<'a>(asg: &'a Asg<'a>, whole_type: &'a Type) -> Result<Cow<'a, Type>, UnaliasError> {
    let mut running = Cow::Borrowed(whole_type);
    let mut depth = 0;

    while let TypeKind::TypeAlias(human_name, type_alias_ref, type_args) = &running.kind {
        let alias = asg
            .type_aliases
            .get(*type_alias_ref)
            .expect("valid type alias ref");

        if type_args.len() != alias.params.len() {
            return Err(UnaliasError::IncorrectNumberOfTypeArgsForAlias(
                human_name.0.clone(),
            ));
        }

        if alias.params.is_empty() {
            running = Cow::Borrowed(&alias.becomes);
        } else {
            let polymorphs = IndexMap::<String, PolyValue>::from_iter(
                alias
                    .params
                    .names()
                    .cloned()
                    .zip(type_args.iter().cloned().map(PolyValue::Type)),
            );

            let recipe = PolyRecipe { polymorphs };

            running = Cow::Owned(
                recipe
                    .resolver()
                    .resolve_type(&alias.becomes)
                    .map_err(|e| UnaliasError::PolymorphError(e.kind))?,
            )
        }

        depth += 1;

        if depth > MAX_UNALIAS_DEPTH {
            return Err(find_type_alias_self_reference(asg, whole_type));
        }
    }

    Ok(running)
}

fn find_type_alias_self_reference(asg: &Asg, whole_type: &Type) -> UnaliasError {
    let mut seen = HashSet::new();
    let mut running = whole_type;
    let mut depth = 0;

    while let TypeKind::TypeAlias(human_name, type_alias_ref, type_args) = &running.kind {
        let alias = asg
            .type_aliases
            .get(*type_alias_ref)
            .expect("valid type alias ref");

        if !type_args.is_empty() || !alias.params.is_empty() {
            todo!("unalias type alias with type args");
        }

        running = &alias.becomes;

        if !seen.insert(type_alias_ref) {
            return UnaliasError::SelfReferentialTypeAlias(human_name.0.clone());
        }

        depth += 1;

        if depth > MAX_UNALIAS_DEPTH {
            break;
        }
    }

    return UnaliasError::MaxDepthExceeded;
}
