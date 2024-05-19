use itertools::Itertools;

use super::{
    ast::{
        ControlLine, ControlLineKind, Define, DefineKind, ElifGroup, Group, GroupPart, IfDefKind,
        IfDefLike, IfGroup, IfLike, PlaceholderAffinity, PreprocessorAst, TextLine,
    },
    pre_token::{PreToken, PreTokenKind, Punctuator},
    ParseError, ParseErrorKind,
};
use crate::{
    c::preprocessor::ast::{FunctionMacro, IfSection},
    look_ahead::LookAhead,
};
use std::{borrow::Borrow, num::NonZeroU32};

pub struct Parser<I: Iterator<Item = Vec<PreToken>>> {
    lines: LookAhead<I>,
}

fn first_tokens<const N: usize>(
    line: &(impl Borrow<[PreToken]> + ?Sized),
) -> Option<[&PreTokenKind; N]> {
    line.borrow()
        .first_chunk::<N>()
        .map(|tokens| std::array::from_fn(|i| &tokens[i].kind))
}

fn is_group_terminator(line: &[PreToken]) -> bool {
    match get_directive_name(line) {
        Some("elif" | "elifdef" | "elifndef" | "else" | "endif") => true,
        _ => false,
    }
}

fn get_directive_name(line: &(impl Borrow<[PreToken]> + ?Sized)) -> Option<&str> {
    match first_tokens::<2>(line) {
        Some(
            [PreTokenKind::Punctuator(Punctuator::Hash), PreTokenKind::Identifier(directive_name)],
        ) => Some(directive_name),
        _ => None,
    }
}

impl<I: Iterator<Item = Vec<PreToken>>> Parser<I> {
    pub fn new(tokens: I) -> Self {
        Self {
            lines: LookAhead::new(tokens),
        }
    }

    pub fn parse_group(&mut self) -> Result<Group, ParseError> {
        let mut parts = Vec::new();

        loop {
            match self.lines.peek() {
                Some(line) if is_group_terminator(line) => break,
                Some(_) => parts.push(self.parse_group_part()?),
                None => break,
            }
        }

        Ok(Group { parts })
    }

    pub fn parse_group_part(&mut self) -> Result<GroupPart, ParseError> {
        let entire_line = match self.lines.next() {
            Some(line) => line,
            None => return Err(ParseErrorKind::ExpectedGroupPart.at(None)),
        };

        let line = entire_line.first().and_then(|token| token.line);
        let directive_name = get_directive_name(&entire_line);

        match directive_name {
            Some(directive_name @ ("if" | "ifdef" | "ifndef")) => {
                let if_group = match directive_name {
                    "if" => IfGroup::IfLike(self.parse_if_like(&entire_line)?),
                    "ifdef" => IfGroup::IfDefLike(
                        self.parse_if_def_like(IfDefKind::Defined, &entire_line)?,
                    ),
                    "ifndef" => IfGroup::IfDefLike(
                        self.parse_if_def_like(IfDefKind::NotDefined, &entire_line)?,
                    ),
                    _ => unreachable!(),
                };

                let mut elif_groups = Vec::<ElifGroup>::new();

                while let Some(peek_line) = self.lines.peek() {
                    elif_groups.push(match get_directive_name(peek_line) {
                        Some("elif") => {
                            let line = self.lines.next().unwrap();
                            ElifGroup::Elif(self.parse_if_like(&line)?)
                        }
                        Some("elifdef") => {
                            let line = self.lines.next().unwrap();
                            ElifGroup::ElifDef(self.parse_if_def_like(IfDefKind::Defined, &line)?)
                        }
                        Some("elifndef") => {
                            let line = self.lines.next().unwrap();
                            ElifGroup::ElifDef(
                                self.parse_if_def_like(IfDefKind::NotDefined, &line)?,
                            )
                        }
                        _ => break,
                    })
                }

                let else_group = if let Some("else") =
                    self.lines.peek().and_then(|line| get_directive_name(line))
                {
                    self.lines.next();
                    Some(self.parse_group()?)
                } else {
                    None
                };

                if let Some("endif") = self.lines.peek().and_then(|line| get_directive_name(line)) {
                    self.lines.next();
                } else {
                    return Err(ParseErrorKind::ExpectedEndif.at(None));
                }

                Ok(GroupPart::IfSection(IfSection {
                    if_group,
                    elif_groups,
                    else_group,
                }))
            }
            Some("define") => Ok(GroupPart::ControlLine(
                ControlLineKind::Define(
                    if entire_line.get(3).map_or(false, |token| {
                        matches!(
                            token.kind,
                            PreTokenKind::Punctuator(Punctuator::OpenParen {
                                preceeded_by_whitespace: false
                            })
                        )
                    }) {
                        Self::parse_define_function_macro(&entire_line)?
                    } else {
                        Self::parse_define_object_macro(&entire_line)?
                    },
                )
                .at(line),
            )),
            Some("include") => Ok(GroupPart::ControlLine(
                ControlLineKind::Include(entire_line[2..].to_vec()).at(line),
            )),
            Some("embed") => Ok(GroupPart::ControlLine(
                ControlLineKind::Embed(entire_line[2..].to_vec()).at(line),
            )),
            Some("undef") => Ok(GroupPart::ControlLine(Self::parse_undef(
                &entire_line,
                line,
            )?)),
            Some("line") => Ok(GroupPart::ControlLine(
                ControlLineKind::Line(entire_line[2..].to_vec()).at(line),
            )),
            Some("error") => Ok(GroupPart::ControlLine(
                ControlLineKind::Error(entire_line[2..].to_vec()).at(line),
            )),
            Some("warning") => Ok(GroupPart::ControlLine(
                ControlLineKind::Warning(entire_line[2..].to_vec()).at(line),
            )),
            Some("pragma") => Ok(Self::parse_pragma(&entire_line)?),
            Some(unknown) => Err(ParseErrorKind::UnrecognizedDirective(unknown.into())
                .at(entire_line.first().and_then(|token| token.line))),
            None => Ok(GroupPart::TextLine(TextLine {
                content: entire_line,
            })),
        }
    }

    pub fn parse_define_function_macro(entire_line: &[PreToken]) -> Result<Define, ParseError> {
        let line_number = entire_line.first().and_then(|token| token.line);
        let mut tokens = LookAhead::new(entire_line.iter().skip(2));

        let name = eat_identifier(&mut tokens).ok_or_else(|| {
            ParseErrorKind::ExpectedDefinitionName
                .at(entire_line.first().and_then(|token| token.line))
        })?;

        match tokens.next() {
            Some(PreToken {
                kind: PreTokenKind::Punctuator(Punctuator::OpenParen { .. }),
                ..
            }) => (),
            _ => return Err(ParseErrorKind::ExpectedOpenParen.at(line_number)),
        }

        let mut parameters = Vec::new();
        let mut is_variadic = false;

        loop {
            match tokens.next() {
                Some(PreToken {
                    kind: PreTokenKind::Identifier(name),
                    ..
                }) => {
                    parameters.push(name.into());
                }
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::Ellipses),
                    ..
                }) => {
                    is_variadic = true;
                }
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::CloseParen),
                    ..
                }) if parameters.is_empty() => break,
                _ => return Err(ParseErrorKind::ExpectedParameterName.at(line_number)),
            }

            match tokens.next() {
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::Comma),
                    ..
                }) => {
                    if is_variadic {
                        return Err(ParseErrorKind::ExpectedCloseParenAfterVarArgs.at(line_number));
                    }
                }
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::CloseParen),
                    ..
                }) => break,
                _ => return Err(ParseErrorKind::ExpectedComma.at(line_number)),
            }
        }

        Ok(Define {
            kind: DefineKind::FunctionMacro(FunctionMacro {
                affinity: PlaceholderAffinity::Discard,
                parameters,
                is_variadic,
                body: tokens.cloned().collect_vec(),
            }),
            name: name.to_string(),
        })
    }

    pub fn parse_define_object_macro(line: &[PreToken]) -> Result<Define, ParseError> {
        // # define NAME REPLACEMENT_TOKENS...

        let line_number = line.first().and_then(|token| token.line);

        let name = match line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
                ..
            }) => name.to_string(),
            _ => return Err(ParseErrorKind::ExpectedDefinitionName.at(line_number)),
        };

        let replacement_tokens = line[3..].to_vec();

        Ok(Define {
            kind: DefineKind::ObjectMacro(replacement_tokens, PlaceholderAffinity::Discard),
            name,
        })
    }

    pub fn parse_undef(
        entire_line: &[PreToken],
        line: Option<NonZeroU32>,
    ) -> Result<ControlLine, ParseError> {
        // # undef NAME

        let name = match entire_line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
                ..
            }) => name.to_string(),
            Some(PreToken { line, .. }) => {
                return Err(ParseErrorKind::ExpectedDefinitionName.at(*line))
            }
            None => return Err(ParseErrorKind::ExpectedDefinitionName.at(line)),
        };

        if let Some(extraneous) = entire_line.get(3) {
            Err(ParseErrorKind::ExpectedNewlineAfterDirective.at(extraneous.line))
        } else {
            Ok(ControlLineKind::Undef(name).at(line))
        }
    }

    pub fn parse_pragma(line: &[PreToken]) -> Result<GroupPart, ParseError> {
        // # pragma ...

        let name = match line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
                ..
            }) => Some(name.as_str()),
            _ => None,
        };

        if let Some("STDC") = name {
            eprintln!("warning: #pragma STDC not supported yet");
            Ok(GroupPart::TextLine(TextLine { content: vec![] }))
        } else {
            Err(ParseErrorKind::UnrecognizedPragmaDirective(
                name.unwrap_or("<invalid pragma directive>").into(),
            )
            .at(line.first().and_then(|token| token.line)))
        }
    }

    pub fn parse_if_like(&mut self, line: &[PreToken]) -> Result<IfLike, ParseError> {
        Ok(IfLike {
            tokens: Self::prepare_for_parsing(&line[2..])?,
            group: self.parse_group()?,
        })
    }

    pub fn parse_if_def_like(
        &mut self,
        kind: IfDefKind,
        entire_line: &[PreToken],
    ) -> Result<IfDefLike, ParseError> {
        let line_number = entire_line.get(0).and_then(|token| token.line);
        let identifier = Self::parse_line_identifier(&entire_line[2..], line_number)?;

        Ok(IfDefLike {
            kind,
            identifier,
            group: self.parse_group()?,
        })
    }

    pub fn parse_line_identifier(
        rest_line: &[PreToken],
        line_number: Option<NonZeroU32>,
    ) -> Result<String, ParseError> {
        match rest_line {
            [PreToken {
                kind: PreTokenKind::Identifier(identifier),
                ..
            }, ..] => {
                if let Some(extraneous) = rest_line.get(1) {
                    Err(ParseErrorKind::UnexpectedToken {
                        after: "identifier for preprocessor directive".into(),
                    }
                    .at(extraneous.line))
                } else {
                    Ok(identifier.into())
                }
            }
            _ => Err(ParseErrorKind::ExpectedIdentifier.at(line_number)),
        }
    }

    pub fn prepare_for_parsing(tokens: &[PreToken]) -> Result<Vec<PreToken>, ParseError> {
        let mut tokens = LookAhead::new(tokens.iter());
        let mut result = Vec::with_capacity(tokens.len());

        while let Some(token) = tokens.next() {
            match &token.kind {
                PreTokenKind::Identifier(name) if name.as_str() == "defined" => {
                    let new_token_kind = match tokens.next().map(|token| &token.kind) {
                        Some(PreTokenKind::Identifier(name)) => {
                            Ok(PreTokenKind::IsDefined(name.clone()))
                        }
                        Some(PreTokenKind::Punctuator(Punctuator::OpenParen { .. })) => {
                            eat_identifier(&mut tokens)
                                .ok_or_else(|| {
                                    ParseErrorKind::ExpectedDefinitionName.at(token.line)
                                })
                                .and_then(|name| {
                                    eat_punctuator(&mut tokens, Punctuator::CloseParen, token.line)
                                        .and_then(|_| Ok(PreTokenKind::IsDefined(name.to_string())))
                                })
                        }
                        _ => Err(ParseErrorKind::ExpectedDefinitionName.at(token.line)),
                    }?;

                    result.push(PreToken::new(new_token_kind, token.line));
                }
                _ => result.push(token.clone()),
            }
        }

        Ok(result)
    }
}

pub fn eat_punctuator<'a>(
    input: &mut LookAhead<impl Iterator<Item = &'a PreToken>>,
    expected: impl Borrow<Punctuator>,
    line_number: Option<NonZeroU32>,
) -> Result<(), ParseError> {
    match input.peek().map(|token| &token.kind) {
        Some(PreTokenKind::Punctuator(punctuator)) if *punctuator == *expected.borrow() => {
            input.next().unwrap();
            Ok(())
        }
        _ => Err(ParseErrorKind::ExpectedPunctuator(expected.borrow().clone()).at(line_number)),
    }
}

pub fn eat_identifier<'a>(
    input: &mut LookAhead<impl Iterator<Item = &'a PreToken>>,
) -> Option<&'a str> {
    match input.peek().map(|token| &token.kind) {
        Some(PreTokenKind::Identifier(identifier)) => {
            input.next().unwrap();
            Some(identifier)
        }
        _ => None,
    }
}

pub fn parse(tokens: impl Iterator<Item = Vec<PreToken>>) -> Result<PreprocessorAst, ParseError> {
    let mut parser = Parser::new(tokens);

    Ok(PreprocessorAst {
        group: parser.parse_group()?,
    })
}
