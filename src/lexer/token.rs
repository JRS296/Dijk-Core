use std::fmt;

/// A byte range `[start, end)` into the source text a token was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Identifier,
    Integer,
    String,

    // Keywords
    Fn,
    Let,
    Mut,
    If,
    Else,
    While,
    For,
    Return,
    Struct,
    Enum,
    True,
    False,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Amp,
    Pipe,

    // Punctuation
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    Semicolon,
    Dot,
    Arrow,

    Eof,
}

impl TokenKind {
    /// The keyword this identifier spells, if it is one.
    pub fn keyword_from_str(text: &str) -> Option<TokenKind> {
        use TokenKind::*;
        Some(match text {
            "fn" => Fn,
            "let" => Let,
            "mut" => Mut,
            "if" => If,
            "else" => Else,
            "while" => While,
            "for" => For,
            "return" => Return,
            "struct" => Struct,
            "enum" => Enum,
            "true" => True,
            "false" => False,
            _ => return None,
        })
    }

    fn name(&self) -> &'static str {
        use TokenKind::*;
        match self {
            Identifier => "IDENTIFIER",
            Integer => "INTEGER",
            String => "STRING",
            Fn => "FN",
            Let => "LET",
            Mut => "MUT",
            If => "IF",
            Else => "ELSE",
            While => "WHILE",
            For => "FOR",
            Return => "RETURN",
            Struct => "STRUCT",
            Enum => "ENUM",
            True => "TRUE",
            False => "FALSE",
            Plus => "PLUS",
            Minus => "MINUS",
            Star => "STAR",
            Slash => "SLASH",
            Percent => "PERCENT",
            Equal => "EQUAL",
            EqualEqual => "EQUALEQUAL",
            Bang => "BANG",
            BangEqual => "BANGEQUAL",
            Less => "LESS",
            LessEqual => "LESSEQUAL",
            Greater => "GREATER",
            GreaterEqual => "GREATEREQUAL",
            Amp => "AMP",
            Pipe => "PIPE",
            LeftParen => "LPAREN",
            RightParen => "RPAREN",
            LeftBrace => "LBRACE",
            RightBrace => "RBRACE",
            LeftBracket => "LBRACKET",
            RightBracket => "RBRACKET",
            Comma => "COMMA",
            Colon => "COLON",
            Semicolon => "SEMICOLON",
            Dot => "DOT",
            Arrow => "ARROW",
            Eof => "EOF",
        }
    }

    /// Whether this kind's printed form carries its lexeme, e.g. `IDENTIFIER("x")`.
    fn carries_lexeme(&self) -> bool {
        matches!(
            self,
            TokenKind::Identifier | TokenKind::Integer | TokenKind::String
        )
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            lexeme: lexeme.into(),
            span,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.kind.carries_lexeme() {
            write!(f, "{}({:?})", self.kind, self.lexeme)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}
