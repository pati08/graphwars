use std::str::FromStr;
use thiserror::Error;

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
        match token {
            RPNToken::Literal(num) => {
                stack.push(ExpressionNode::Literal(num));
            }
            RPNToken::VariableName(name) => {
                stack.push(ExpressionNode::Variable(name));
            }
            RPNToken::UnaryOperator(op) => {
                if let Some(operand) = stack.pop() {
                    stack.push(ExpressionNode::Unary(op, Box::new(operand)));
                } else {
                    return Err(TreeBuildError::NotEnoughUnaryOperands);
                }
            }
            RPNToken::BinaryOperator(op) => {
                let right =
                    stack.pop().ok_or(TreeBuildError::MissingRightOperand)?;
                let left =
                    stack.pop().ok_or(TreeBuildError::MissingLeftOperand)?;
                stack.push(ExpressionNode::Pair(
                    op,
                    Box::new(left),
                    Box::new(right),
                ));
            }
            RPNToken::Function(func) => {
                let operand =
                    stack.pop().ok_or(TreeBuildError::MissingFunctionArg)?;
                stack.push(ExpressionNode::Function(func, Box::new(operand)));
            }
        }
    }

    if stack.len() != 1 {
        return Err(TreeBuildError::RemainingNodes);
    }

    Ok(stack.pop().unwrap())
}

impl FromStr for ParsedFunction {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens = tokenize(s)?;
        let rpn = shunting_yard(tokens);
        let expression_tree = build_expression_tree(rpn)?;
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
    TokenizerError(#[from] TokenizerError),
    #[error("Failed to build expression tree: {0}")]
    TreeBuildError(#[from] TreeBuildError),
}

#[derive(Debug, Error)]
pub enum TreeBuildError {
    #[error("Insufficient operands for unary operator")]
    NotEnoughUnaryOperands,
    #[error("Missing left operand for binary operator")]
    MissingLeftOperand,
    #[error("Missing right operand for binary operator")]
    MissingRightOperand,
    #[error("Missing function argument")]
    MissingFunctionArg,
    #[error("Invalid expression: multiple nodes remain on stack")]
    RemainingNodes,
}

#[derive(Clone, Debug, PartialEq)]
enum ExpressionNode {
    Literal(f32),
    Variable(String),
    Pair(BinaryOp, Box<ExpressionNode>, Box<ExpressionNode>),
    Unary(UnaryOp, Box<ExpressionNode>),
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
            ExpressionNode::Pair(op, left, right) => {
                Ok(op.apply(left.eval(vars)?, right.eval(vars)?)?)
            }
            ExpressionNode::Unary(operator, operand) => {
                Ok(operator.apply(operand.eval(vars)?))
            }
            ExpressionNode::Literal(val) => Ok(*val),
            ExpressionNode::Variable(var) => {
                if let Some((_, val)) = vars.iter().find(|i| i.0 == *var) {
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
    Sin,
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
            Self::Sin => Ok(arg.sin()),
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
    UnaryOperator(UnaryOp),
    BinaryOperator(BinaryOp),
    Function(SupportedFunction),
    VariableName(String),
    Literal(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum UnaryOp {
    Negate,
}
impl UnaryOp {
    fn apply(&self, arg: f32) -> f32 {
        match self {
            Self::Negate => -arg,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BinaryOp {
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

impl BinaryOp {
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
            .nth(at)
            .map(|c| TOKEN_OPS.iter().find(|&i| i.0 == c).map(|v| v.1))
            .flatten()
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

fn should_be_unary_minus(tokens: &[InfixToken], current_pos: usize) -> bool {
    if current_pos == 0 {
        return true;
    }

    matches!(
        tokens.get(current_pos - 1),
        Some(InfixToken::ParenOpen) | Some(InfixToken::Operator(_))
    )
}

fn should_add_implicit_multiply(prev: &InfixToken, next: &InfixToken) -> bool {
    match (prev, next) {
        // Cases like: 2x, 2(x+1)
        (InfixToken::Literal(_), InfixToken::Variable(_)) |
        (InfixToken::Literal(_), InfixToken::ParenOpen) |
        // Cases like: x(y+1), (x+1)(y+2)
        (InfixToken::Variable(_), InfixToken::ParenOpen) |
        (InfixToken::ParenClose, InfixToken::ParenOpen) |
        // Cases like: xy, x(y+1)
        (InfixToken::Variable(_), InfixToken::Variable(_)) => true,
        _ => false
    }
}

fn handle_function_application(tokens: Vec<InfixToken>) -> Vec<InfixToken> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        if matches!(tokens[i], InfixToken::Function(_)) {
            let func = tokens[i];
            // Look ahead - if next token is not an open paren, treat as implicit function application
            if i + 1 < tokens.len()
                && !matches!(tokens[i + 1], InfixToken::ParenOpen)
            {
                // Push the operand first, then the function
                result.push(tokens[i + 1]);
                if let InfixToken::Function(f) = func {
                    result.push(InfixToken::Function(f));
                }
                i += 2;
                continue;
            }
        }
        result.push(tokens[i]);
        i += 1;
    }

    result
}

fn should_pop_operator(
    stack_op: InfixTokenOperator,
    current_op: InfixTokenOperator,
    is_unary: bool,
) -> bool {
    if is_unary {
        // Unary operators have highest precedence
        return false;
    }

    let stack_precedence = get_operator_precedence(stack_op);
    let current_precedence = get_operator_precedence(current_op);

    if is_right_associative(current_op) {
        stack_precedence > current_precedence
    } else {
        stack_precedence >= current_precedence
    }
}

fn insert_implicit_multiplications(tokens: Vec<InfixToken>) -> Vec<InfixToken> {
    let mut result = Vec::new();
    let mut prev_token: Option<&InfixToken> = None;

    for token in tokens.iter() {
        if let Some(prev) = prev_token {
            if should_add_implicit_multiply(prev, token) {
                result.push(InfixToken::Operator(
                    InfixTokenOperator::ImplicitMultiply,
                ));
            }
        }
        result.push(*token);
        prev_token = Some(token);
    }

    result
}

fn shunting_yard(tokens: Vec<InfixToken>) -> Vec<RPNToken> {
    // First handle function applications
    let tokens = handle_function_application(tokens);

    // Then handle implicit multiplications
    let tokens = insert_implicit_multiplications(tokens);

    let mut output: Vec<RPNToken> = Vec::new();
    let mut operator_stack: Vec<(InfixToken, bool)> = Vec::new(); // Store operator and whether it's unary

    for (pos, token) in tokens.iter().copied().enumerate() {
        match token {
            InfixToken::Literal(num) => {
                output.push(RPNToken::Literal(num));
            }
            InfixToken::Variable(var) => {
                output.push(RPNToken::VariableName(var.to_string()));
            }
            InfixToken::Function(func) => {
                // If it's a bare function (from our handle_function_application),
                // directly output it
                if pos > 0 && !matches!(tokens[pos - 1], InfixToken::ParenOpen)
                {
                    output.push(RPNToken::Function(func));
                } else {
                    operator_stack.push((InfixToken::Function(func), false));
                }
            }
            InfixToken::ParenOpen => {
                operator_stack.push((InfixToken::ParenOpen, false));
            }
            InfixToken::ParenClose => {
                while let Some((top, _)) = operator_stack.last() {
                    if matches!(top, InfixToken::ParenOpen) {
                        operator_stack.pop();
                        if let Some((InfixToken::Function(func), _)) =
                            operator_stack.last()
                        {
                            output.push(RPNToken::Function(*func));
                            operator_stack.pop();
                        }
                        break;
                    }
                    if let Some((InfixToken::Operator(op), is_unary)) =
                        operator_stack.pop()
                    {
                        if is_unary {
                            output
                                .push(RPNToken::UnaryOperator(UnaryOp::Negate));
                        } else {
                            output.push(RPNToken::BinaryOperator(match op {
                                InfixTokenOperator::Add => BinaryOp::Add,
                                InfixTokenOperator::SubtractOrNegate => {
                                    BinaryOp::Subtract
                                }
                                InfixTokenOperator::Multiply
                                | InfixTokenOperator::ImplicitMultiply => {
                                    BinaryOp::Multiply
                                }
                                InfixTokenOperator::Divide => BinaryOp::Divide,
                                InfixTokenOperator::Power => BinaryOp::Power,
                            }));
                        }
                    }
                }
            }
            InfixToken::Operator(op) => {
                let is_unary =
                    matches!(op, InfixTokenOperator::SubtractOrNegate)
                        && should_be_unary_minus(&tokens, pos);

                while let Some((
                    InfixToken::Operator(stack_op),
                    stack_is_unary,
                )) = operator_stack.last().copied()
                {
                    if should_pop_operator(stack_op, op, is_unary) {
                        operator_stack.pop();
                        if stack_is_unary {
                            output
                                .push(RPNToken::UnaryOperator(UnaryOp::Negate));
                        } else {
                            output.push(RPNToken::BinaryOperator(
                                match stack_op {
                                    InfixTokenOperator::Add => BinaryOp::Add,
                                    InfixTokenOperator::SubtractOrNegate => {
                                        BinaryOp::Subtract
                                    }
                                    InfixTokenOperator::Multiply
                                    | InfixTokenOperator::ImplicitMultiply => {
                                        BinaryOp::Multiply
                                    }
                                    InfixTokenOperator::Divide => {
                                        BinaryOp::Divide
                                    }
                                    InfixTokenOperator::Power => {
                                        BinaryOp::Power
                                    }
                                },
                            ));
                        }
                    } else {
                        break;
                    }
                }

                operator_stack.push((token, is_unary));
            }
        }
    }

    while let Some((op, is_unary)) = operator_stack.pop() {
        match op {
            InfixToken::Operator(stack_op) => {
                if is_unary {
                    output.push(RPNToken::UnaryOperator(UnaryOp::Negate));
                } else {
                    output.push(RPNToken::BinaryOperator(match stack_op {
                        InfixTokenOperator::Add => BinaryOp::Add,
                        InfixTokenOperator::SubtractOrNegate => {
                            BinaryOp::Subtract
                        }
                        InfixTokenOperator::Multiply
                        | InfixTokenOperator::ImplicitMultiply => {
                            BinaryOp::Multiply
                        }
                        InfixTokenOperator::Divide => BinaryOp::Divide,
                        InfixTokenOperator::Power => BinaryOp::Power,
                    }));
                }
            }
            InfixToken::Function(func) => {
                output.push(RPNToken::Function(func));
            }
            _ => {}
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_func() {
        let test_sets = [
            ("sin(x)", vec![
                InfixToken::Function(SupportedFunction::Sin),
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
            println!();
        }
    }

    #[test]
    fn test_build_tree() {
        let test_sets = [(
            vec![
                RPNToken::VariableName("x".to_string()),
                RPNToken::UnaryOperator(UnaryOp::Negate),
            ],
            ExpressionNode::Unary(
                UnaryOp::Negate,
                Box::new(ExpressionNode::Variable("x".to_string())),
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
                    InfixToken::Operator(InfixTokenOperator::SubtractOrNegate),
                    InfixToken::Variable('x'),
                ],
                vec![
                    RPNToken::VariableName("x".to_string()),
                    RPNToken::UnaryOperator(UnaryOp::Negate),
                ],
            ),
            (
                vec![
                    InfixToken::ParenOpen,
                    InfixToken::Operator(InfixTokenOperator::SubtractOrNegate),
                    InfixToken::Literal(1.),
                    InfixToken::ParenClose,
                ],
                vec![
                    RPNToken::Literal(1.),
                    RPNToken::UnaryOperator(UnaryOp::Negate),
                ],
            ),
        ];
        for (infix, correct_rpn) in test_sets {
            let rpn = shunting_yard(infix);
            assert_eq!(rpn, correct_rpn);
        }
    }
}
