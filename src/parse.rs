use std::str::FromStr;
use thiserror::Error;

pub struct ParsedFunction(ExpressionNode);

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid token found in expression at {}", .0.failure_idx)]
    TokenizerError(#[from] TokenizerError),
}

impl FromStr for ParsedFunction {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens = tokenize(s)?;
    }
}

enum ExpressionNode {
    Literal(f32),
    Variable(String),
    Pair(BinaryOp, Box<ExpressionNode>, Box<ExpressionNode>),
    Unary(UnaryOp, Box<ExpressionNode>),
}

#[derive(Clone, Copy)]
enum SupportedFunction {
    Sin,
    Exp,
    Ln,
    Log10,
    Sqrt,
}

fn shunting_yard(tokens: Vec<InfixToken>) {
    todo!()
}

enum RPNToken {
    UnaryOperator(UnaryOp),
    BinaryOperator(BinaryOp),
    FunctionName(String),
    VariableName(String),
    Literal(f32),
}

enum UnaryOp {
    Negate,
}
enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

#[derive(Clone, Copy)]
enum InfixTokenOperator {
    Add,
    SubtractOrNegate,
    Multiply,
    Divide,
    Power,
}

enum InfixToken {
    ParenOpen,
    ParenClose,
    Function(SupportedFunction),
    Variable(char),
    Operator(InfixTokenOperator),
    Literal(f32),
}

fn get_func(input: &str) -> Option<(SupportedFunction, usize)> {
    const FUNC_NAMES: &[(&str, SupportedFunction)] = &[
        ("sin", SupportedFunction::Sin),
        ("exp", SupportedFunction::Exp),
        ("ln", SupportedFunction::Ln),
        ("log10", SupportedFunction::Log10),
        ("sqrt", SupportedFunction::Sqrt),
    ];

    for (name, func) in FUNC_NAMES {
        if input.starts_with(name) {
            return Some((*func, name.len()));
        }
    }
    None
}

#[derive(Debug, Error)]
pub struct TokenizerError {
    pub failure_idx: usize,
}

impl std::fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to generate tokens at character {}",
            self.failure_idx
        )
    }
}

fn tokenize(expression: &str) -> Result<Vec<InfixToken>, TokenizerError> {
    const TOKEN_OPS: &[(char, InfixTokenOperator)] = &[
        ('+', InfixTokenOperator::Add),
        ('-', InfixTokenOperator::SubtractOrNegate),
        ('*', InfixTokenOperator::Multiply),
        ('/', InfixTokenOperator::Divide),
        ('^', InfixTokenOperator::Power),
    ];

    let expression = expression
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    let mut tokens = Vec::new();

    let mut at: usize = 0;
    while at < expression.len() {
        if let Some((func, len)) = get_func(&expression[at..]) {
            at += len;
            tokens.push(InfixToken::Function(func));
        } else if expression
            .chars()
            .nth(at)
            .is_some_and(|c| c.is_alphabetic())
        {
            tokens.push(InfixToken::Variable(
                expression.chars().nth(at).unwrap(),
            ));
            at += 1;
        } else if let Some((num, len)) = read_literal(&expression[at..]) {
            tokens.push(InfixToken::Literal(num));
            at += len;
        } else if let Some(op) = expression
            .chars()
            .next()
            .map(|c| TOKEN_OPS.iter().find(|&i| i.0 == c).map(|v| v.1))
            .flatten()
        {
            tokens.push(InfixToken::Operator(op));
            at += 1;
        } else if let Some('(') = expression.chars().next() {
            tokens.push(InfixToken::ParenOpen);
            at += 1;
        } else if let Some(')') = expression.chars().next() {
            tokens.push(InfixToken::ParenClose);
            at += 1;
        } else {
            return Err(TokenizerError { failure_idx: at });
        }
    }

    Ok(tokens)
}

fn read_literal(input: &str) -> Option<(f32, usize)> {
    if !input.chars().next().is_some_and(|c| c.is_numeric()) {
        return None;
    }
    let strnum = input
        .chars()
        .scan(false, |p, c| {
            if c.is_numeric() {
                Some(c)
            } else if c == '.' && !*p {
                *p = true;
                Some(c)
            } else {
                None
            }
        })
        .collect::<String>();
    strnum.parse::<f32>().ok().map(|v| (v, strnum.len()))
}
