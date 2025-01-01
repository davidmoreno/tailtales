use crate::record::Record;

#[derive(Debug, PartialEq, Clone)]
pub enum AST {
    Variable(String),
    String(String),
    Number(i64),
    GreaterThan(Box<AST>, Box<AST>),
    LessThan(Box<AST>, Box<AST>),
    Equal(Box<AST>, Box<AST>),
    Greater(Box<AST>, Box<AST>),
    Less(Box<AST>, Box<AST>),
    Not(Box<AST>),
    And(Box<AST>, Box<AST>),
    Or(Box<AST>, Box<AST>),
    Empty,
}

pub fn parse(input: &str) -> Result<AST, String> {
    let mut tokens = tokenize(input);
    parse_expression(&mut tokens)
}

enum Token {
    Number(i64),
    Variable(String),
    String(String),
    Boolean(bool),
    GreaterThan,
    LessThan,
    Equal,
    Greater,
    Less,
    Not,
    And,
    Or,
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '0'..='9' => {
                let mut num = c.to_string();
                while let Some(&c) = chars.peek() {
                    if c.is_digit(10) {
                        num.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num.parse().unwrap()));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut var = c.to_string();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        var.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Variable(var));
            }
            '"' => {
                let mut string = String::new();
                while let Some(c) = chars.next() {
                    if c == '"' {
                        break;
                    }
                    string.push(c);
                }
                tokens.push(Token::String(string));
            }
            '>' => {
                tokens.push(Token::GreaterThan);
            }
            '<' => {
                tokens.push(Token::LessThan);
            }
            '=' => {
                tokens.push(Token::Equal);
            }
            '!' => {
                tokens.push(Token::Not);
            }
            '&' => {
                tokens.push(Token::And);
            }
            '|' => {
                tokens.push(Token::Or);
            }
            _ => {}
        }
    }
    tokens
}

fn parse_expression(tokens: &mut Vec<Token>) -> Result<AST, String> {
    let mut ast = parse_term(tokens)?;
    while let Some(token) = tokens.first() {
        match token {
            Token::GreaterThan => {
                tokens.remove(0);
                ast = AST::GreaterThan(Box::new(ast), Box::new(parse_term(tokens)?));
            }
            Token::LessThan => {
                tokens.remove(0);
                ast = AST::LessThan(Box::new(ast), Box::new(parse_term(tokens)?));
            }
            Token::Equal => {
                tokens.remove(0);
                ast = AST::Equal(Box::new(ast), Box::new(parse_term(tokens)?));
            }
            Token::Not => {
                tokens.remove(0);
                ast = AST::Not(Box::new(parse_term(tokens)?));
            }
            Token::And => {
                tokens.remove(0);
                ast = AST::And(Box::new(ast), Box::new(parse_term(tokens)?));
            }
            Token::Or => {
                tokens.remove(0);
                ast = AST::Or(Box::new(ast), Box::new(parse_term(tokens)?));
            }
            _ => break,
        }
    }
    Ok(ast)
}

fn parse_term(tokens: &mut Vec<Token>) -> Result<AST, String> {
    if tokens.len() == 0 {
        return Ok(AST::Empty);
    }

    match tokens.remove(0) {
        Token::Number(n) => Ok(AST::Number(n)),
        Token::Variable(v) => Ok(AST::Variable(v)),
        Token::String(s) => Ok(AST::String(s)),
        _ => Err("unexpected token".to_string()),
    }
}

#[derive(Debug, PartialEq)]
enum Value {
    Number(i64),
    String(String),
    Boolean(bool),
}

pub fn execute(ast: AST, record: &Record) -> Value {
    match ast {
        AST::Variable(var) => {
            if let Some(value) = record.get(&var) {
                match value.parse::<i64>() {
                    Ok(n) => Value::Number(n),
                    Err(_) => Value::String(value.to_string()),
                }
            } else {
                Value::String("".to_string())
            }
        }
        AST::String(s) => Value::String(s),
        AST::Number(n) => Value::Number(n),
        AST::GreaterThan(lhs, rhs) => {
            let lhs = execute(*lhs, record);
            let rhs = execute(*rhs, record);
            match (lhs, rhs) {
                (Value::Number(lhs), Value::Number(rhs)) => Value::Boolean(lhs > rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::LessThan(lhs, rhs) => {
            let lhs = execute(*lhs, record);
            let rhs = execute(*rhs, record);
            match (lhs, rhs) {
                (Value::Number(lhs), Value::Number(rhs)) => Value::Boolean(lhs < rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Equal(lhs, rhs) => {
            let lhs = execute(*lhs, record);
            let rhs = execute(*rhs, record);
            Value::Boolean(lhs == rhs)
        }
        AST::Greater(lhs, rhs) => {
            let lhs = execute(*lhs, record);
            let rhs = execute(*rhs, record);
            match (lhs, rhs) {
                (Value::Number(lhs), Value::Number(rhs)) => Value::Boolean(lhs >= rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Less(lhs, rhs) => {
            let lhs = execute(*lhs, record);
            let rhs = execute(*rhs, record);
            match (lhs, rhs) {
                (Value::Number(lhs), Value::Number(rhs)) => Value::Boolean(lhs <= rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Not(ast) => {
            let ast = execute(*ast, record);
            match ast {
                Value::Boolean(b) => Value::Boolean(!b),
                _ => Value::Boolean(false),
            }
        }
        AST::And(lhs, rhs) => {
            let lhs = execute(*lhs, record);
            let rhs = execute(*rhs, record);
            match (lhs, rhs) {
                (Value::Boolean(lhs), Value::Boolean(rhs)) => Value::Boolean(lhs && rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Or(lhs, rhs) => {
            let lhs = execute(*lhs, record);
            let rhs = execute(*rhs, record);
            match (lhs, rhs) {
                (Value::Boolean(lhs), Value::Boolean(rhs)) => Value::Boolean(lhs || rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Empty => Value::Boolean(true),
    }
}

impl AST {
    pub fn matches(&self, record: &Record) -> bool {
        match execute(self.clone(), record) {
            Value::Boolean(b) => b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::record::Record;

    use super::*;

    #[test]
    fn test_parse() {
        assert_eq!(
            parse("1 > 2"),
            Ok(AST::GreaterThan(
                Box::new(AST::Number(1)),
                Box::new(AST::Number(2))
            ))
        );
        assert_eq!(
            parse("1 < 2"),
            Ok(AST::LessThan(
                Box::new(AST::Number(1)),
                Box::new(AST::Number(2))
            ))
        );
        assert_eq!(
            parse("1 == 2"),
            Ok(AST::Equal(
                Box::new(AST::Number(1)),
                Box::new(AST::Number(2))
            ))
        );
        assert_eq!(
            parse("1 >= 2"),
            Ok(AST::Greater(
                Box::new(AST::Number(1)),
                Box::new(AST::Number(2))
            ))
        );
        assert_eq!(
            parse("1 <= 2"),
            Ok(AST::Less(
                Box::new(AST::Number(1)),
                Box::new(AST::Number(2))
            ))
        );
        assert_eq!(parse("!1"), Ok(AST::Not(Box::new(AST::Number(1)))));
        assert_eq!(
            parse("1 && 2"),
            Ok(AST::And(Box::new(AST::Number(1)), Box::new(AST::Number(2))))
        );
        assert_eq!(
            parse("1 || 2"),
            Ok(AST::Or(Box::new(AST::Number(1)), Box::new(AST::Number(2))))
        );
        assert_eq!(
            parse("var > 1"),
            Ok(AST::GreaterThan(
                Box::new(AST::Variable("var".to_string())),
                Box::new(AST::Number(1)),
            )),
        );
        assert_eq!(
            parse("var1 == var2"),
            Ok(AST::Equal(
                Box::new(AST::Variable("var1".to_string())),
                Box::new(AST::Variable("var2".to_string())),
            )),
        );
    }
    fn test_execute() {
        let record = Record::new("2024-01-01 00:00:00".to_string())
            .set_data("hostname", "localhost".to_string())
            .set_data("program", "test".to_string())
            .set_data("rest", "message".to_string());
        assert_eq!(
            execute(
                AST::Equal(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(false),
        );
        assert_eq!(
            execute(
                AST::GreaterThan(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(false),
        );
        assert_eq!(
            execute(
                AST::LessThan(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(true),
        );
        assert_eq!(
            execute(
                AST::And(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(true),
        );
        assert_eq!(
            execute(
                AST::Or(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(true),
        );
    }
}
