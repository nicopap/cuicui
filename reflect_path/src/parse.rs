//! Vendoring of bevy's `ParsedPath` implementation.
//!
//! We need access to individual components
use std::iter;

use crate::{Access, Path};

enum Token<'a> {
    Dot,
    CrossHatch,
    OpenBracket,
    CloseBracket,
    Ident(&'a str),
}

impl<'a> Token<'a> {
    const DOT: char = '.';
    const CROSSHATCH: char = '#';
    const OPEN_BRACKET: char = '[';
    const CLOSE_BRACKET: char = ']';
    const OPEN_BRACKET_STR: &'static str = "[";
    const CLOSE_BRACKET_STR: &'static str = "]";
}

/// # Panics
///
/// If `path` contains a field index accessor, such as `#0`.
///
/// # Safety
///
/// `path` must be a valid path according to bevy's implementation.
pub(crate) unsafe fn parse_path<'a>(path: &'a str) -> Path<'a> {
    let mut parser = PathParser::new(path);

    let iter = iter::from_fn(|| {
        let token = parser.next_token()?;
        Some(parser.token_to_access(token))
    });
    Path(iter.collect())
}

/// # Safety
///
/// `token` must be `Some(Token::Ident(v))`.
unsafe fn ident_unchecked(token: Option<Token>) -> &str {
    match token {
        Some(Token::Ident(value)) => value,
        // SAFETY: upheld by function invariant
        _ => unsafe { std::hint::unreachable_unchecked() },
    }
}
struct PathParser<'a> {
    path: &'a str,
}

impl<'a> PathParser<'a> {
    /// # Panics
    ///
    /// If `path` contains a field index accessor, such as `#0`.
    ///
    /// # Safety
    ///
    /// `path` must be a valid path according to bevy's implementation.
    unsafe fn new(path: &'a str) -> Self {
        assert!(!path.contains(Token::CROSSHATCH));
        Self { path }
    }

    fn next_token(&mut self) -> Option<Token<'a>> {
        match self.path.chars().next() {
            None => return None,
            Some(Token::DOT) => {
                self.path = &self.path[1..];
                return Some(Token::Dot);
            }
            Some(Token::OPEN_BRACKET) => {
                self.path = &self.path[1..];
                return Some(Token::OpenBracket);
            }
            Some(Token::CLOSE_BRACKET) => {
                self.path = &self.path[1..];
                return Some(Token::CloseBracket);
            }
            Some(_) => {}
        }

        // we can assume we are parsing an ident now
        for (char_index, character) in self.path.chars().enumerate() {
            let is_terminal = matches!(
                character,
                Token::DOT | Token::OPEN_BRACKET | Token::CLOSE_BRACKET
            );
            if is_terminal {
                let ident = Token::Ident(&self.path[..char_index]);
                self.path = &self.path[char_index..];
                return Some(ident);
            }
        }
        let ident = Token::Ident(self.path);
        self.path = &self.path[self.path.len()..];
        Some(ident)
    }

    fn token_to_access(&mut self, token: Token<'a>) -> Access<'a> {
        match token {
            Token::Dot => {
                let value = unsafe { ident_unchecked(self.next_token()) };
                value
                    .parse::<usize>()
                    .map(Access::TupleIndex)
                    .unwrap_or(Access::Field(value))
            }
            Token::OpenBracket => {
                let value = unsafe { ident_unchecked(self.next_token()) };
                self.next_token();
                let parsed = unsafe { value.parse::<usize>().unwrap_unchecked() };
                Access::ArrayIndex(parsed)
            }
            Token::Ident(value) => value
                .parse::<usize>()
                .map(Access::TupleIndex)
                .unwrap_or(Access::Field(value)),
            _ => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}
