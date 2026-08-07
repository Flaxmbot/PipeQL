use std::fmt;

use crate::ast::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    From,
    Filter,
    Select,
    Derive,
    Join,
    Group,
    Sort,
    Take,
    Skip,
    Into,
    Insert,
    Update,
    Delete,
    Table,
    As,
    On,
    And,
    Or,
    Not,
    In,
    Is,
    Null,
    True,
    False,
    Left,
    Right,
    Full,
    Inner,
    Asc,
    Desc,
    Upsert,
    Conflict,
    Do,
    Union,
    All,

    // Literals
    Integer(i64),
    Float(f64),
    String(String),

    // Identifiers
    Ident(String),

    // Parameters
    Param(String),
    ParamBraced(String),

    // Operators
    Pipe,     // |
    Eq,       // ==
    NotEq,    // !=
    Lt,       // <
    LtEq,     // <=
    Gt,       // >
    GtEq,     // >=
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Assign,   // =
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    Comma,    // ,
    Dot,      // .

    // Special
    Comment(String),
    Newline,
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::From => write!(f, "from"),
            TokenKind::Filter => write!(f, "filter"),
            TokenKind::Select => write!(f, "select"),
            TokenKind::Derive => write!(f, "derive"),
            TokenKind::Join => write!(f, "join"),
            TokenKind::Group => write!(f, "group"),
            TokenKind::Sort => write!(f, "sort"),
            TokenKind::Take => write!(f, "take"),
            TokenKind::Skip => write!(f, "skip"),
            TokenKind::Into => write!(f, "into"),
            TokenKind::Insert => write!(f, "insert"),
            TokenKind::Update => write!(f, "update"),
            TokenKind::Delete => write!(f, "delete"),
            TokenKind::Table => write!(f, "table"),
            TokenKind::As => write!(f, "as"),
            TokenKind::On => write!(f, "on"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Is => write!(f, "is"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Left => write!(f, "left"),
            TokenKind::Right => write!(f, "right"),
            TokenKind::Full => write!(f, "full"),
            TokenKind::Inner => write!(f, "inner"),
            TokenKind::Asc => write!(f, "asc"),
            TokenKind::Desc => write!(f, "desc"),
            TokenKind::Upsert => write!(f, "upsert"),
            TokenKind::Conflict => write!(f, "conflict"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::Union => write!(f, "union"),
            TokenKind::All => write!(f, "all"),
            TokenKind::Integer(v) => write!(f, "{v}"),
            TokenKind::Float(v) => write!(f, "{v}"),
            TokenKind::String(v) => write!(f, "'{v}'"),
            TokenKind::Ident(v) => write!(f, "{v}"),
            TokenKind::Param(v) => write!(f, "${v}"),
            TokenKind::ParamBraced(v) => write!(f, "${{{v}}}"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Eq => write!(f, "=="),
            TokenKind::NotEq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Assign => write!(f, "="),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Comment(v) => write!(f, "--{v}"),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexerError {
    pub message: String,
    pub span: Span,
    pub suggestion: Option<String>,
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Lexer error at {}..{}: {}",
            self.span.start, self.span.end, self.message
        )
    }
}

impl std::error::Error for LexerError {}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.remaining().chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() && ch != '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_line_comment(&mut self) -> TokenKind {
        let mut text = String::new();
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            text.push(ch);
            self.advance();
        }
        TokenKind::Comment(text.trim().to_string())
    }

    fn read_string(&mut self, start: usize) -> Result<TokenKind, LexerError> {
        let mut value = String::new();
        loop {
            match self.advance() {
                Some('\'') => {
                    // Check for escaped quote ''
                    if self.peek() == Some('\'') {
                        value.push('\'');
                        self.advance();
                    } else {
                        return Ok(TokenKind::String(value));
                    }
                    continue;
                }
                Some(ch) => value.push(ch),
                None => {
                    return Err(LexerError {
                        message: "Unterminated string literal".to_string(),
                        span: Span::new(start, self.pos),
                        suggestion: Some(
                            "Did you forget to close the string with a `'`?".to_string(),
                        ),
                    })
                }
            }
        }
    }

    fn read_number(&mut self, _start: usize, first: char) -> TokenKind {
        let mut s = String::new();
        s.push(first);
        let mut is_float = false;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                s.push(ch);
                self.advance();
            } else if ch == '.' && !is_float {
                is_float = true;
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if is_float {
            TokenKind::Float(s.parse().unwrap_or(0.0))
        } else {
            TokenKind::Integer(s.parse().unwrap_or(0))
        }
    }

    fn read_identifier(&mut self, first: char) -> String {
        let mut s = String::new();
        s.push(first);
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    fn keyword_or_ident(s: &str) -> TokenKind {
        match s.to_lowercase().as_str() {
            "from" => TokenKind::From,
            "filter" => TokenKind::Filter,
            "select" => TokenKind::Select,
            "derive" => TokenKind::Derive,
            "join" => TokenKind::Join,
            "group" => TokenKind::Group,
            "sort" => TokenKind::Sort,
            "take" => TokenKind::Take,
            "skip" => TokenKind::Skip,
            "into" => TokenKind::Into,
            "insert" => TokenKind::Insert,
            "update" => TokenKind::Update,
            "delete" => TokenKind::Delete,
            "table" => TokenKind::Table,
            "as" => TokenKind::As,
            "on" => TokenKind::On,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "in" => TokenKind::In,
            "is" => TokenKind::Is,
            "null" => TokenKind::Null,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "left" => TokenKind::Left,
            "right" => TokenKind::Right,
            "full" => TokenKind::Full,
            "inner" => TokenKind::Inner,
            "asc" => TokenKind::Asc,
            "desc" => TokenKind::Desc,
            "upsert" => TokenKind::Upsert,
            "conflict" => TokenKind::Conflict,
            "do" => TokenKind::Do,
            "union" => TokenKind::Union,
            "all" => TokenKind::All,
            _ => TokenKind::Ident(s.to_string()),
        }
    }

    fn read_param(&mut self, start: usize) -> Result<TokenKind, LexerError> {
        match self.peek() {
            Some('{') => {
                self.advance(); // skip {
                let mut name = String::new();
                loop {
                    match self.advance() {
                        Some('}') => {
                            return Ok(TokenKind::ParamBraced(name));
                        }
                        Some(ch) if ch.is_alphanumeric() || ch == '_' => {
                            name.push(ch);
                        }
                        Some(ch) => {
                            return Err(LexerError {
                                message: format!("Invalid character '{ch}' in parameter name"),
                                span: Span::new(start, self.pos),
                                suggestion: Some(
                                    "Parameter names may contain letters, digits, and underscores"
                                        .to_string(),
                                ),
                            });
                        }
                        None => {
                            return Err(LexerError {
                                message: "Unterminated parameter".to_string(),
                                span: Span::new(start, self.pos),
                                suggestion: Some("Did you forget to close the parameter with `}`? e.g. `${name}`".to_string()),
                            });
                        }
                    }
                }
            }
            Some(ch) if ch.is_alphanumeric() || ch == '_' => {
                self.advance(); // consume the first character
                let name = self.read_identifier(ch);
                Ok(TokenKind::Param(name))
            }
            Some(ch) => Err(LexerError {
                message: format!("Expected parameter name after '$', found '{ch}'"),
                span: Span::new(start, self.pos),
                suggestion: Some("Parameters use the form `$name` or `${name}`".to_string()),
            }),
            None => Err(LexerError {
                message: "Unexpected end of input after '$'".to_string(),
                span: Span::new(start, self.pos),
                suggestion: Some("Parameters use the form `$name` or `${name}`".to_string()),
            }),
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, Vec<LexerError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        loop {
            self.skip_whitespace();

            let start = self.pos;

            if self.pos >= self.input.len() {
                tokens.push(Token::new(TokenKind::Eof, Span::new(start, start)));
                break;
            }

            let ch = match self.peek() {
                Some(ch) => ch,
                None => {
                    tokens.push(Token::new(TokenKind::Eof, Span::new(start, start)));
                    break;
                }
            };

            let kind = match ch {
                '\n' => {
                    self.advance();
                    // Collapse consecutive newlines
                    if tokens
                        .last()
                        .is_none_or(|t: &Token| t.kind != TokenKind::Newline)
                    {
                        TokenKind::Newline
                    } else {
                        continue;
                    }
                }
                '|' => {
                    self.advance();
                    TokenKind::Pipe
                }
                '(' => {
                    self.advance();
                    TokenKind::LParen
                }
                ')' => {
                    self.advance();
                    TokenKind::RParen
                }
                '[' => {
                    self.advance();
                    TokenKind::LBracket
                }
                ']' => {
                    self.advance();
                    TokenKind::RBracket
                }
                ',' => {
                    self.advance();
                    TokenKind::Comma
                }
                '.' => {
                    self.advance();
                    TokenKind::Dot
                }
                '+' => {
                    self.advance();
                    TokenKind::Plus
                }
                '-' => {
                    self.advance();
                    // Check for line comment --
                    if self.peek() == Some('-') {
                        self.advance();
                        self.read_line_comment()
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.advance();
                    TokenKind::Star
                }
                '/' => {
                    self.advance();
                    TokenKind::Slash
                }
                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::Eq
                    } else {
                        TokenKind::Assign
                    }
                }
                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::NotEq
                    } else {
                        errors.push(LexerError {
                            message: "Unexpected character '!'".to_string(),
                            span: Span::new(start, self.pos),
                            suggestion: Some("Use `!=` for not-equal".to_string()),
                        });
                        continue;
                    }
                }
                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::LtEq
                    } else if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }
                '\'' => {
                    self.advance();
                    match self.read_string(start) {
                        Ok(kind) => kind,
                        Err(e) => {
                            errors.push(e);
                            continue;
                        }
                    }
                }
                '$' => {
                    self.advance();
                    match self.read_param(start) {
                        Ok(kind) => kind,
                        Err(e) => {
                            errors.push(e);
                            continue;
                        }
                    }
                }
                ch if ch.is_ascii_digit() => {
                    self.advance(); // consume the first digit
                    self.read_number(start, ch)
                }
                ch if ch.is_alphanumeric() || ch == '_' => {
                    self.advance(); // consume the first character
                    let ident = self.read_identifier(ch);
                    Self::keyword_or_ident(&ident)
                }
                ch => {
                    self.advance();
                    errors.push(LexerError {
                        message: format!("Unexpected character '{ch}'"),
                        span: Span::new(start, self.pos),
                        suggestion: Some("Check the PipeQL grammar; only `|`, `==`, `!=`, `<>`, `<=`, `>=`, `<`, `>`, `+`, `-`, `*`, `/`, `=`, `(`, `)`, `[`, `]`, `,`, `.`, `$`, and `'` are operators".to_string()),
                    });
                    continue;
                }
            };

            tokens.push(Token::new(kind, Span::new(start, self.pos)));
        }

        if errors.is_empty() {
            Ok(tokens)
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("from users | filter age > 18");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::From);
        assert_eq!(tokens[1].kind, TokenKind::Ident("users".into()));
        assert_eq!(tokens[2].kind, TokenKind::Pipe);
        assert_eq!(tokens[3].kind, TokenKind::Filter);
        assert_eq!(tokens[4].kind, TokenKind::Ident("age".into()));
        assert_eq!(tokens[5].kind, TokenKind::Gt);
        assert_eq!(tokens[6].kind, TokenKind::Integer(18));
    }

    #[test]
    fn test_string_literal() {
        let mut lexer = Lexer::new("from users | filter name == 'Alice'");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[6].kind, TokenKind::String("Alice".into()));
    }

    #[test]
    fn test_escaped_string() {
        let mut lexer = Lexer::new("from t | filter x == '''hello'''");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[6].kind, TokenKind::String("'hello'".into()));
    }

    #[test]
    fn test_parameters() {
        let mut lexer = Lexer::new("from users | filter id == $user_id and name == ${full_name}");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[6].kind, TokenKind::Param("user_id".into()));
        assert_eq!(tokens[10].kind, TokenKind::ParamBraced("full_name".into()));
    }

    #[test]
    fn test_keywords_case_insensitive() {
        let mut lexer = Lexer::new("FROM USERS | FILTER age > 18");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::From);
        assert_eq!(tokens[2].kind, TokenKind::Pipe);
        assert_eq!(tokens[3].kind, TokenKind::Filter);
    }

    #[test]
    fn test_mutation_keywords() {
        let mut lexer = Lexer::new(
            "into notes | insert [title = $t] | update [x = 1] | delete | table t [id int]",
        );
        let tokens = lexer.tokenize().unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
        assert!(kinds.contains(&TokenKind::Into));
        assert!(kinds.contains(&TokenKind::Insert));
        assert!(kinds.contains(&TokenKind::Update));
        assert!(kinds.contains(&TokenKind::Delete));
        assert!(kinds.contains(&TokenKind::Table));
    }

    #[test]
    fn test_newlines_as_pipe() {
        let mut lexer = Lexer::new("from users\nfilter age > 18\nselect [id]");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::From);
        assert_eq!(tokens[2].kind, TokenKind::Newline);
        assert_eq!(tokens[3].kind, TokenKind::Filter);
    }

    #[test]
    fn test_line_comment() {
        let mut lexer = Lexer::new("from users -- this is a comment\n| filter age > 18");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::From);
        assert_eq!(tokens[1].kind, TokenKind::Ident("users".into()));
        // The comment is preserved as a Comment token with its text.
        assert_eq!(
            tokens[2].kind,
            TokenKind::Comment("this is a comment".into())
        );
        // After the comment, the newline becomes a Newline token (pipe separator)
        assert_eq!(tokens[3].kind, TokenKind::Newline);
        assert_eq!(tokens[4].kind, TokenKind::Pipe);
    }

    #[test]
    fn test_float_literal() {
        let mut lexer = Lexer::new("from t | filter x == 3.25");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[6].kind, TokenKind::Float(3.25));
    }

    #[test]
    fn test_span_tracking() {
        let mut lexer = Lexer::new("from users");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].span, Span::new(0, 4));
        assert_eq!(tokens[1].span, Span::new(5, 10));
    }
}
