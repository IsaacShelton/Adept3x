use itertools::Itertools;

use super::{
    ast::{
        ControlLine, Define, DefineKind, ElifGroup, Group, GroupPart, IfDefKind, IfDefLike,
        IfGroup, IfLike, PreprocessorAst, TextLine,
    },
    pre_token::{PreToken, PreTokenKind, Punctuator},
    ParseError,
};
use crate::{
    c::preprocessor::ast::{IfSection, FunctionMacro},
    look_ahead::LookAhead,
};
use std::borrow::Borrow;

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
        let line = match self.lines.next() {
            Some(line) => line,
            None => return Err(ParseError::ExpectedGroupPart),
        };

        let directive_name = get_directive_name(&line);

        match directive_name {
            Some(directive_name @ ("if" | "ifdef" | "ifndef")) => {
                let if_group = match directive_name {
                    "if" => IfGroup::IfLike(self.parse_if_like(&line)?),
                    "ifdef" => {
                        IfGroup::IfDefLike(self.parse_if_def_like(IfDefKind::Defined, &line)?)
                    }
                    "ifndef" => {
                        IfGroup::IfDefLike(self.parse_if_def_like(IfDefKind::NotDefined, &line)?)
                    }
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
                    return Err(ParseError::ExpectedEndif);
                }

                Ok(GroupPart::IfSection(IfSection {
                    if_group,
                    elif_groups,
                    else_group,
                }))
            }
            Some("define") => Ok(GroupPart::ControlLine(ControlLine::Define(
                if line.get(3).map_or(false, |line| {
                    matches!(
                        line.kind,
                        PreTokenKind::Punctuator(Punctuator::OpenParen {
                            preceeded_by_whitespace: false
                        })
                    )
                }) {
                    Self::parse_define_function_macro(&line)?
                } else {
                    Self::parse_define_object_macro(&line)?
                },
            ))),
            Some("include") => Ok(GroupPart::ControlLine(ControlLine::Include(
                line[2..].to_vec(),
            ))),
            Some("embed") => Ok(GroupPart::ControlLine(ControlLine::Embed(
                line[2..].to_vec(),
            ))),
            Some("undef") => Ok(GroupPart::ControlLine(Self::parse_undef(&line)?)),
            Some("line") => Ok(GroupPart::ControlLine(ControlLine::Line(
                line[2..].to_vec(),
            ))),
            Some("error") => Ok(GroupPart::ControlLine(ControlLine::Error(
                line[2..].to_vec(),
            ))),
            Some("warning") => Ok(GroupPart::ControlLine(ControlLine::Warning(
                line[2..].to_vec(),
            ))),
            Some("pragma") => Ok(Self::parse_pragma(&line)?),
            Some(unknown) => Err(ParseError::UnrecognizedDirective(unknown.into())),
            None => Ok(GroupPart::TextLine(TextLine { content: line })),
        }
    }

    pub fn parse_define_function_macro(line: &[PreToken]) -> Result<Define, ParseError> {
        let mut tokens = LookAhead::new(line.iter().skip(2));

        let name = match tokens.next() {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
            }) => name.to_string(),
            _ => return Err(ParseError::ExpectedDefinitionName),
        };

        match tokens.next() {
            Some(PreToken {
                kind: PreTokenKind::Punctuator(Punctuator::OpenParen { .. }),
            }) => (),
            _ => return Err(ParseError::ExpectedOpenParen),
        }

        let mut parameters = Vec::new();
        let mut is_variadic = false;

        loop {
            match tokens.next() {
                Some(PreToken {
                    kind: PreTokenKind::Identifier(name),
                }) => {
                    parameters.push(name.into());
                }
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::Ellipses),
                }) => {
                    is_variadic = true;
                }
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::CloseParen),
                }) if parameters.is_empty() => break,
                _ => return Err(ParseError::ExpectedParameterName),
            }

            match tokens.next() {
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::Comma),
                }) => {
                    if is_variadic {
                        return Err(ParseError::ExpectedCloseParenAfterVarArgs);
                    }
                }
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::CloseParen),
                }) => break,
                _ => return Err(ParseError::ExpectedComma),
            }
        }

        Ok(Define {
            kind: DefineKind::FunctionMacro(FunctionMacro {
                parameters,
                is_variadic,
                body: tokens.cloned().collect_vec(),
            }),
            name,
        })
    }

    pub fn parse_define_object_macro(line: &[PreToken]) -> Result<Define, ParseError> {
        // # define NAME REPLACEMENT_TOKENS...

        let name = match line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
            }) => name.to_string(),
            _ => return Err(ParseError::ExpectedDefinitionName),
        };

        let replacement_tokens = line[3..].to_vec();

        Ok(Define {
            kind: DefineKind::ObjectMacro(replacement_tokens),
            name,
        })
    }

    pub fn parse_undef(line: &[PreToken]) -> Result<ControlLine, ParseError> {
        // # undef NAME

        let name = match line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
            }) => name.to_string(),
            _ => return Err(ParseError::ExpectedDefinitionName),
        };

        if line.len() != 3 {
            Err(ParseError::ExpectedNewlineAfterDirective)
        } else {
            Ok(ControlLine::Undef(name))
        }
    }

    pub fn parse_pragma(line: &[PreToken]) -> Result<GroupPart, ParseError> {
        // # pragma ...

        let name = match line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
            }) => Some(name.as_str()),
            _ => None,
        };

        if let Some("STDC") = name {
            eprintln!("warning: #pragma STDC not supported yet");
            Ok(GroupPart::TextLine(TextLine { content: vec![] }))
        } else {
            Err(ParseError::UnrecognizedPragmaDirective(
                name.unwrap_or("<invalid pragma directive>").into(),
            ))
        }
    }

    pub fn parse_if_like(&mut self, line: &[PreToken]) -> Result<IfLike, ParseError> {
        Ok(IfLike {
            tokens: line[2..].to_vec(),
            group: self.parse_group()?,
        })
    }

    pub fn parse_if_def_like(
        &mut self,
        kind: IfDefKind,
        line: &[PreToken],
    ) -> Result<IfDefLike, ParseError> {
        let identifier = Self::parse_line_identifier(&line[2..])?;

        Ok(IfDefLike {
            kind,
            identifier,
            group: self.parse_group()?,
        })
    }

    pub fn parse_line_identifier(rest_line: &[PreToken]) -> Result<String, ParseError> {
        match rest_line {
            [PreToken {
                kind: PreTokenKind::Identifier(identifier),
            }, ..] => {
                if rest_line.len() == 1 {
                    Ok(identifier.into())
                } else {
                    Err(ParseError::UnexpectedToken {
                        after: "identifier for preprocessor directive".into(),
                    })
                }
            }
            _ => Err(ParseError::ExpectedIdentifier),
        }
    }
}

pub fn parse(tokens: impl Iterator<Item = Vec<PreToken>>) -> Result<PreprocessorAst, ParseError> {
    let mut parser = Parser::new(tokens);

    Ok(PreprocessorAst {
        group: parser.parse_group()?,
    })
}
