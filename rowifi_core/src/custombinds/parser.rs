use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, anychar, char, digit1, multispace0, multispace1},
    combinator::{all_consuming, map, map_res},
    error::VerboseError,
    multi::{many1, separated_list0},
    sequence::{delimited, pair, preceded, tuple},
    Err, IResult, Parser,
};
use std::fmt::{Display, Formatter, Result as FmtResult};

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
    Operation(Operator, Box<Expression>, Option<Box<Expression>>),
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
    map(delimited(char('"'), many1(anychar), char('"')), |s| {
        Atom::String(s.into_iter().collect())
    })
    .parse(i)
}

fn parse_atom(i: &str) -> IResult<&str, Atom, VerboseError<&str>> {
    alt((parse_number, parse_string)).parse(i)
}

fn parse_constant(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    map(parse_atom, Expression::Constant).parse(i)
}

fn parse_function_args(i: &str) -> IResult<&str, Vec<Expression>, VerboseError<&str>> {
    separated_list0(
        preceded(multispace0, char(',')),
        preceded(multispace0, map(parse_atom, Expression::Constant)),
    )(i)
}

fn parse_function(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    let (i, (name, args)) = pair(
        alphanumeric1,
        preceded(
            multispace0,
            delimited(char('('), parse_function_args, char(')')),
        ),
    )
    .parse(i)?;
    Ok((i, Expression::Function(name.to_string(), args)))
}

fn parse_brackets(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    delimited(
        char('('),
        alt((parse_operation, parse_expression)),
        char(')'),
    )
    .parse(i)
}

fn parse_comparison(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    let (i, (e1, op, e2)) = tuple((
        preceded(multispace0, parse_term),
        preceded(
            multispace0,
            alt((tag(">="), tag(">"), tag("<="), tag("<"), tag("=="))),
        ),
        preceded(multispace0, parse_term),
    ))
    .parse(i)?;

    let operator = match op {
        ">=" => Operator::GreaterEqual,
        ">" => Operator::Greater,
        "<=" => Operator::LessEqual,
        "<" => Operator::Less,
        "==" => Operator::Equal,
        _ => unreachable!(),
    };

    Ok((
        i,
        Expression::Operation(operator, Box::new(e1), Some(Box::new(e2))),
    ))
}

fn parse_term(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    alt((parse_function, parse_constant)).parse(i)
}

fn parse_negation(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    let (i, _) = tag("not")(i)?;
    let (i, _) = multispace1(i)?;
    let (i, expr) = parse_expression(i)?;
    Ok((
        i,
        Expression::Operation(Operator::Not, Box::new(expr), None),
    ))
}

fn parse_operation(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    let (i, e1) = preceded(multispace0, parse_expression)(i)?;
    let (i, rest) = nom::multi::many0(|input| {
        let (input, op) = preceded(multispace0, parse_operator)(input)?;
        let (input, expr) = preceded(multispace0, parse_expression)(input)?;
        Ok((input, (op, expr)))
    })(i)?;

    Ok((
        i,
        rest.into_iter().fold(e1, |acc, (op, expr)| {
            Expression::Operation(op, Box::new(acc), Some(Box::new(expr)))
        }),
    ))
}

fn parse_expression(i: &str) -> IResult<&str, Expression, VerboseError<&str>> {
    alt((
        parse_negation,
        parse_comparison,
        parse_brackets,
        parse_function,
        parse_constant,
    ))
    .parse(i)
}

pub fn parser(code: &str) -> Result<Expression, Err<VerboseError<&str>>> {
    let (_, t) = all_consuming(parse_operation).parse(code)?;
    Ok(t)
}

impl Display for Operator {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Operator::Greater => write!(f, ">"),
            Operator::GreaterEqual => write!(f, ">="),
            Operator::Less => write!(f, "<"),
            Operator::LessEqual => write!(f, "<="),
            Operator::Equal => write!(f, "=="),
            Operator::Not => write!(f, "not"),
            Operator::And => write!(f, "and"),
            Operator::Or => write!(f, "or"),
        }
    }
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
                    Some(Box::new(Expression::Function(
                        "IsInGroup".to_string(),
                        vec![Expression::Constant(Atom::Num(2))]
                    )))
                )
            ))
        );
    }

    #[test]
    fn full_test() {
        assert_eq!(
            parser("IsInGroup(1) and (GetRank(2) >= 3) or IsInGroup(2)"),
            Ok(Expression::Operation(
                Operator::Or,
                Box::new(Expression::Operation(
                    Operator::And,
                    Box::new(Expression::Function(
                        "IsInGroup".to_string(),
                        vec![Expression::Constant(Atom::Num(1))]
                    )),
                    Some(Box::new(Expression::Operation(
                        Operator::GreaterEqual,
                        Box::new(Expression::Function(
                            "GetRank".to_string(),
                            vec![Expression::Constant(Atom::Num(2))]
                        )),
                        Some(Box::new(Expression::Constant(Atom::Num(3))))
                    )))
                )),
                Some(Box::new(Expression::Function(
                    "IsInGroup".to_string(),
                    vec![Expression::Constant(Atom::Num(2))]
                )))
            ))
        );
    }

    #[test]
    fn full_test_2() {
        assert_eq!(
            parser("not HasRank(1000,25) and not HasRank (2000,25)"),
            Ok(Expression::Operation(
                Operator::And,
                Box::new(Expression::Operation(
                    Operator::Not,
                    Box::new(Expression::Function(
                        "HasRank".to_string(),
                        vec![
                            Expression::Constant(Atom::Num(1000)),
                            Expression::Constant(Atom::Num(25))
                        ]
                    )),
                    None
                )),
                Some(Box::new(Expression::Operation(
                    Operator::Not,
                    Box::new(Expression::Function(
                        "HasRank".to_string(),
                        vec![
                            Expression::Constant(Atom::Num(2000)),
                            Expression::Constant(Atom::Num(25))
                        ]
                    )),
                    None
                )))
            ))
        );

        let res = parser("not IsInGroup(3198375) and not IsInGroup(4929233) and not IsInGroup(3114833) and not IsInGroup(4929259) and not IsInGroup(7029618) and not IsInGroup(4929492) and not IsInGroup(4929487) and not IsInGroup(9606178) and not IsInGroup(8049931) and not IsInGroup(9582920) and not IsInGroup(15531002) and not IsInGroup(14504795) and not IsInGroup(34097972) and not (GetRank(3108077) >= 200 and GetRank(3108077) <= 253) and not (GetRank(5040124) >= 10) and GetRank(3108077) >=10");
        println!("{:?}", res);
        assert!(res.is_ok());
    }

    #[test]
    fn full_test_3() {
        assert_eq!(
            parser("(GetRank(1000) >= 200 and GetRank(1000) <= 253) and not (GetRank(2000) >= 10)"),
            Ok(Expression::Operation(
                Operator::And,
                Box::new(Expression::Operation(
                    Operator::And,
                    Box::new(Expression::Operation(
                        Operator::GreaterEqual,
                        Box::new(Expression::Function(
                            "GetRank".into(),
                            vec![Expression::Constant(Atom::Num(1000))]
                        )),
                        Some(Box::new(Expression::Constant(Atom::Num(200))))
                    )),
                    Some(Box::new(Expression::Operation(
                        Operator::LessEqual,
                        Box::new(Expression::Function(
                            "GetRank".into(),
                            vec![Expression::Constant(Atom::Num(1000))]
                        )),
                        Some(Box::new(Expression::Constant(Atom::Num(253))))
                    )),)
                )),
                Some(Box::new(Expression::Operation(
                    Operator::Not,
                    Box::new(Expression::Operation(
                        Operator::GreaterEqual,
                        Box::new(Expression::Function(
                            "GetRank".into(),
                            vec![Expression::Constant(Atom::Num(2000))]
                        )),
                        Some(Box::new(Expression::Constant(Atom::Num(10))))
                    )),
                    None
                )))
            ))
        )
    }
}
