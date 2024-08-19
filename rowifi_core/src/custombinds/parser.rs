use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0},
    combinator::{all_consuming, map, map_res},
    error::VerboseError,
    sequence::{delimited, pair, preceded, tuple},
    Err, IResult, Parser,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operator {
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
    Not,
    And,
    Or,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Atom {
    Num(u64),
    String(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expression {
    Constant(Atom),
    Function(String, Vec<Expression>),
    Operation(Operator, Box<Expression>, Box<Expression>),
}

fn parse_operator(i: &str) -> IResult<&str, Operator, VerboseError<&str>> {
    let (i, t) = alt((
        tag(">="),
        tag(">"),
        tag("<="),
        tag("<"),
        tag("=="),
        tag("and"),
        tag("or"),
    ))(i)?;
    let op = match t {
        ">=" => Operator::GreaterEqual,
        ">" => Operator::Greater,
        "<=" => Operator::LessEqual,
        "<" => Operator::Less,
        "==" => Operator::Equal,
        "and" => Operator::And,
        "or" => Operator::Or,
        _ => unreachable!(),
    };
    Ok((i, op))
}

fn parse_number(i: &str) -> IResult<&str, Atom, VerboseError<&str>> {
    map_res(digit1, |digit_str: &str| {
        digit_str.parse::<u64>().map(Atom::Num)
    })
    .parse(i)
}

fn parse_string(i: &str) -> IResult<&str, Atom, VerboseError<&str>> {
    map(delimited(char('"'), alpha1, char('"')), |s: &str| {
        Atom::String(s.to_string())
    })
    .parse(i)
}

fn parse_atom(i: &str) -> IResult<&str, Atom, VerboseError<&str>> {
    alt((parse_number, parse_string)).parse(i)
}

fn parse_constant(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    map(parse_atom, Expression::Constant).parse(i)
}

fn parse_function(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    let (i, t) = pair(alphanumeric1, delimited(char('('), parse_atom, char(')'))).parse(i)?;
    let exp = Expression::Function(t.0.to_string(), vec![Expression::Constant(t.1)]);
    Ok((i, exp))
}

fn parse_brackets(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    delimited(
        char('('),
        alt((parse_operation, parse_expression)),
        char(')'),
    )
    .parse(i)
}

fn parse_operation(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    let (i, (e1, op, e2)) = tuple((
        preceded(multispace0, parse_expression),
        preceded(multispace0, parse_operator),
        preceded(multispace0, parse_expression),
    ))
    .parse(i)?;
    Ok((i, Expression::Operation(op, Box::new(e1), Box::new(e2))))
}

fn parse_expression(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    alt((parse_brackets, parse_function, parse_constant)).parse(i)
}

pub fn parser(code: &str) -> Result<Expression, Err<VerboseError<&str>>> {
    let (_, t) = all_consuming(alt((parse_operation, parse_expression))).parse(code)?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_test() {
        assert_eq!(parse_operator(">="), Ok(("", Operator::GreaterEqual)));
        assert_eq!(parse_operator(">"), Ok(("", Operator::Greater)));
        assert_eq!(parse_operator("<="), Ok(("", Operator::LessEqual)));
        assert_eq!(parse_operator("<"), Ok(("", Operator::Less)));
        assert_eq!(parse_operator("=="), Ok(("", Operator::Equal)));
        assert_eq!(parse_operator("and"), Ok(("", Operator::And)));
        assert_eq!(parse_operator("or"), Ok(("", Operator::Or)));
    }

    #[test]
    fn number_test() {
        assert_eq!(parse_number("123"), Ok(("", Atom::Num(123))));
        assert!(parse_number("abs").is_err());
    }

    #[test]
    fn string_test() {
        assert_eq!(
            parse_string("\"seven\""),
            Ok(("", Atom::String("seven".to_string())))
        );
    }

    #[test]
    fn function_test() {
        assert_eq!(
            parse_function("IsInGroup(1)"),
            Ok((
                "",
                Expression::Function(
                    "IsInGroup".to_string(),
                    vec![Expression::Constant(Atom::Num(1))]
                )
            ))
        );
    }

    #[test]
    fn bracket_test() {
        assert_eq!(
            parse_brackets("(IsInGroup(123))"),
            Ok((
                "",
                Expression::Function(
                    "IsInGroup".to_string(),
                    vec![Expression::Constant(Atom::Num(123))]
                )
            ))
        );
    }

    #[test]
    fn operation_test() {
        assert_eq!(
            parse_operation("IsInGroup(1) and IsInGroup(2)"),
            Ok((
                "",
                Expression::Operation(
                    Operator::And,
                    Box::new(Expression::Function(
                        "IsInGroup".to_string(),
                        vec![Expression::Constant(Atom::Num(1))]
                    )),
                    Box::new(Expression::Function(
                        "IsInGroup".to_string(),
                        vec![Expression::Constant(Atom::Num(2))]
                    ))
                )
            ))
        );
    }

    #[test]
    fn full_test() {
        assert_eq!(
            parser("IsInGroup(1) and (GetRank(2) >= 3)"),
            Ok(Expression::Operation(
                Operator::And,
                Box::new(Expression::Function(
                    "IsInGroup".to_string(),
                    vec![Expression::Constant(Atom::Num(1))]
                )),
                Box::new(Expression::Operation(
                    Operator::GreaterEqual,
                    Box::new(Expression::Function(
                        "GetRank".to_string(),
                        vec![Expression::Constant(Atom::Num(2))]
                    )),
                    Box::new(Expression::Constant(Atom::Num(3)))
                ))
            ))
        )
    }
}
