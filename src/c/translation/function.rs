use super::{parameters::has_parameters, translate_expr, types::get_name_and_type, TranslateCtx};
use crate::{
    asg::TypeParams,
    ast::{self, Func, FuncHead, Param, Params},
    c::{
        ast::{
            Attribute, BlockItem, BlockItemKind, CTypedef, CompoundStatement,
            DeclarationSpecifiers, Declarator, ExprStatement, JumpStatement,
            ParameterDeclarationCore, ParameterTypeList, StorageClassSpecifier, UnlabeledStatement,
        },
        parser::{error::ParseErrorKind, ParseError},
    },
    source_files::Source,
    workspace::compile::c_code::CFileType,
};

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

            required.push(Param::new(Some(param_info.name), param_info.ast_type));
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

    let head = FuncHead {
        name: func_info.name,
        type_params: TypeParams::default(),
        givens: vec![],
        params: Params {
            required,
            is_cstyle_vararg: parameter_type_list.is_variadic,
        },
        return_type: func_info.ast_type,
        is_foreign: true,
        is_exposed: body.is_some(),
        source,
        abide_abi: true,
        tag: None,
        privacy: c_file_type.privacy(),
    };

    ctx.ast_file.funcs.push(Func { head, stmts });

    Ok(())
}

fn translate_compound_statement(
    ctx: &mut TranslateCtx,
    compound_statement: &CompoundStatement,
) -> Result<Vec<ast::Stmt>, ParseError> {
    let mut stmts = vec![];

    for block_item in &compound_statement.statements {
        stmts.push(translate_block_item(ctx, block_item)?);
    }

    Ok(stmts)
}

fn translate_block_item(
    ctx: &mut TranslateCtx,
    block_item: &BlockItem,
) -> Result<ast::Stmt, ParseError> {
    match &block_item.kind {
        BlockItemKind::Declaration(_) => todo!("translate_block_item declaration"),
        BlockItemKind::UnlabeledStatement(unlabeled_statement) => {
            translate_unlabeled_statement(ctx, unlabeled_statement, block_item.source)
        }
        BlockItemKind::Label(_) => todo!("translate_block_item label"),
    }
}

fn translate_unlabeled_statement(
    ctx: &mut TranslateCtx,
    unlabeled_statement: &UnlabeledStatement,
    source: Source,
) -> Result<ast::Stmt, ParseError> {
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

            translate_jump_statement(ctx, jump_statement, source)
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
) -> Result<ast::Stmt, ParseError> {
    match expr_statement {
        ExprStatement::Empty => todo!(),
        ExprStatement::Normal(attributes, expr) => {
            if !attributes.is_empty() {
                return Err(ParseError::message(
                    "Attributes are not supported on expressions yet",
                    expr.source,
                ));
            }

            Ok(ast::StmtKind::Expr(translate_expr(ctx, expr)?).at(expr.source))
        }
    }
}
