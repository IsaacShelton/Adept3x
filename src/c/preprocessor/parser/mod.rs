mod error;

use super::{
    ast::{
        ControlLine, ControlLineKind, Define, DefineKind, ElifGroup, Group, GroupPart, IfDefKind,
        IfDefLike, IfGroup, IfLike, ObjMacro, PlaceholderAffinity, PreprocessorAst, TextLine,
    },
    error::PreprocessorErrorKind,
    lexer::{LexedLine, PreTokenLine},
    pre_token::{PreToken, PreTokenKind, Punctuator},
    PreprocessorError,
};
use crate::{
    c::preprocessor::ast::{FuncMacro, IfSection},
    diagnostics::{Diagnostics, WarningDiagnostic},
    inflow::{Inflow, TryPeek},
    look_ahead::LookAhead,
    source_files::Source,
};
pub use error::{ParseError, ParseErrorKind};
use itertools::Itertools;
use std::borrow::Borrow;

pub struct Parser<'a, T: Inflow<LexedLine>> {
    lines: T,
    disabled: bool,
    diagnostics: &'a Diagnostics<'a>,
}

impl<'a, T: Inflow<LexedLine>> Parser<'a, T> {
    pub fn new(lines: T, diagnostics: &'a Diagnostics) -> Self {
        Self {
            lines,
            disabled: false,
            diagnostics,
        }
    }

    pub fn parse_eof(&mut self) -> Result<Source, PreprocessorError> {
        match self.lines.next()? {
            PreTokenLine::Line(_, source) => Err(PreprocessorErrorKind::ExpectedEof.at(source)),
            PreTokenLine::EndOfFile(source) => Ok(source),
        }
    }

    pub fn parse_group(&mut self) -> Result<Group, PreprocessorError> {
        let mut parts = Vec::new();

        while let PreTokenLine::Line(tokens, _) = self.lines.try_peek()? {
            if is_group_terminator(tokens) {
                break;
            }

            match self.parse_group_part()? {
                GroupPart::TextLine(TextLine { content }) if content.is_empty() => (),
                part => parts.push(part),
            }
        }

        Ok(Group { parts })
    }

    pub fn parse_group_part(&mut self) -> Result<GroupPart, PreprocessorError> {
        let (entire_line, start_of_line) = match self.lines.next().expect("group part") {
            PreTokenLine::Line(line, start_of_line) => (line, start_of_line),
            PreTokenLine::EndOfFile(eof) => {
                return Err(ParseErrorKind::ExpectedGroupPart.at(eof).into())
            }
        };

        let start_of_line = entire_line
            .first()
            .map(|token| token.source)
            .unwrap_or(start_of_line);

        let directive_name = peek_directive_name(&entire_line);

        match directive_name {
            Some(directive_name @ ("if" | "ifdef" | "ifndef")) => {
                let if_group = match directive_name {
                    "if" => IfGroup::IfLike(self.parse_if_like(&entire_line, start_of_line)?),
                    "ifdef" => IfGroup::IfDefLike(self.parse_if_def_like(
                        IfDefKind::Defined,
                        &entire_line,
                        start_of_line,
                    )?),
                    "ifndef" => IfGroup::IfDefLike(self.parse_if_def_like(
                        IfDefKind::NotDefined,
                        &entire_line,
                        start_of_line,
                    )?),
                    _ => unreachable!(),
                };

                let mut elif_groups = Vec::<ElifGroup>::new();

                while let PreTokenLine::Line(peek_line, _) = self.lines.try_peek()? {
                    elif_groups.push(match peek_directive_name(peek_line) {
                        Some("elif") => {
                            let (line, _) = self.lines.next().unwrap().unwrap_line();
                            ElifGroup::Elif(self.parse_if_like(&line, start_of_line)?)
                        }
                        Some("elifdef") => {
                            let (line, _) = self.lines.next().unwrap().unwrap_line();
                            ElifGroup::ElifDef(self.parse_if_def_like(
                                IfDefKind::Defined,
                                &line,
                                start_of_line,
                            )?)
                        }
                        Some("elifndef") => {
                            let (line, _) = self.lines.next().unwrap().unwrap_line();
                            ElifGroup::ElifDef(self.parse_if_def_like(
                                IfDefKind::NotDefined,
                                &line,
                                start_of_line,
                            )?)
                        }
                        _ => break,
                    })
                }

                let else_group = if let Some("else") =
                    self.lines.try_peek().map_or(None, |line| match line {
                        PreTokenLine::Line(line, _) => peek_directive_name(line),
                        PreTokenLine::EndOfFile(_) => None,
                    }) {
                    let _ = self.lines.next();
                    Some(self.parse_group()?)
                } else {
                    None
                };

                match self.lines.try_peek()? {
                    PreTokenLine::Line(line, source) => {
                        if let Some("endif") = peek_directive_name(line) {
                            if line.len() != 2 {
                                return Err(ParseErrorKind::ExpectedNewlineAfterDirective
                                    .at(*source)
                                    .into());
                            }

                            let _ = self.lines.next();
                        } else {
                            return Err(ParseErrorKind::ExpectedEndif.at(*source).into());
                        }
                    }
                    PreTokenLine::EndOfFile(end_of_file) => {
                        return Err(ParseErrorKind::ExpectedEndif.at(*end_of_file).into());
                    }
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
                        Self::parse_define_func_macro(&entire_line, start_of_line)?
                    } else {
                        Self::parse_define_object_macro(&entire_line, start_of_line)?
                    },
                )
                .at(start_of_line),
            )),
            Some("include") => Ok(GroupPart::ControlLine(
                ControlLineKind::Include(entire_line[2..].to_vec()).at(start_of_line),
            )),
            Some("embed") => Ok(GroupPart::ControlLine(
                ControlLineKind::Embed(entire_line[2..].to_vec()).at(start_of_line),
            )),
            Some("undef") => Ok(GroupPart::ControlLine(Self::parse_undef(
                &entire_line,
                start_of_line,
            )?)),
            Some("line") => Ok(GroupPart::ControlLine(
                ControlLineKind::Line(entire_line[2..].to_vec()).at(start_of_line),
            )),
            Some("error") => Ok(GroupPart::ControlLine(
                ControlLineKind::Error(entire_line[2..].to_vec()).at(start_of_line),
            )),
            Some("warning") => Ok(GroupPart::ControlLine(
                ControlLineKind::Warning(entire_line[2..].to_vec()).at(start_of_line),
            )),
            Some("pragma") => Ok(self.parse_pragma(&entire_line)?),
            Some(unknown) => Err(ParseErrorKind::UnrecognizedDirective(unknown.into())
                .at(start_of_line)
                .into()),
            None => {
                let mut entire_line = entire_line;

                Ok(GroupPart::TextLine(TextLine {
                    content: if self.disabled {
                        entire_line.drain(..).map(PreToken::protect).collect_vec()
                    } else {
                        entire_line
                    },
                }))
            }
        }
    }

    pub fn parse_define_func_macro(
        entire_line: &[PreToken],
        start_of_line: Source,
    ) -> Result<Define, PreprocessorError> {
        let mut tokens = LookAhead::new(entire_line.iter().skip(2));

        let name = eat_identifier(&mut tokens)
            .ok_or_else(|| ParseErrorKind::ExpectedDefinitionName.at(start_of_line))?;

        let source = entire_line
            .get(2)
            .expect("definition name to be specified")
            .source;

        match tokens.next() {
            Some(PreToken {
                kind: PreTokenKind::Punctuator(Punctuator::OpenParen { .. }),
                ..
            }) => (),
            _ => return Err(ParseErrorKind::ExpectedOpenParen.at(start_of_line).into()),
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
                _ => {
                    return Err(ParseErrorKind::ExpectedParameterName
                        .at(start_of_line)
                        .into())
                }
            }

            match tokens.next() {
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::Comma),
                    ..
                }) => {
                    if is_variadic {
                        return Err(ParseErrorKind::ExpectedCloseParenAfterVarArgs
                            .at(start_of_line)
                            .into());
                    }
                }
                Some(PreToken {
                    kind: PreTokenKind::Punctuator(Punctuator::CloseParen),
                    ..
                }) => break,
                _ => return Err(ParseErrorKind::ExpectedComma.at(start_of_line).into()),
            }
        }

        Ok(Define {
            kind: DefineKind::FuncMacro(FuncMacro {
                affinity: PlaceholderAffinity::Discard,
                params: parameters,
                is_variadic,
                body: tokens.cloned().collect_vec(),
            }),
            name: name.to_string(),
            source,
            is_file_local_only: false,
        })
    }

    pub fn parse_define_object_macro(
        line: &[PreToken],
        start_of_line: Source,
    ) -> Result<Define, PreprocessorError> {
        // # define NAME REPLACEMENT_TOKENS...

        let (name, source) = match line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
                source,
            }) => (name.to_string(), *source),
            _ => {
                return Err(ParseErrorKind::ExpectedDefinitionName
                    .at(start_of_line)
                    .into())
            }
        };

        let replacement_tokens = line[3..].to_vec();

        Ok(Define {
            kind: DefineKind::ObjMacro(ObjMacro::new(
                replacement_tokens,
                PlaceholderAffinity::Discard,
            )),
            name,
            source,
            is_file_local_only: false,
        })
    }

    pub fn parse_undef(
        entire_line: &[PreToken],
        start_of_line: Source,
    ) -> Result<ControlLine, ParseError> {
        // # undef NAME

        let name = match entire_line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
                ..
            }) => name.to_string(),
            Some(PreToken { source, .. }) => {
                return Err(ParseErrorKind::ExpectedDefinitionName.at(*source))
            }
            None => return Err(ParseErrorKind::ExpectedDefinitionName.at(start_of_line)),
        };

        if let Some(extraneous) = entire_line.get(3) {
            Err(ParseErrorKind::ExpectedNewlineAfterDirective.at(extraneous.source))
        } else {
            Ok(ControlLineKind::Undef(name).at(start_of_line))
        }
    }

    pub fn parse_pragma(&mut self, line: &[PreToken]) -> Result<GroupPart, PreprocessorError> {
        // # pragma ...

        let (name, source) = match line.get(2) {
            Some(PreToken {
                kind: PreTokenKind::Identifier(name),
                source,
            }) => (name.as_str(), *source),
            _ => {
                let pragma_source = line.get(1).unwrap().source;

                return Err(ParseErrorKind::UnrecognizedPragmaDirective(
                    "<invalid pragma directive>".into(),
                )
                .at(pragma_source)
                .into());
            }
        };

        if name == "ADEPT" {
            self.parse_adept_pragma(line)?;
            Ok(GroupPart::TextLine(TextLine { content: vec![] }))
        } else if name == "once" {
            self.diagnostics.push(WarningDiagnostic::new(
                "Directive `#pragma once` is not supported yet",
                source,
            ));
            Ok(GroupPart::TextLine(TextLine { content: vec![] }))
        } else if name == "STDC" {
            self.diagnostics.push(WarningDiagnostic::new(
                "Directive `#pragma STDC` is not supported yet",
                source,
            ));
            Ok(GroupPart::TextLine(TextLine { content: vec![] }))
        } else {
            Err(ParseErrorKind::UnrecognizedPragmaDirective(name.into())
                .at(source)
                .into())
        }
    }

    pub fn parse_adept_pragma(&mut self, line: &[PreToken]) -> Result<(), PreprocessorError> {
        // #pragma ADEPT ...
        //               ^

        let start_source = line[2].source;

        if let Some("PREPROCESSOR") = line.get(3).and_then(|category| category.get_identifier()) {
            match line.get(4).and_then(|action| action.get_identifier()) {
                Some("ENABLE") => {
                    self.disabled = false;
                    return Ok(());
                }
                Some("DISABLE") => {
                    self.disabled = true;
                    return Ok(());
                }
                _ => (),
            }
        }

        Err(ParseErrorKind::UnrecognizedAdeptPragmaDirective
            .at(start_source)
            .into())
    }

    pub fn parse_if_like(
        &mut self,
        line: &[PreToken],
        start_of_line: Source,
    ) -> Result<IfLike, PreprocessorError> {
        Ok(IfLike {
            tokens: Self::prepare_for_parsing(&line[2..], start_of_line)?,
            group: self.parse_group()?,
            source: start_of_line,
        })
    }

    pub fn parse_if_def_like(
        &mut self,
        kind: IfDefKind,
        entire_line: &[PreToken],
        start_of_line: Source,
    ) -> Result<IfDefLike, PreprocessorError> {
        let identifier = Self::parse_ifdef_name(&entire_line[2..], start_of_line)?;

        Ok(IfDefLike {
            kind,
            identifier,
            group: self.parse_group()?,
        })
    }

    pub fn parse_ifdef_name(
        rest_line: &[PreToken],
        start_of_line: Source,
    ) -> Result<String, PreprocessorError> {
        match rest_line {
            [PreToken {
                kind: PreTokenKind::Identifier(identifier),
                ..
            }, ..] => {
                if let Some(extraneous) = rest_line.get(1) {
                    Err(ParseErrorKind::UnexpectedToken {
                        after: "identifier for preprocessor directive".into(),
                    }
                    .at(extraneous.source)
                    .into())
                } else {
                    Ok(identifier.into())
                }
            }
            _ => Err(ParseErrorKind::ExpectedMacroNameFor
                .at(start_of_line)
                .into()),
        }
    }

    pub fn prepare_for_parsing(
        tokens: &[PreToken],
        start_of_line: Source,
    ) -> Result<Vec<PreToken>, PreprocessorError> {
        let mut tokens = LookAhead::new(tokens.iter());
        let mut result = Vec::with_capacity(tokens.len());

        while let Some(token) = tokens.next() {
            match &token.kind {
                PreTokenKind::Identifier(name) if name.as_str() == "defined" => {
                    let new_token_kind = match tokens.next() {
                        Some(PreToken {
                            kind: PreTokenKind::Identifier(name),
                            ..
                        }) => Ok(PreTokenKind::IsDefined(name.clone())),
                        Some(PreToken {
                            kind: PreTokenKind::Punctuator(Punctuator::OpenParen { .. }),
                            source,
                        }) => eat_identifier(&mut tokens)
                            .ok_or_else(|| {
                                ParseErrorKind::ExpectedDefinitionName.at(*source).into()
                            })
                            .and_then(|name| {
                                eat_punctuator(&mut tokens, Punctuator::CloseParen, start_of_line)
                                    .map(|_| PreTokenKind::IsDefined(name.to_string()))
                            }),
                        _ => Err(ParseErrorKind::ExpectedDefinitionName
                            .at(start_of_line)
                            .into()),
                    }?;

                    result.push(new_token_kind.at(token.source));
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
    start_of_line: Source,
) -> Result<(), PreprocessorError> {
    match input.peek().map(|token| &token.kind) {
        Some(PreTokenKind::Punctuator(punctuator)) if *punctuator == *expected.borrow() => {
            input.next().unwrap();
            Ok(())
        }
        _ => Err(
            ParseErrorKind::ExpectedPunctuator(expected.borrow().clone())
                .at(start_of_line)
                .into(),
        ),
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

fn first_tokens<const N: usize>(
    line: &(impl Borrow<[PreToken]> + ?Sized),
) -> Option<[&PreTokenKind; N]> {
    line.borrow()
        .first_chunk::<N>()
        .map(|tokens| std::array::from_fn(|i| &tokens[i].kind))
}

fn is_group_terminator(line: &[PreToken]) -> bool {
    matches!(
        peek_directive_name(line),
        Some("elif" | "elifdef" | "elifndef" | "else" | "endif")
    )
}

fn peek_directive_name(line: &(impl Borrow<[PreToken]> + ?Sized)) -> Option<&str> {
    match first_tokens::<2>(line) {
        Some(
            [PreTokenKind::Punctuator(Punctuator::Hash), PreTokenKind::Identifier(directive_name)],
        ) => Some(directive_name),
        _ => None,
    }
}

pub fn parse(
    tokens: impl Inflow<LexedLine>,
    diagnostics: &Diagnostics,
) -> Result<PreprocessorAst, PreprocessorError> {
    let mut parser = Parser::new(tokens, diagnostics);

    Ok(PreprocessorAst {
        group: parser.parse_group()?,
        eof: parser.parse_eof()?,
    })
}
