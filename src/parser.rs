// Module: parser
//
// This module provides functionality to convert infix mathematical expressions
// (using single-digit numbers and the operators '+' and '-') into postfix (Reverse Polish Notation) expressions.
// It uses a recursive descent parser approach, following a simple context-free grammar.
//
// The main entry point is the `infix_to_postfix` function.

/// Error type for parsing failures.
#[derive(Debug)]
pub enum ParseError {
    /// The input contained an unexpected character.
    UnexpectedChar(char),
    /// The input ended unexpectedly.
    UnexpectedEnd,
}

/// Converts an infix expression (e.g., "2+3-4") to postfix (e.g., "23+4-").
///
/// Arguments:
///     expr: &str - The infix expression to convert. Only digits and '+'/'-' are allowed.
///
/// Returns:
///     Result<String, ParseError> - Ok(postfix) if successful, Err(ParseError) otherwise.
///
/// Example:
///     let result = infix_to_postfix("2+3-4");
///     assert_eq!(result.unwrap(), "23+4-");
pub fn infix_to_postfix(expr: &str) -> Result<String, ParseError> {
    let mut parser = Parser::new(expr);
    let mut output = String::new();
    parser.parse_expr(&mut output)?;
    // If there are leftover characters, it's an error.
    if parser.peek().is_some() {
        return Err(ParseError::UnexpectedChar(parser.peek().unwrap()));
    }
    Ok(output)
}

// Internal parser struct for recursive descent parsing.
struct Parser<'a> {
    chars: std::str::Chars<'a>,
    lookahead: Option<char>,
}

impl<'a> Parser<'a> {
    /// Creates a new parser for the given input string.
    fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let lookahead = chars.next();
        Self { chars, lookahead }
    }

    /// Returns the current lookahead character without consuming it.
    fn peek(&self) -> Option<char> {
        self.lookahead
    }

    /// Consumes and returns the current lookahead character, advancing to the next.
    fn next(&mut self) -> Option<char> {
        let current = self.lookahead;
        self.lookahead = self.chars.next();
        current
    }

    /// Parses an expression according to the grammar:
    /// exp -> term (('+' | '-') term)*
    fn parse_expr(&mut self, output: &mut String) -> Result<(), ParseError> {
        self.parse_term(output)?;
        while let Some(op) = self.peek() {
            if op == '+' || op == '-' {
                let op = self.next().unwrap(); // Safe: peeked above
                self.parse_term(output)?;
                output.push(op);
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Parses a term (a single digit).
    fn parse_term(&mut self, output: &mut String) -> Result<(), ParseError> {
        match self.next() {
            Some(ch) if ch.is_ascii_digit() => {
                output.push(ch);
                Ok(())
            }
            Some(ch) => Err(ParseError::UnexpectedChar(ch)),
            None => Err(ParseError::UnexpectedEnd),
        }
    }
}
