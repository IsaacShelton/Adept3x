use super::{depleted::Depleted, Environment};
use crate::{
    c::{
        encoding::Encoding,
        preprocessor::{
            ast::{Define, DefineKind, FunctionMacro, PlaceholderAffinity},
            error::PreprocessorErrorKind,
            pre_token::{PreToken, PreTokenKind, Punctuator},
            ParseErrorKind, PreprocessorError,
        },
    },
    look_ahead::LookAhead,
    source_files::Source,
};
use itertools::Itertools;

pub fn expand_region(
    pre_tokens: &[PreToken],
    environment: &Environment,
    depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let mut expanded = Vec::with_capacity(pre_tokens.len() + 16);
    let mut tokens = LookAhead::new(pre_tokens.iter());

    while let Some(token) = tokens.next() {
        expand_token(token, &mut tokens, environment, depleted, &mut expanded)?;
    }

    Ok(expanded)
}

pub fn expand_region_allow_concats(
    pre_tokens: &[PreToken],
    environment: &Environment,
    depleted: &mut Depleted,
    strip_placeholders: bool,
    start_of_macro_call: Source,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let mut expanded = expand_region(pre_tokens, environment, depleted)?;
    resolve_concats(expanded.drain(..), strip_placeholders, start_of_macro_call)
}

fn expand_token<'a>(
    token: &PreToken,
    tokens: &mut LookAhead<impl Iterator<Item = &'a PreToken>>,
    environment: &Environment,
    depleted: &mut Depleted,
    expanded: &mut Vec<PreToken>,
) -> Result<(), PreprocessorError> {
    match &token.kind {
        PreTokenKind::Identifier(name) => {
            if let Some(define) = environment.find_define(name) {
                let start_of_macro = token.source;
                let hash = Depleted::hash_define(define);

                if !depleted.contains(hash) {
                    let (replacement, placeholder_affinity) = match &define.kind {
                        DefineKind::ObjectMacro(replacement, affinity) => (replacement, affinity),
                        DefineKind::FunctionMacro(function_macro) => (
                            &expand_function_macro(
                                token,
                                tokens,
                                function_macro,
                                environment,
                                depleted,
                            )?,
                            &function_macro.affinity,
                        ),
                    };

                    // Expand the replacement in the context of the current environment
                    depleted.push(hash);
                    expanded.append(&mut expand_region_allow_concats(
                        replacement,
                        environment,
                        depleted,
                        placeholder_affinity.is_discard(),
                        start_of_macro,
                    )?);
                    depleted.pop(hash);

                    // Process any function-macro invocations that span between expanded function-macro
                    // results and upcoming tokens
                    while let (
                        Some(PreToken {
                            kind: PreTokenKind::Identifier(name),
                            ..
                        }),
                        Some(PreToken {
                            kind: PreTokenKind::Punctuator(Punctuator::OpenParen { .. }),
                            ..
                        }),
                    ) = (expanded.last(), tokens.peek())
                    {
                        let nested = environment.find_define(name);
                        let hash = Depleted::hash_define(define);

                        match nested {
                            Some(Define {
                                kind: DefineKind::FunctionMacro(function_macro),
                                ..
                            }) if !depleted.contains(hash) => {
                                let replacement = &expand_function_macro(
                                    &expanded.pop().unwrap(),
                                    tokens,
                                    function_macro,
                                    environment,
                                    depleted,
                                )?;

                                depleted.push(hash);
                                expanded.append(&mut expand_region_allow_concats(
                                    replacement,
                                    environment,
                                    depleted,
                                    function_macro.affinity.is_discard(),
                                    start_of_macro,
                                )?);
                                depleted.pop(hash);
                            }
                            _ => break,
                        }
                    }

                    // Macro invocation was successful
                    return Ok(());
                }
            }

            // Otherwise, just a normal identifier
            expanded.push(token.clone());
            Ok(())
        }
        PreTokenKind::IsDefined(name) => {
            expanded.push(
                PreTokenKind::Number(if environment.find_define(name).is_some() {
                    "1".into()
                } else {
                    "0".into()
                })
                .at(token.source),
            );
            Ok(())
        }
        PreTokenKind::HeaderName(_)
        | PreTokenKind::Number(_)
        | PreTokenKind::CharacterConstant(_, _)
        | PreTokenKind::StringLiteral(_, _)
        | PreTokenKind::Punctuator(_)
        | PreTokenKind::ProtectedIdentifier(_)
        | PreTokenKind::UniversalCharacterName(_)
        | PreTokenKind::Other(_)
        | PreTokenKind::Placeholder => {
            expanded.push(token.clone());
            Ok(())
        }
        PreTokenKind::EndOfSequence => unreachable!(),
    }
}

fn expand_function_macro<'a>(
    token: &PreToken,
    tokens: &mut LookAhead<impl Iterator<Item = &'a PreToken>>,
    function_macro: &FunctionMacro,
    parent_environment: &Environment,
    parent_depleted: &mut Depleted,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let start_of_macro_call = token.source;

    // Eat '('
    match tokens.next() {
        Some(PreToken {
            kind: PreTokenKind::Punctuator(Punctuator::OpenParen { .. }),
            ..
        }) => (),
        _ => {
            // Not invoking the macro, just using the name
            return Ok(vec![token.clone()]);
        }
    }

    // Parse function-macro arguments
    let mut args = Vec::<Vec<PreToken>>::with_capacity(4);
    let mut paren_depth = 0;

    loop {
        let part = match tokens.next() {
            Some(
                token @ PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::CloseParen),
                    ..
                },
            ) => {
                if paren_depth == 0 {
                    break;
                }

                paren_depth -= 1;
                Some(token.clone())
            }
            Some(
                token @ PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::OpenParen { .. }),
                    ..
                },
            ) => {
                paren_depth += 1;
                Some(token.clone())
            }
            Some(PreToken {
                kind: PreTokenKind::Punctuator(Punctuator::Comma),
                ..
            }) if paren_depth == 0 => {
                if args.is_empty() {
                    args.push(Vec::new());
                }
                args.push(Vec::new());
                None
            }
            Some(token) => Some(token.clone()),
            None => {
                return Err(
                    PreprocessorErrorKind::ParseError(ParseErrorKind::ExpectedCloseParen)
                        .at(token.source),
                )
            }
        };

        if let Some(part) = part {
            // Append argument part to current argument
            if args.is_empty() {
                args.push(Vec::new());
            }

            args.last_mut()
                .expect("at least one function-macro argument has been created")
                .push(part);
        }
    }

    // Allow "zero arguments" when calling function-macros that only take a single argument.
    if args.is_empty() && function_macro.parameters.len() == 1 {
        args.push(vec![]);
    }

    // Validate number of arguments
    if args.len() != function_macro.parameters.len()
        && !(args.len() > function_macro.parameters.len() && function_macro.is_variadic)
    {
        return Err(PreprocessorErrorKind::ParseError(
            if !function_macro.is_variadic && args.len() > function_macro.parameters.len() {
                ParseErrorKind::TooManyArguments
            } else {
                ParseErrorKind::NotEnoughArguments
            },
        )
        .at(token.source));
    }

    // Inject stringized arguments
    let body = inject_stringized_arguments(function_macro, &args, start_of_macro_call);

    // Expand the values for each argument
    for arg in args.iter_mut() {
        *arg = expand_region(arg, parent_environment, parent_depleted)?;
    }

    // Create environment to replace parameters with specified argument values
    let mut args_only_environment = Environment::default();

    for (i, parameter_name) in function_macro.parameters.iter().enumerate() {
        // Replace all empty arg values with placeholder token
        if args[i].is_empty() {
            args[i].push(PreTokenKind::Placeholder.at(start_of_macro_call));
        }

        args_only_environment.add_define(Define {
            kind: DefineKind::ObjectMacro(std::mem::take(&mut args[i]), PlaceholderAffinity::Keep),
            name: parameter_name.clone(),
            source: Source::internal(),
            is_file_local_only: false,
        });
    }

    // Create __VA__ARGS__ definition if applicable
    if args.len() > function_macro.parameters.len() {
        #[allow(unstable_name_collisions)]
        let rest = args
            .splice(
                function_macro.parameters.len()..args.len(),
                std::iter::empty(),
            )
            .intersperse(vec![
                // NOTE: The location information of inserted comma preprocessor tokens will be
                // missing, but an error message caused by them is extremely rare in
                // practice so doesn't really matter
                // TODO: Remember location information of each comma preprocessor token that needs
                // to be inserted
                PreTokenKind::Punctuator(Punctuator::Comma).at(start_of_macro_call),
            ])
            .flatten()
            .collect_vec();

        // Expand value that will be used for __VA_ARGS__
        let mut rest = expand_region(&rest, parent_environment, parent_depleted)?;

        // Replace __VA_ARGS__ with placeholder token if empty
        if rest.is_empty() {
            rest.push(PreTokenKind::Placeholder.at(start_of_macro_call));
        }

        args_only_environment.add_define(Define {
            kind: DefineKind::ObjectMacro(rest, PlaceholderAffinity::Keep),
            name: "__VA_ARGS__".into(),
            source: Source::internal(),
            is_file_local_only: false,
        });

        // Add `#define __VA_OPT__(...) __VA_ARGS__` to local environment
        args_only_environment.add_define(Define {
            kind: DefineKind::FunctionMacro(FunctionMacro {
                affinity: PlaceholderAffinity::Keep,
                parameters: vec![],
                is_variadic: true,
                body: vec![PreTokenKind::Identifier("__VA_ARGS__".into()).at(start_of_macro_call)],
            }),
            name: "__VA_OPT__".into(),
            source: Source::internal(),
            is_file_local_only: false,
        });
    } else if function_macro.is_variadic {
        // No variadic arguments passed, despite this function-macro
        // being variadic, so we must define __VA_ARGS__ to be empty (a placeholder token).
        args_only_environment.add_define(Define {
            kind: DefineKind::ObjectMacro(
                vec![PreTokenKind::Placeholder.at(start_of_macro_call)],
                PlaceholderAffinity::Keep,
            ),
            name: "__VA_ARGS__".into(),
            source: Source::internal(),
            is_file_local_only: false,
        });

        // Add `#define __VA_OPT__(...)` to local environment
        args_only_environment.add_define(Define {
            kind: DefineKind::FunctionMacro(FunctionMacro {
                affinity: PlaceholderAffinity::Keep,
                parameters: vec![],
                is_variadic: true,
                body: vec![],
            }),
            name: "__VA_OPT__".into(),
            source: Source::internal(),
            is_file_local_only: false,
        })
    }

    // Evaluate function macro with arguments
    let mut depleted = Depleted::new();
    expand_region(&body, &args_only_environment, &mut depleted)
}

// Handles '# PARAMETER_NAME' sequences inside of function-macro bodies during expansion
fn inject_stringized_arguments(
    function_macro: &FunctionMacro,
    args: &[Vec<PreToken>],
    start_of_macro_call: Source,
) -> Vec<PreToken> {
    let mut result = Vec::new();
    let mut tokens = LookAhead::new(function_macro.body.iter());

    // TODO: CLEANUP: This part could be cleaned up
    while let Some(token) = tokens.next() {
        if let PreTokenKind::Punctuator(Punctuator::Hash) = &token.kind {
            if let Some(PreToken {
                kind: PreTokenKind::Identifier(param_name),
                ..
            }) = tokens.peek_nth(0)
            {
                if let Some((index, _)) = function_macro
                    .parameters
                    .iter()
                    .find_position(|param| *param == param_name)
                {
                    tokens
                        .next()
                        .expect("eat paramater name after '#' during stringization of parameter during macro expansion");

                    let arg_tokens = args.get(index).expect("argument specified for parameter");
                    let stringized = arg_tokens.iter().map(|t| t.to_string()).join(" ");

                    result.push(
                        PreTokenKind::StringLiteral(Encoding::Default, stringized)
                            .at(start_of_macro_call),
                    );
                    continue;
                }
            }
        }

        result.push(token.clone());
    }

    result
}

// Handles '##' concatenation operator
fn resolve_concats(
    tokens: impl Iterator<Item = PreToken>,
    strip_placeholders: bool,
    start_of_macro_call: Source,
) -> Result<Vec<PreToken>, PreprocessorError> {
    let mut tokens = LookAhead::new(tokens);
    let mut result = Vec::new();

    while let Some(first) = tokens.next() {
        if let Some(PreTokenKind::Punctuator(Punctuator::HashConcat)) =
            tokens.peek().map(|token| &token.kind)
        {
            if let Some(second) = tokens.peek_nth(1) {
                let is_two_placeholders =
                    first.kind.is_placeholder() && second.kind.is_placeholder();

                let concat_source = tokens.next().expect("eat '##'").source;

                if is_two_placeholders {
                    // Leave the second placeholder token as the concatenated result.
                    // We need to do this, because it will affect further concatenations.
                } else {
                    let second = tokens.next().expect("second argument to '##'");
                    result.push(concat(&first, &second, concat_source)?);
                }

                continue;
            }
        }

        if first.kind.is_placeholder() {
            if result.last().map_or(false, |token: &PreToken| {
                matches!(token.kind, PreTokenKind::Punctuator(Punctuator::Hash))
            }) {
                // Resolve generated '# (placeholder)' occurances
                result.pop().unwrap();
                result.push(
                    PreTokenKind::StringLiteral(Encoding::Default, "".into())
                        .at(start_of_macro_call),
                );
            } else if !strip_placeholders {
                // Otherwise preserve the placeholder if requested
                result.push(first.clone());
            }
        } else {
            // Not a placeholder token, keep it
            result.push(first.clone());
        }
    }

    Ok(result)
}

fn concat(a: &PreToken, b: &PreToken, source: Source) -> Result<PreToken, PreprocessorError> {
    // NOTE: We don't support concatenating punctuator tokens. It doesn't
    // seem like this feature is ever used intentionally, so we won't support it for now.
    // If someone can find a real-world use case please let me know.
    match (&a.kind, &b.kind) {
        (PreTokenKind::Placeholder, _) => Ok(b.clone()),
        (_, PreTokenKind::Placeholder) => Ok(a.clone()),
        (PreTokenKind::Identifier(a_name), PreTokenKind::Identifier(b_name)) => {
            Ok(PreTokenKind::Identifier(format!("{}{}", a_name, b_name)).at(a.source))
        }
        (PreTokenKind::Identifier(a_name), PreTokenKind::Number(b_number)) => {
            Ok(PreTokenKind::Identifier(format!("{}{}", a_name, b_number)).at(a.source))
        }
        (PreTokenKind::Number(a_number), PreTokenKind::Identifier(b_identifier)) => {
            Ok(PreTokenKind::Number(format!("{}{}", a_number, b_identifier)).at(a.source))
        }
        (
            PreTokenKind::StringLiteral(a_encoding, a_content),
            PreTokenKind::StringLiteral(b_encoding, b_content),
        ) => {
            if a_encoding == b_encoding {
                Ok(PreTokenKind::StringLiteral(
                    a_encoding.clone(),
                    format!("{}{}", a_content, b_content),
                )
                .at(a.source))
            } else {
                Err(PreprocessorErrorKind::CannotConcatTokens.at(source))
            }
        }
        _ => Err(PreprocessorErrorKind::CannotConcatTokens.at(source)),
    }
}
