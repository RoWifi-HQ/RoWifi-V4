use rowifi_models::{id::RoleId, roblox::id::GroupId};
use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
};

use super::parser::{Atom, Expression, Operator};

pub enum EvaluationResult {
    Bool(bool),
    Number(u64),
}

pub struct EvaluationContext<'c> {
    pub roles: &'c [RoleId],
    pub ranks: &'c HashMap<GroupId, u32>,
    pub username: &'c str,
}

pub enum EvaluationError {
    IncorrectArgumentCount {
        name: &'static str,
        expected: usize,
        found: usize,
    },
    IncorrectArgument {
        name: &'static str,
        idx: usize,
        found: &'static str,
        expected: &'static str,
    },
    UnknownFunction {
        name: String,
    },
}

pub fn evaluate(
    expr: &Expression,
    context: &EvaluationContext<'_>,
) -> Result<EvaluationResult, EvaluationError> {
    match expr {
        Expression::Operation(op, e1, e2) => {
            let lhs = evaluate(e1, context)?;
            let rhs = evaluate(e2, context)?;
            let res = match op {
                Operator::And => lhs.and(rhs),
                Operator::Or => lhs.or(rhs),
                Operator::GreaterEqual => EvaluationResult::Bool(lhs >= rhs),
                Operator::Greater => EvaluationResult::Bool(lhs > rhs),
                Operator::LessEqual => EvaluationResult::Bool(lhs <= rhs),
                Operator::Less => EvaluationResult::Bool(lhs < rhs),
                Operator::Equal => EvaluationResult::Bool(lhs == rhs),
                Operator::Not => todo!(),
            };
            Ok(res)
        }
        Expression::Function(name, args) => {
            let res = match name.as_str() {
                "IsInGroup" => {
                    if args.len() == 1 {
                        let group = match &args[0] {
                            Expression::Constant(Atom::Num(group)) => *group,
                            Expression::Function(_, _) | Expression::Operation(_, _, _) => {
                                let res = evaluate(&args[0], context)?;
                                match res {
                                    EvaluationResult::Bool(b) => b as u64,
                                    EvaluationResult::Number(n) => n,
                                }
                            }
                            Expression::Constant(Atom::String(_)) => {
                                return Err(EvaluationError::IncorrectArgument {
                                    name: "IsInGroup",
                                    idx: 0,
                                    found: "String",
                                    expected: "Number",
                                })
                            }
                        };
                        let success = context.ranks.contains_key(&GroupId(group));
                        Ok(EvaluationResult::Bool(success))
                    } else {
                        return Err(EvaluationError::IncorrectArgumentCount {
                            name: "IsInGroup",
                            expected: 1,
                            found: args.len(),
                        });
                    }
                }
                "HasRank" => {
                    if args.len() == 2 {
                        let group = match &args[0] {
                            Expression::Constant(Atom::Num(group)) => *group,
                            Expression::Constant(Atom::String(_)) => {
                                return Err(EvaluationError::IncorrectArgument {
                                    name: "HasRank",
                                    idx: 0,
                                    found: "String",
                                    expected: "Number",
                                })
                            }
                            Expression::Function(_, _) | Expression::Operation(_, _, _) => {
                                let res = evaluate(&args[0], context)?;
                                match res {
                                    EvaluationResult::Bool(b) => b as u64,
                                    EvaluationResult::Number(n) => n,
                                }
                            }
                        };
                        let rank = match &args[1] {
                            Expression::Constant(Atom::Num(group)) => *group,
                            Expression::Constant(Atom::String(_)) => {
                                return Err(EvaluationError::IncorrectArgument {
                                    name: "HasRank",
                                    idx: 1,
                                    found: "String",
                                    expected: "Number",
                                })
                            }
                            Expression::Function(_, _) | Expression::Operation(_, _, _) => {
                                let res = evaluate(&args[1], context)?;
                                match res {
                                    EvaluationResult::Bool(b) => b as u64,
                                    EvaluationResult::Number(n) => n,
                                }
                            }
                        };
                        let success = match context.ranks.get(&GroupId(group)) {
                            Some(r) => *r == rank as u32,
                            None => false,
                        };
                        Ok(EvaluationResult::Bool(success))
                    } else {
                        return Err(EvaluationError::IncorrectArgumentCount {
                            name: "HasRank",
                            expected: 2,
                            found: args.len(),
                        });
                    }
                }
                "HasRole" => {
                    if args.len() == 1 {
                        let role = match &args[0] {
                            Expression::Constant(Atom::Num(role)) => *role,
                            Expression::Function(_, _) | Expression::Operation(_, _, _) => {
                                let res = evaluate(&args[0], context)?;
                                match res {
                                    EvaluationResult::Bool(b) => b as u64,
                                    EvaluationResult::Number(n) => n,
                                }
                            }
                            Expression::Constant(Atom::String(_)) => {
                                return Err(EvaluationError::IncorrectArgument {
                                    name: "HasRole",
                                    idx: 0,
                                    found: "String",
                                    expected: "Number",
                                })
                            }
                        };
                        let success = context.roles.contains(&RoleId::new(role));
                        Ok(EvaluationResult::Bool(success))
                    } else {
                        return Err(EvaluationError::IncorrectArgumentCount {
                            name: "HasRole",
                            expected: 1,
                            found: args.len(),
                        });
                    }
                }
                "WithString" => {
                    if args.len() == 1 {
                        let name = match &args[0] {
                            Expression::Constant(Atom::String(str)) => str,
                            _ => {
                                return Err(EvaluationError::IncorrectArgument {
                                    name: "HasRole",
                                    idx: 0,
                                    found: "String",
                                    expected: "Number",
                                })
                            }
                        };
                        let success = context.username.contains(name.as_str());
                        Ok(EvaluationResult::Bool(success))
                    } else {
                        return Err(EvaluationError::IncorrectArgumentCount {
                            name: "WithString",
                            expected: 1,
                            found: args.len(),
                        });
                    }
                }
                "GetRank" => {
                    if args.len() == 1 {
                        let group = match &args[0] {
                            Expression::Constant(Atom::Num(group)) => *group,
                            Expression::Function(_, _) | Expression::Operation(_, _, _) => {
                                let res = evaluate(&args[0], context)?;
                                match res {
                                    EvaluationResult::Bool(b) => b as u64,
                                    EvaluationResult::Number(n) => n,
                                }
                            }
                            Expression::Constant(Atom::String(_)) => {
                                return Err(EvaluationError::IncorrectArgument {
                                    name: "GetRank",
                                    idx: 0,
                                    found: "String",
                                    expected: "Number",
                                })
                            }
                        };
                        Ok(EvaluationResult::Number(group))
                    } else {
                        return Err(EvaluationError::IncorrectArgumentCount {
                            name: "GetRank",
                            expected: 1,
                            found: args.len(),
                        });
                    }
                }
                _ => return Err(EvaluationError::UnknownFunction { name: name.clone() }),
            };
            res
        }
        Expression::Constant(atom) => match atom {
            Atom::Num(num) => Ok(EvaluationResult::Number(*num)),
            Atom::String(_) => Ok(EvaluationResult::Bool(true)),
        },
    }
}

impl EvaluationResult {
    pub fn and(self, rhs: Self) -> EvaluationResult {
        let res = match (self, rhs) {
            (Self::Bool(b), Self::Number(n)) | (Self::Number(n), Self::Bool(b)) => b && (n != 0),
            (Self::Bool(b1), Self::Bool(b2)) => b1 && b2,
            (Self::Number(n1), Self::Number(n2)) => n1 != 0 && n2 != 0,
        };
        EvaluationResult::Bool(res)
    }

    pub fn or(self, rhs: Self) -> EvaluationResult {
        let res = match (self, rhs) {
            (Self::Bool(b), Self::Number(n)) | (Self::Number(n), Self::Bool(b)) => b || (n != 0),
            (Self::Bool(b1), Self::Bool(b2)) => b1 || b2,
            (Self::Number(n1), Self::Number(n2)) => n1 != 0 || n2 != 0,
        };
        EvaluationResult::Bool(res)
    }
}

impl PartialOrd for EvaluationResult {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        match (self, rhs) {
            (Self::Bool(b), Self::Number(n)) => (*b as u64).partial_cmp(&n),
            (Self::Number(n), Self::Bool(b)) => n.partial_cmp(&(*b as u64)),
            (Self::Bool(b1), Self::Bool(b2)) => b1.partial_cmp(&b2),
            (Self::Number(n1), Self::Number(n2)) => n1.partial_cmp(&n2),
        }
    }
}

impl PartialEq for EvaluationResult {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (Self::Bool(b), Self::Number(n)) => (*b as u64).eq(n),
            (Self::Number(n), Self::Bool(b)) => n.eq(&(*b as u64)),
            (Self::Bool(b1), Self::Bool(b2)) => b1.eq(b2),
            (Self::Number(n1), Self::Number(n2)) => n1.eq(n2),
        }
    }
}

impl Display for EvaluationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::IncorrectArgument {
                name,
                idx,
                found,
                expected,
            } => write!(
                f,
                "Argument {} of function {} is expected to be of type {}. It was found to be a {}",
                idx + 1,
                name,
                expected,
                found
            ),
            Self::IncorrectArgumentCount {
                name,
                expected,
                found,
            } => write!(
                f,
                "Function {} is expected to have {} arguments. It has {} arguments currently",
                name, expected, found
            ),
            Self::UnknownFunction { name } => {
                write!(f, "Function {} is not a valid function", name)
            }
        }
    }
}
