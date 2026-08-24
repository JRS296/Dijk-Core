use super::error::{LexError, LexErrorKind};
use super::token::{Span, Token, TokenKind};

/// Turns Dijk source text into a stream of tokens.
///
/// The lexer only recognizes *what* the pieces of the source are — it does
/// not know how they fit together (the parser's job) or what they mean
/// (semantic analysis's job). See `docs/lexer.md`.
/// Operates on raw bytes rather than decoded `char`s: Dijk Core's initial
/// lexical grammar (see `docs/lexer.md`) is ASCII-only, so a byte offset
/// doubles as a source position without needing UTF-8 decoding.
pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source, bytes: source.as_bytes(), pos: 0 }
    }

    /// Lex the entire source, returning every token (always ending with
    /// `Eof`) alongside any lexical errors encountered along the way.
    /// Lexing continues past an error so more than one can be reported.
    pub fn tokenize(mut self) -> (Vec<Token>, Vec<LexError>) {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        loop {
            self.skip_whitespace_and_comments();

            let start = self.pos;
            let Some(c) = self.peek() else {
                tokens.push(Token::new(TokenKind::Eof, "", Span::new(start, start)));
                break;
            };

            match self.next_token(c, start) {
                Ok(token) => tokens.push(token),
                Err(error) => errors.push(error),
            }
        }

        (tokens, errors)
    }

    fn next_token(&mut self, c: char, start: usize) -> Result<Token, LexError> {
        if is_identifier_start(c) {
            return Ok(self.read_identifier(start));
        }
        if c.is_ascii_digit() {
            return Ok(self.read_integer(start));
        }
        if c == '"' {
            return self.read_string(start);
        }

        self.advance();
        let kind = match c {
            '+' => TokenKind::Plus,
            '-' => {
                if self.consume('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '=' => {
                if self.consume('=') {
                    TokenKind::EqualEqual
                } else {
                    TokenKind::Equal
                }
            }
            '!' => {
                if self.consume('=') {
                    TokenKind::BangEqual
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                if self.consume('=') {
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                if self.consume('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            '&' => TokenKind::Amp,
            '|' => TokenKind::Pipe,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            ';' => TokenKind::Semicolon,
            '.' => TokenKind::Dot,
            other => {
                return Err(LexError::new(
                    LexErrorKind::UnexpectedCharacter(other),
                    Span::new(start, self.pos),
                ));
            }
        };

        Ok(Token::new(kind, &self.source[start..self.pos], Span::new(start, self.pos)))
    }

    fn read_identifier(&mut self, start: usize) -> Token {
        while self.peek().is_some_and(is_identifier_continue) {
            self.advance();
        }

        let lexeme = &self.source[start..self.pos];
        let kind = TokenKind::keyword_from_str(lexeme).unwrap_or(TokenKind::Identifier);
        Token::new(kind, lexeme, Span::new(start, self.pos))
    }

    fn read_integer(&mut self, start: usize) -> Token {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }

        let lexeme = &self.source[start..self.pos];
        Token::new(TokenKind::Integer, lexeme, Span::new(start, self.pos))
    }

    fn read_string(&mut self, start: usize) -> Result<Token, LexError> {
        self.advance(); // opening quote

        let content_start = self.pos;
        loop {
            match self.peek() {
                None | Some('\n') => {
                    return Err(LexError::new(
                        LexErrorKind::UnterminatedString,
                        Span::new(start, self.pos),
                    ));
                }
                Some('"') => break,
                Some(_) => {
                    self.advance();
                }
            }
        }

        let lexeme = self.source[content_start..self.pos].to_string();
        self.advance(); // closing quote
        Ok(Token::new(TokenKind::String, lexeme, Span::new(start, self.pos)))
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.peek_at(0)
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.bytes.get(self.pos + offset).map(|&b| b as char)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    /// If the current character matches `expected`, consume it and return true.
    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn tokenize(source: &str) -> (Vec<Token>, Vec<LexError>) {
    Lexer::new(source).tokenize()
}
