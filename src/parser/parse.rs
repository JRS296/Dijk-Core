use super::error::{ParseError, ParseErrorKind};
use super::syntax::{
    BinaryOp, Block, ElseBranch, Expr, Function, Param, Program, Stmt, Type, UnaryOp,
};
use crate::lexer::token::{Span, Token, TokenKind};

/// Parse a complete token stream (as produced by [`crate::lexer::tokenize`])
/// into a [`Program`], alongside any parse errors encountered.
///
/// Parsing continues past an error where practical (see `docs/parser.md`
/// for the recovery strategy), so a single pass can surface more than one
/// problem. `tokens` must end with an `Eof` token, as the lexer guarantees.
pub fn parse(tokens: Vec<Token>) -> (Program, Vec<ParseError>) {
    let mut parser = Parser::new(tokens);
    let mut functions = Vec::new();

    while !parser.is_at_end() {
        match parser.parse_function() {
            Ok(function) => functions.push(function),
            Err(err) => {
                parser.errors.push(err);
                parser.synchronize_to_declaration();
            }
        }
    }

    parser.errors.sort_by_key(|e| e.span.start);
    (Program { functions }, parser.errors)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<ParseError>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: Vec::new(),
        }
    }

    // --- token stream primitives ---

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> TokenKind {
        self.peek().kind
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    fn is_at_end(&self) -> bool {
        self.peek_kind() == TokenKind::Eof
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.pos].clone();
        if !self.is_at_end() {
            self.pos += 1;
        }
        token
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind, expected: &str) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error_at_current(expected))
        }
    }

    fn error_at_current(&self, expected: &str) -> ParseError {
        let token = self.peek();
        ParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: expected.to_string(),
                found: token.kind,
            },
            token.span,
        )
    }

    /// After a declaration fails to parse, skip tokens until the next
    /// likely declaration start (`fn`) or end of file.
    fn synchronize_to_declaration(&mut self) {
        while !self.is_at_end() {
            if self.check(TokenKind::Fn) {
                return;
            }
            self.advance();
        }
    }

    /// After a statement fails to parse, skip tokens until the statement
    /// terminator `;` (consumed) or the end of the enclosing block `}`
    /// (left for the block loop to see), so a single block can still
    /// report more than one broken statement.
    fn synchronize_to_statement(&mut self) {
        while !self.is_at_end() {
            if self.check(TokenKind::Semicolon) {
                self.advance();
                return;
            }
            if self.check(TokenKind::RightBrace) {
                return;
            }
            self.advance();
        }
    }

    // --- declarations ---

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let fn_tok = self.expect(TokenKind::Fn, "'fn'")?;
        let name_tok = self.expect(TokenKind::Identifier, "function name")?;
        self.expect(TokenKind::LeftParen, "'('")?;
        let params = self.parse_parameters()?;
        self.expect(TokenKind::RightParen, "')'")?;

        let return_type = if self.match_token(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        let span = Span::new(fn_tok.span.start, body.span.end);
        Ok(Function {
            name: name_tok.lexeme,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_parameters(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        if self.check(TokenKind::RightParen) {
            return Ok(params);
        }

        loop {
            if !self.check(TokenKind::Identifier) {
                return Err(self.error_at_current("parameter or ')'"));
            }
            let name_tok = self.advance();
            self.expect(TokenKind::Colon, "':'")?;
            let ty = self.parse_type()?;
            let span = Span::new(name_tok.span.start, ty.span.end);
            params.push(Param {
                name: name_tok.lexeme,
                ty,
                span,
            });

            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        Ok(params)
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let tok = self.expect(TokenKind::Identifier, "type name")?;
        Ok(Type {
            name: tok.lexeme,
            span: tok.span,
        })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let open = self.expect(TokenKind::LeftBrace, "'{'")?;
        let mut statements = Vec::new();

        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize_to_statement();
                }
            }
        }

        let close = self.expect(TokenKind::RightBrace, "'}'")?;
        let span = Span::new(open.span.start, close.span.end);
        Ok(Block { statements, span })
    }

    // --- statements ---

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_let_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
        let let_tok = self.advance();
        let name_tok = self.expect(TokenKind::Identifier, "identifier")?;
        let ty = if self.match_token(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Equal, "'='")?;
        let value = self.parse_expression()?;
        let semi = self.expect(TokenKind::Semicolon, "';'")?;
        let span = Span::new(let_tok.span.start, semi.span.end);
        Ok(Stmt::Let {
            name: name_tok.lexeme,
            ty,
            value,
            span,
        })
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, ParseError> {
        let return_tok = self.advance();
        let value = if self.check(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        let semi = self.expect(TokenKind::Semicolon, "';'")?;
        let span = Span::new(return_tok.span.start, semi.span.end);
        Ok(Stmt::Return { value, span })
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, ParseError> {
        let if_tok = self.advance();
        let condition = self.parse_expression()?;
        let then_block = self.parse_block()?;
        let mut end = then_block.span.end;

        let else_branch = if self.match_token(TokenKind::Else) {
            if self.check(TokenKind::If) {
                let nested = self.parse_if_statement()?;
                end = nested.span().end;
                Some(ElseBranch::If(Box::new(nested)))
            } else {
                let block = self.parse_block()?;
                end = block.span.end;
                Some(ElseBranch::Block(block))
            }
        } else {
            None
        };

        let span = Span::new(if_tok.span.start, end);
        Ok(Stmt::If {
            condition,
            then_block,
            else_branch,
            span,
        })
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, ParseError> {
        let while_tok = self.advance();
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        let span = Span::new(while_tok.span.start, body.span.end);
        Ok(Stmt::While {
            condition,
            body,
            span,
        })
    }

    fn parse_expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.parse_expression()?;
        let semi = self.expect(TokenKind::Semicolon, "';'")?;
        let span = Span::new(expr.span().start, semi.span.end);
        Ok(Stmt::Expression { expr, span })
    }

    // --- expressions, by ascending precedence ---
    //
    // expression  ::= assignment
    // assignment  ::= identifier "=" assignment | equality
    // equality    ::= comparison (("==" | "!=") comparison)*
    // comparison  ::= term (("<" | ">" | "<=" | ">=") term)*
    // term        ::= factor (("+" | "-") factor)*
    // factor      ::= unary (("*" | "/") unary)*
    // unary       ::= ("-" | "!") unary | primary
    // primary     ::= INTEGER | STRING | "true" | "false" | IDENTIFIER
    //               | "(" expression ")"

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_equality()?;

        if self.match_token(TokenKind::Equal) {
            let equals_span = self.previous().span;
            return match expr {
                Expr::Identifier(name, name_span) => {
                    let value = self.parse_assignment()?;
                    let span = Span::new(name_span.start, value.span().end);
                    Ok(Expr::Assign {
                        name,
                        value: Box::new(value),
                        span,
                    })
                }
                _ => Err(ParseError::new(
                    ParseErrorKind::InvalidAssignmentTarget,
                    equals_span,
                )),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = if self.match_token(TokenKind::EqualEqual) {
                BinaryOp::Eq
            } else if self.match_token(TokenKind::BangEqual) {
                BinaryOp::NotEq
            } else {
                break;
            };
            let right = self.parse_comparison()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;
        loop {
            let op = if self.match_token(TokenKind::Less) {
                BinaryOp::Lt
            } else if self.match_token(TokenKind::LessEqual) {
                BinaryOp::Le
            } else if self.match_token(TokenKind::Greater) {
                BinaryOp::Gt
            } else if self.match_token(TokenKind::GreaterEqual) {
                BinaryOp::Ge
            } else {
                break;
            };
            let right = self.parse_term()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = if self.match_token(TokenKind::Plus) {
                BinaryOp::Add
            } else if self.match_token(TokenKind::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };
            let right = self.parse_factor()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.match_token(TokenKind::Star) {
                BinaryOp::Mul
            } else if self.match_token(TokenKind::Slash) {
                BinaryOp::Div
            } else {
                break;
            };
            let right = self.parse_unary()?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        let op = if self.check(TokenKind::Minus) {
            Some(UnaryOp::Neg)
        } else if self.check(TokenKind::Bang) {
            Some(UnaryOp::Not)
        } else {
            None
        };

        if let Some(op) = op {
            let op_span = self.advance().span;
            let operand = self.parse_unary()?;
            let span = Span::new(op_span.start, operand.span().end);
            return Ok(Expr::Unary {
                op,
                operand: Box::new(operand),
                span,
            });
        }

        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Integer => {
                self.advance();
                Ok(Expr::Integer(token.lexeme, token.span))
            }
            TokenKind::String => {
                self.advance();
                Ok(Expr::String(token.lexeme, token.span))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true, token.span))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false, token.span))
            }
            TokenKind::Identifier => {
                self.advance();
                Ok(Expr::Identifier(token.lexeme, token.span))
            }
            TokenKind::LeftParen => {
                self.advance();
                let inner = self.parse_expression()?;
                let close = self.expect(TokenKind::RightParen, "')'")?;
                let span = Span::new(token.span.start, close.span.end);
                Ok(Expr::Grouping(Box::new(inner), span))
            }
            _ => Err(self.error_at_current("expression")),
        }
    }
}
