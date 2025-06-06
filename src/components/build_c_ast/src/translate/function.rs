use super::{TranslateCtx, parameters::has_parameters, translate_expr, types::get_name_and_type};
use crate::{
    CFileType,
    parse::{ParseError, error::ParseErrorKind},
};
use attributes::{Exposure, SymbolOwnership, Tag};
use c_ast::*;
use source_files::Source;

pub fn declare_function(
    ctx: &mut TranslateCtx,
    _attribute_specifiers: &[Attribute],
    declaration_specifiers: &DeclarationSpecifiers,
    declarator: &Declarator,
    parameter_type_list: &ParameterTypeList,
    body: Option<CompoundStatement>,
    c_file_type: CFileType,
) -> Result<(), ParseError> {
    let source = declarator.source;
    let func_info = get_name_and_type(ctx, declarator, declaration_specifiers, false)?;
    let mut required = vec![];

    if func_info.specifiers.function_specifier.is_some() {
        return Err(ParseErrorKind::Misc("Function specifiers are not supported yet").at(source));
    }

    if has_parameters(parameter_type_list) {
        for param in parameter_type_list.parameter_declarations.iter() {
            let param_info = match &param.core {
                ParameterDeclarationCore::Declarator(declarator) => {
                    get_name_and_type(ctx, declarator, &param.declaration_specifiers, true)?
                }
                ParameterDeclarationCore::AbstractDeclarator(_) => {
                    todo!("translate abstact declaration core")
                }
                ParameterDeclarationCore::Nothing => {
                    todo!("translate parameter declaration core nothing")
                }
            };

            if param_info.specifiers.storage_class.is_some() {
                return Err(ParseError::message(
                    "Storage classes not support on typedef",
                    param.source,
                ));
            }

            if param_info.specifiers.function_specifier.is_some() {
                return Err(ParseError::message(
                    "Function specifiers cannot be used on typedef",
                    source,
                ));
            }

            required.push(ast::Param::new(Some(param_info.name), param_info.ast_type));
        }
    }

    match func_info.specifiers.storage_class {
        Some(StorageClassSpecifier::Typedef) => {
            if body.is_some() {
                return Err(ParseError::message(
                    "Cannot typedef function with body",
                    declarator.source,
                ));
            }

            let ast_type = ast::TypeKind::FuncPtr(ast::FuncPtr {
                parameters: required,
                return_type: Box::new(func_info.ast_type),
                is_cstyle_variadic: parameter_type_list.is_variadic,
            })
            .at(declarator.source);

            ctx.typedefs.insert(func_info.name, CTypedef { ast_type });
            return Ok(());
        }
        Some(_) => {
            return Err(
                ParseErrorKind::Misc("Unsupported storage class here").at(declarator.source)
            );
        }
        None => (),
    }

    let stmts = body
        .as_ref()
        .map(|body| translate_compound_statement(ctx, body))
        .transpose()?
        .unwrap_or_default();

    let ownership = if body.is_some() {
        SymbolOwnership::Owned(Exposure::Exposed)
    } else {
        SymbolOwnership::Reference
    };

    let tag = (func_info.name == "main").then_some(Tag::Main);

    let head = ast::FuncHead {
        name: func_info.name,
        type_params: ast::TypeParams::default(),
        givens: vec![],
        params: ast::Params {
            required,
            is_cstyle_vararg: parameter_type_list.is_variadic,
        },
        return_type: func_info.ast_type,
        ownership,
        source,
        abide_abi: true,
        tag,
        privacy: c_file_type.privacy(),
    };

    ctx.ast_file.funcs.push(ast::Func::new(head, stmts));
    Ok(())
}

fn translate_compound_statement(
    ctx: &mut TranslateCtx,
    compound_statement: &CompoundStatement,
) -> Result<Vec<ast::Stmt>, ParseError> {
    let mut stmts = vec![];

    for block_item in &compound_statement.statements {
        stmts.extend(translate_block_item(ctx, block_item)?);
    }

    Ok(stmts)
}

fn translate_block_item(
    ctx: &mut TranslateCtx,
    block_item: &BlockItem,
) -> Result<Option<ast::Stmt>, ParseError> {
    match &block_item.kind {
        BlockItemKind::Declaration(declaration) => match declaration {
            Declaration::Common(_) => todo!("translate_block_item common declaration"),
            Declaration::StaticAssert(static_assert) => Ok(Some(
                ast::ExprKind::StaticAssert(
                    Box::new(translate_expr(ctx, &static_assert.condition.value)?),
                    static_assert.message.clone(),
                )
                .at(block_item.source)
                .stmt(),
            )),
            Declaration::Attribute(_) => todo!("translate_block_item attribute declaration"),
        },
        BlockItemKind::UnlabeledStatement(unlabeled_statement) => Ok(
            translate_unlabeled_statement(ctx, unlabeled_statement, block_item.source)?,
        ),
        BlockItemKind::Label(_) => todo!("translate_block_item label"),
    }
}

fn translate_unlabeled_statement(
    ctx: &mut TranslateCtx,
    unlabeled_statement: &UnlabeledStatement,
    source: Source,
) -> Result<Option<ast::Stmt>, ParseError> {
    match unlabeled_statement {
        UnlabeledStatement::ExprStatement(expr_statement) => {
            translate_expr_statement(ctx, expr_statement)
        }
        UnlabeledStatement::PrimaryBlock(_, _) => {
            todo!("translate_unlabeled_statement primary block")
        }
        UnlabeledStatement::JumpStatement(attributes, jump_statement) => {
            if !attributes.is_empty() {
                return Err(ParseError::message(
                    "Attributes on jump statements are not supported yet",
                    source,
                ));
            }

            Ok(Some(translate_jump_statement(ctx, jump_statement, source)?))
        }
    }
}

fn translate_jump_statement(
    ctx: &mut TranslateCtx,
    jump_statement: &JumpStatement,
    source: Source,
) -> Result<ast::Stmt, ParseError> {
    Ok(match jump_statement {
        JumpStatement::Goto(_) => todo!("translate goto statement"),
        JumpStatement::Continue => ast::StmtKind::Expr(ast::ExprKind::Continue.at(source)),
        JumpStatement::Break => ast::StmtKind::Expr(ast::ExprKind::Break.at(source)),
        JumpStatement::Return(expr) => {
            let expr = expr
                .as_ref()
                .map(|expr| translate_expr(ctx, expr))
                .transpose()?;
            ast::StmtKind::Return(expr)
        }
    }
    .at(source))
}

fn translate_expr_statement(
    ctx: &mut TranslateCtx,
    expr_statement: &ExprStatement,
) -> Result<Option<ast::Stmt>, ParseError> {
    match expr_statement {
        ExprStatement::Empty => Ok(None),
        ExprStatement::Normal(attributes, expr) => {
            if !attributes.is_empty() {
                return Err(ParseError::message(
                    "Attributes are not supported on expressions yet",
                    expr.source,
                ));
            }

            Ok(Some(
                ast::StmtKind::Expr(translate_expr(ctx, expr)?).at(expr.source),
            ))
        }
    }
}
