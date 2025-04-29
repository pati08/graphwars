use std::str::FromStr;
use thiserror::Error;

#[derive(Clone)]
pub struct ParsedFunction {
    tree: ExpressionNode,
    bound_vars: Vec<(String, f32)>,
}

impl ParsedFunction {
    pub fn add_var(&mut self, var: impl ToString, val: f32) {
        let var = var.to_string();
        let binding = (var, val);
        if self.bound_vars.contains(&binding) {
            return;
        }
        self.bound_vars.push(binding);
    }
    pub fn bind<T: ToString + Send + Sync>(
        &self,
        var: T,
    ) -> impl Fn(f32) -> Result<f32, EvalError> + Send + Sync + use<T> {
        let vars = self.bound_vars.clone();
        let tree = self.tree.clone();
        move |v: f32| {
            tree.eval(
                &vars
                    .iter()
                    .map(|i| i.to_owned())
                    .chain(std::iter::once((var.to_string(), v)))
                    .collect::<Box<[_]>>(),
            )
        }
    }
}

fn build_expression_tree(
    rpn_tokens: Vec<RPNToken>,
) -> Result<ExpressionNode, TreeBuildError> {
    let mut stack: Vec<ExpressionNode> = Vec::new();
    for token in rpn_tokens {
        let new = match token {
            RPNToken::Literal(num) => ExpressionNode::Literal(num),
            RPNToken::Variable(var) => ExpressionNode::Variable(var),
            RPNToken::Function(func) => ExpressionNode::Function(
                func,
                Box::new(
                    stack.pop().ok_or(TreeBuildError::MissingFunctionArg)?,
                ),
            ),
            RPNToken::ExpressionOp(op) => {
                let right = Box::new(
                    stack.pop().ok_or(TreeBuildError::MissingRightOperand)?,
                );
                let left = Box::new(
                    stack.pop().ok_or(TreeBuildError::MissingLeftOperand)?,
                );
                ExpressionNode::Operation(op, left, right)
            }
        };
        stack.push(new);
    }
    if stack.len() > 1 {
        return Err(TreeBuildError::RemainingNodes);
    }
    if stack.is_empty() {
        return Err(TreeBuildError::EmptyExpression);
    }
    Ok(stack[0].clone())
}

impl FromStr for ParsedFunction {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens = tokenize(s)?;
        let rpn = shunting_yard(tokens);
        let expression_tree = build_expression_tree(rpn?)?;
        Ok(ParsedFunction {
            tree: expression_tree,
            bound_vars: Vec::new(),
        })
    }
}

// Add this to your ParseError enum
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid token found in expression at {}", .0.failure_idx)]
    Tokenizer(#[from] TokenizerError),
    #[error("Failed to build expression tree: {0}")]
    TreeBuild(#[from] TreeBuildError),
    #[error("Shunting yard failed")]
    ShuntingYard(#[from] ShuntingYardError),
}

#[derive(Debug, Error)]
pub enum TreeBuildError {
    #[error("Missing left operand for binary operator")]
    MissingLeftOperand,
    #[error("Missing right operand for binary operator")]
    MissingRightOperand,
    #[error("Missing function argument")]
    MissingFunctionArg,
    #[error("Invalid expression: multiple nodes remain on stack")]
    RemainingNodes,
    #[error("Empty expression")]
    EmptyExpression,
}

#[derive(Clone, Debug, PartialEq)]
enum ExpressionNode {
    Literal(f32),
    Variable(char),
    Operation(ExpressionOp, Box<ExpressionNode>, Box<ExpressionNode>),
    Function(SupportedFunction, Box<ExpressionNode>),
}

#[derive(Debug, Error)]
pub enum EvalError {
    #[error("Undefined variable used")]
    UndefinedVariable,
    #[error("Function failed")]
    FunctionEvalErr(#[from] FunctionEvalErr),
    #[error("Binary operator error")]
    BinaryOpErr(#[from] BinaryOpErr),
}

impl ExpressionNode {
    fn eval(&self, vars: &[(String, f32)]) -> Result<f32, EvalError> {
        match self {
            ExpressionNode::Operation(op, left, right) => {
                Ok(op.apply(left.eval(vars)?, right.eval(vars)?)?)
            }
            ExpressionNode::Literal(val) => Ok(*val),
            ExpressionNode::Variable(var) => {
                if let Some((_, val)) =
                    vars.iter().find(|i| i.0 == var.to_string())
                {
                    Ok(*val)
                } else {
                    Err(EvalError::UndefinedVariable)
                }
            }
            ExpressionNode::Function(func, arg) => {
                Ok(func.apply(arg.eval(vars)?)?)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum SupportedFunction {
    Sine,
    Exp,
    Ln,
    Log10,
    Sqrt,
}

#[derive(Debug, Error)]
pub enum FunctionEvalErr {
    #[error("Argument was not in function domain")]
    OutOfDomain,
}
impl SupportedFunction {
    fn apply(&self, arg: f32) -> Result<f32, FunctionEvalErr> {
        match self {
            Self::Sine => Ok(arg.sin()),
            Self::Exp => Ok(1. / (1. + std::f32::consts::E.powf(arg))),
            Self::Ln => {
                if arg > 0. {
                    Ok(arg.ln())
                } else {
                    Err(FunctionEvalErr::OutOfDomain)
                }
            }
            Self::Log10 => {
                if arg > 0. {
                    Ok(arg.log10())
                } else {
                    Err(FunctionEvalErr::OutOfDomain)
                }
            }
            Self::Sqrt => {
                if arg >= 0. {
                    Ok(arg.sqrt())
                } else {
                    Err(FunctionEvalErr::OutOfDomain)
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum RPNToken {
    ExpressionOp(ExpressionOp),
    Function(SupportedFunction),
    Variable(char),
    Literal(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ExpressionOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
}

#[derive(Debug, Error)]
pub enum BinaryOpErr {
    #[error("Divided by 0")]
    Div0,
}

impl ExpressionOp {
    fn apply(&self, left: f32, right: f32) -> Result<f32, BinaryOpErr> {
        match self {
            Self::Add => Ok(left + right),
            Self::Subtract => Ok(left - right),
            Self::Multiply => Ok(left * right),
            Self::Divide => {
                if right != 0. {
                    Ok(left / right)
                } else {
                    Err(BinaryOpErr::Div0)
                }
            }
            Self::Power => Ok(left.powf(right)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum InfixTokenOperator {
    Add,
    SubtractOrNegate,
    Multiply,
    Divide,
    Power,
    ImplicitMultiply,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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
        ("sin", SupportedFunction::Sine),
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
            .nth(at)
            .and_then(|c| TOKEN_OPS.iter().find(|&i| i.0 == c).map(|v| v.1))
        {
            tokens.push(InfixToken::Operator(op));
            at += 1;
        } else if let Some('(') = expression.chars().nth(at) {
            tokens.push(InfixToken::ParenOpen);
            at += 1;
        } else if let Some(')') = expression.chars().nth(at) {
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

fn get_operator_precedence(op: InfixTokenOperator) -> u8 {
    match op {
        InfixTokenOperator::Add => 1,
        InfixTokenOperator::SubtractOrNegate => 1,
        InfixTokenOperator::Multiply => 2,
        InfixTokenOperator::ImplicitMultiply => 3, // Higher than explicit multiply
        InfixTokenOperator::Divide => 2,
        InfixTokenOperator::Power => 5, // Increased to be higher than function application
    }
}

fn is_right_associative(op: InfixTokenOperator) -> bool {
    matches!(op, InfixTokenOperator::Power)
}

fn perform_unary_minus(tokens: &[InfixToken]) -> Vec<InfixToken> {
    tokens
        .iter()
        .copied()
        .scan(true, |acc, i| {
            let res = if i
                == InfixToken::Operator(InfixTokenOperator::SubtractOrNegate)
                && *acc
            {
                vec![
                    InfixToken::Literal(-1.),
                    InfixToken::Operator(InfixTokenOperator::ImplicitMultiply),
                ]
            } else {
                vec![i]
            };
            *acc = matches!(i, InfixToken::ParenOpen | InfixToken::Operator(_));
            Some(res)
        })
        .flatten()
        .collect()
}

fn insert_implicit_multiplication(tokens: &[InfixToken]) -> Vec<InfixToken> {
    let mut output = Vec::new();
    for token in tokens {
        if matches!(
            output.last(),
            Some(
                InfixToken::Variable(_)
                    | InfixToken::ParenClose
                    | InfixToken::Literal(_)
            )
        ) && matches!(
            token,
            InfixToken::Literal(_)
                | InfixToken::ParenOpen
                | InfixToken::Variable(_)
                | InfixToken::Function(_)
        ) {
            output.push(InfixToken::Operator(
                InfixTokenOperator::ImplicitMultiply,
            ));
        }
        output.push(*token);
    }
    output
}

fn shunting_yard(
    mut tokens: Vec<InfixToken>,
) -> Result<Vec<RPNToken>, ShuntingYardError> {
    tokens = perform_unary_minus(&tokens);
    tokens = insert_implicit_multiplication(&tokens);
    let mut output: Vec<RPNToken> = Vec::new();
    let mut opstack: Vec<InfixToken> = Vec::new();
    for token in tokens {
        match token {
            InfixToken::Literal(num) => output.push(RPNToken::Literal(num)),
            InfixToken::Variable(var) => output.push(RPNToken::Variable(var)),
            InfixToken::Function(_) => opstack.push(token),
            InfixToken::Operator(o1) => {
                while let Some(InfixToken::Operator(o2)) = opstack.last()
                    && (get_operator_precedence(*o2)
                        > get_operator_precedence(o1)
                        || (get_operator_precedence(o1)
                            == get_operator_precedence(*o2)
                            && !is_right_associative(o1)))
                {
                    let op = match o2 {
                        InfixTokenOperator::Add => {
                            RPNToken::ExpressionOp(ExpressionOp::Add)
                        }
                        InfixTokenOperator::Multiply => {
                            RPNToken::ExpressionOp(ExpressionOp::Multiply)
                        }
                        InfixTokenOperator::Divide => {
                            RPNToken::ExpressionOp(ExpressionOp::Divide)
                        }
                        InfixTokenOperator::SubtractOrNegate => {
                            RPNToken::ExpressionOp(ExpressionOp::Subtract)
                        }
                        InfixTokenOperator::Power => {
                            RPNToken::ExpressionOp(ExpressionOp::Power)
                        }
                        InfixTokenOperator::ImplicitMultiply => {
                            RPNToken::ExpressionOp(ExpressionOp::Multiply)
                        }
                    };
                    output.push(op);
                    let _ = opstack.pop();
                }
                opstack.push(InfixToken::Operator(o1));
            }
            InfixToken::ParenOpen => opstack.push(token),
            InfixToken::ParenClose => {
                loop {
                    match opstack.last() {
                        None => {
                            return Err(ShuntingYardError::MismatchedParens);
                        }
                        Some(InfixToken::ParenOpen) => break,
                        Some(InfixToken::Operator(op)) => {
                            output.push(match op {
                                InfixTokenOperator::Add => {
                                    RPNToken::ExpressionOp(ExpressionOp::Add)
                                }
                                InfixTokenOperator::Multiply => {
                                    RPNToken::ExpressionOp(
                                        ExpressionOp::Multiply,
                                    )
                                }
                                InfixTokenOperator::Divide => {
                                    RPNToken::ExpressionOp(ExpressionOp::Divide)
                                }
                                InfixTokenOperator::SubtractOrNegate => {
                                    RPNToken::ExpressionOp(
                                        ExpressionOp::Subtract,
                                    )
                                }
                                InfixTokenOperator::Power => {
                                    RPNToken::ExpressionOp(ExpressionOp::Power)
                                }
                                InfixTokenOperator::ImplicitMultiply => {
                                    RPNToken::ExpressionOp(
                                        ExpressionOp::Multiply,
                                    )
                                }
                            });
                            opstack.pop();
                        }
                        _ => unreachable!(),
                    }
                }
                assert!(matches!(opstack.pop(), Some(InfixToken::ParenOpen)));
                if let Some(InfixToken::Function(func)) = opstack.last() {
                    output.push(RPNToken::Function(*func));
                    let _ = opstack.pop();
                }
            }
        }
    }
    while let Some(op) = opstack.pop() {
        match op {
            InfixToken::ParenOpen => {
                return Err(ShuntingYardError::MismatchedParens);
            }
            InfixToken::Operator(op) => {
                output.push(RPNToken::ExpressionOp(match op {
                    InfixTokenOperator::Add => ExpressionOp::Add,
                    InfixTokenOperator::Multiply => ExpressionOp::Multiply,
                    InfixTokenOperator::Divide => ExpressionOp::Divide,
                    InfixTokenOperator::SubtractOrNegate => {
                        ExpressionOp::Subtract
                    }
                    InfixTokenOperator::Power => ExpressionOp::Power,
                    InfixTokenOperator::ImplicitMultiply => {
                        ExpressionOp::Multiply
                    }
                }))
            }
            _ => unreachable!(),
        }
    }
    Ok(output)
}

#[derive(Error, Debug)]
pub enum ShuntingYardError {
    #[error("Mismatched parentheses")]
    MismatchedParens,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_func() {
        let test_sets = [
            ("sin(x)", vec![
                InfixToken::Function(SupportedFunction::Sine),
                InfixToken::ParenOpen,
                InfixToken::Variable('x'),
                InfixToken::ParenClose,
            ]),
            ("1/(1000(x-19.5))", vec![
                InfixToken::Literal(1.),
                InfixToken::Operator(InfixTokenOperator::Divide),
                InfixToken::ParenOpen,
                InfixToken::Literal(1000.),
                InfixToken::ParenOpen,
                InfixToken::Variable('x'),
                InfixToken::Operator(InfixTokenOperator::SubtractOrNegate),
                InfixToken::Literal(19.5),
                InfixToken::ParenClose,
                InfixToken::ParenClose,
            ]),
        ];
        for (input, correct_tokens) in test_sets {
            let tokens = tokenize(input)
                .unwrap_or_else(|_| panic!("Failed to tokenize \"{input}\""));
            assert_eq!(tokens, correct_tokens);
        }
    }

    #[test]
    fn test_build_tree() {
        let test_sets = [(
            vec![
                RPNToken::Literal(-1.),
                RPNToken::Variable('x'),
                RPNToken::ExpressionOp(ExpressionOp::Multiply),
            ],
            ExpressionNode::Operation(
                ExpressionOp::Multiply,
                Box::new(ExpressionNode::Literal(-1.)),
                Box::new(ExpressionNode::Variable('x')),
            ),
        )];
        for (tokens, correct_tree) in test_sets {
            let tree = build_expression_tree(tokens).unwrap();
            assert_eq!(tree, correct_tree);
        }
    }

    #[test]
    fn test_shunting_yard() {
        let test_sets = [
            (
                vec![
                    InfixToken::Literal(2.),
                    InfixToken::Operator(InfixTokenOperator::Multiply),
                    InfixToken::Literal(3.),
                ],
                vec![
                    RPNToken::Literal(2.),
                    RPNToken::Literal(3.),
                    RPNToken::ExpressionOp(ExpressionOp::Multiply),
                ],
            ),
            (
                vec![
                    InfixToken::Operator(InfixTokenOperator::SubtractOrNegate),
                    InfixToken::Variable('x'),
                ],
                vec![
                    RPNToken::Literal(-1.),
                    RPNToken::Variable('x'),
                    RPNToken::ExpressionOp(ExpressionOp::Multiply),
                ],
            ),
            (
                vec![InfixToken::Literal(6.), InfixToken::Variable('x')],
                vec![
                    RPNToken::Literal(6.),
                    RPNToken::Variable('x'),
                    RPNToken::ExpressionOp(ExpressionOp::Multiply),
                ],
            ),
            (
                vec![
                    InfixToken::Variable('x'),
                    InfixToken::Operator(InfixTokenOperator::Add),
                    InfixToken::Function(SupportedFunction::Sine),
                    InfixToken::ParenOpen,
                    InfixToken::Literal(10.),
                    InfixToken::Variable('x'),
                    InfixToken::ParenClose,
                ],
                vec![
                    RPNToken::Variable('x'),
                    RPNToken::Literal(10.),
                    RPNToken::Variable('x'),
                    RPNToken::ExpressionOp(ExpressionOp::Multiply),
                    RPNToken::Function(SupportedFunction::Sine),
                    RPNToken::ExpressionOp(ExpressionOp::Add),
                ],
            ),
            (
                vec![
                    InfixToken::Literal(0.3),
                    InfixToken::Function(SupportedFunction::Sine),
                    InfixToken::ParenOpen,
                    InfixToken::Variable('x'),
                    InfixToken::ParenClose,
                ],
                vec![
                    RPNToken::Literal(0.3),
                    RPNToken::Variable('x'),
                    RPNToken::Function(SupportedFunction::Sine),
                    RPNToken::ExpressionOp(ExpressionOp::Multiply),
                ],
            ),
        ];
        for (infix, correct_rpn) in test_sets {
            let rpn = shunting_yard(infix).expect("Shunting yard failed");
            assert_eq!(rpn, correct_rpn);
        }
    }
}
