/*
    Constant expression parser for C preprocessor #if-like directives.

    Each expression must be re-parsed per environment, since all `#define`s
    must be replaced before parsing the expression, which may include additions that aren't strictly
    on expression level boundries.

    For example:

    ````
    #include <stdio.h>

    #define X 10
    #define Y 10

    #define A X
    #define B == Y

    int main(){
        #if A B
        printf("Hello World\n");
        #endif
        return 0;
    }
    ```

    Should result in:

    ```
    #include <stdio.h>

    int main(){
        printf("Hello World\n");
        return 0;
    }
    ```

    Cursed, I know.
*/

use crate::{
    c::preprocessor::{
        ast::ConstExpr,
        pre_token::{PreToken, PreTokenKind},
        ParseError,
    },
    look_ahead::LookAhead,
};
use std::num::IntErrorKind;

pub struct ExprParser<'a, I>
where
    I: Iterator<Item = &'a PreToken>,
{
    input: LookAhead<I>,
}

impl<'a, I: Iterator<Item = &'a PreToken>> ExprParser<'a, I> {
    pub fn new(tokens: I) -> Self {
        Self {
            input: LookAhead::new(tokens),
        }
    }

    pub fn parse(tokens: I) -> Result<ConstExpr, ParseError> {
        let mut parser = Self::new(tokens);
        parser.parse_expr()
    }

    pub fn parse_expr(&mut self) -> Result<ConstExpr, ParseError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }

    fn parse_expr_primary(&mut self) -> Result<ConstExpr, ParseError> {
        // We don't have any post-fix unary operators,
        // so a primary expression is just the primary base expression
        self.parse_expr_primary_base()
    }

    fn parse_expr_primary_base(&mut self) -> Result<ConstExpr, ParseError> {
        let token = self.input.peek();

        match token.map(|token| &token.kind) {
            Some(PreTokenKind::Identifier(..)) => Ok(ConstExpr::Constant(0)), // Undeclared defines are zero
            Some(PreTokenKind::Number(numeric)) => Self::parse_number(numeric),
            _ => Err(ParseError::ExpectedExpression),
        }
    }

    fn parse_number(number: &str) -> Result<ConstExpr, ParseError> {
        if number.starts_with("0x") || number.starts_with("0X") {
            Self::parse_number_radix(&number[..2], 16)
        } else if number.starts_with("0") {
            Self::parse_number_radix(number, 8)
        } else {
            Self::parse_number_radix(number, 10)
        }
    }

    fn parse_number_radix(number: &str, radix: u32) -> Result<ConstExpr, ParseError> {
        match i64::from_str_radix(number, radix) {
            Ok(value) => Ok(ConstExpr::Constant(value)),
            Err(error) if *error.kind() == IntErrorKind::PosOverflow => {
                match u64::from_str_radix(number, radix) {
                    Ok(value) => Ok(ConstExpr::Constant(value as i64)),
                    Err(_) => Err(ParseError::BadInteger),
                }
            }
            Err(_) => Err(ParseError::BadInteger),
        }
    }

    fn parse_operator_expr(
        &mut self,
        _precedence: usize,
        _expr: ConstExpr,
    ) -> Result<ConstExpr, ParseError> {
        // TODO: Parse operator expressions and handle ternarys
        todo!()
    }
}
