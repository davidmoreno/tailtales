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
    let mut tokens = match tokenize(input) {
        Ok(tokens) => tokens,
        Err(e) => return Err(e),
    };
    // We convert simple variables to strings. If want the variable as it must exist, use !!var
    match parse_expression(&mut tokens) {
        Ok(AST::Variable(var)) => Ok(AST::String(var)),
        Ok(ast) => Ok(ast),
        Err(e) => Err(e),
    }
}

#[derive(Debug, PartialEq)]
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

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
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
                // >=
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::GreaterThan);
                } else {
                    tokens.push(Token::Greater);
                }
            }
            '<' => {
                // <=
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token::LessThan);
                } else {
                    tokens.push(Token::Less);
                }
            }
            '=' => {
                tokens.push(Token::Equal);
                // optional =
                if let Some(&'=') = chars.peek() {
                    chars.next();
                }
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
            ' ' => {}
            _ => {
                return Err(format!("unexpected character {:?}", c));
            }
        }
    }
    Ok(tokens)
}

fn parse_expression(tokens: &mut Vec<Token>) -> Result<AST, String> {
    let mut ast = parse_term(tokens)?;

    while let Some(token) = tokens.first() {
        match token {
            Token::Greater => {
                tokens.remove(0);
                ast = AST::Greater(Box::new(ast), Box::new(parse_term(tokens)?));
            }
            Token::GreaterThan => {
                tokens.remove(0);
                ast = AST::GreaterThan(Box::new(ast), Box::new(parse_term(tokens)?));
            }
            Token::Less => {
                tokens.remove(0);
                ast = AST::Less(Box::new(ast), Box::new(parse_term(tokens)?));
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
            Token::Number(n) => {
                ast = AST::Number(*n);
                tokens.remove(0);
            }
            Token::Variable(v) => {
                ast = AST::Variable(v.clone());
                tokens.remove(0);
            }
            Token::String(s) => {
                ast = AST::String(s.clone());
                tokens.remove(0);
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
        token => Err(format!("unexpected token {:?}", token)),
    }
}

#[derive(Debug, PartialEq)]
enum Value {
    Number(i64),
    String(String),
    Boolean(bool),
}

/**
 * Example cases, for record "this is a test=10", {"test": "10"}:
 *
 * "this" -> true
 * this -> true
 * !!this -> false
 * test -> true
 * !!test -> true
 * !!nope -> false
 * test >= 10 -> false
 * 10 -> true
 * 20 -> false
 * test > 0 && test < 100 -> true
 * test > 10 || test < 100 -> true
 * test > 10 || test < 5 -> false
 * (test > 10 || test < 5) && test < 100 -> false
 */

pub fn execute(ast: &AST, record: &Record) -> Value {
    match ast {
        AST::String(s) => Value::Boolean(record.original.contains(s)),
        AST::Variable(var) => {
            if let Some(value) = record.get(&var) {
                match value.parse::<i64>() {
                    Ok(n) => Value::Number(n),
                    Err(_) => Value::String(value.to_string()),
                }
            } else {
                Value::Boolean(false)
            }
        }
        AST::Number(n) => Value::Number(n.clone()),
        AST::GreaterThan(lhs, rhs) => {
            let lhs = execute(&lhs, record);
            let rhs = execute(&rhs, record);
            match (lhs, rhs) {
                (Value::Number(lhs), Value::Number(rhs)) => Value::Boolean(lhs > rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::LessThan(lhs, rhs) => {
            let lhs = execute(&lhs, record);
            let rhs = execute(&rhs, record);
            match (lhs, rhs) {
                (Value::Number(lhs), Value::Number(rhs)) => Value::Boolean(lhs < rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Equal(lhs, rhs) => {
            let lhs = execute(&lhs, record);
            let rhs = execute(&rhs, record);
            Value::Boolean(lhs == rhs)
        }
        AST::Greater(lhs, rhs) => {
            let lhs = execute(&lhs, record);
            let rhs = execute(&rhs, record);
            match (lhs, rhs) {
                (Value::Number(lhs), Value::Number(rhs)) => Value::Boolean(lhs >= rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Less(lhs, rhs) => {
            let lhs = execute(&lhs, record);
            let rhs = execute(&rhs, record);
            match (lhs, rhs) {
                (Value::Number(lhs), Value::Number(rhs)) => Value::Boolean(lhs <= rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Not(ast) => {
            let ast = execute_to_bool(ast, record);
            match ast {
                Value::Boolean(b) => Value::Boolean(!b),
                _ => Value::Boolean(false),
            }
        }
        AST::And(lhs, rhs) => {
            let lhs = execute_to_bool(&lhs, record);
            let rhs = execute_to_bool(&rhs, record);

            match (lhs, rhs) {
                (Value::Boolean(lhs), Value::Boolean(rhs)) => Value::Boolean(lhs && rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Or(lhs, rhs) => {
            let lhs = execute_to_bool(&lhs, record);
            let rhs = execute_to_bool(&rhs, record);
            match (lhs, rhs) {
                (Value::Boolean(lhs), Value::Boolean(rhs)) => Value::Boolean(lhs || rhs),
                _ => Value::Boolean(false),
            }
        }
        AST::Empty => Value::Boolean(true),
    }
}
fn execute_to_bool(ast: &AST, record: &Record) -> Value {
    match execute(ast, record) {
        Value::Number(n) => Value::Boolean(true),
        Value::String(s) => Value::Boolean(record.data.get(&s).is_some()),
        Value::Boolean(b) => Value::Boolean(b),
    }
}

impl AST {
    pub fn matches(&self, record: &Record) -> bool {
        match execute(self, record) {
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
    fn test_tokenize() {
        let tokens = tokenize("1 > 2");
        assert_eq!(
            tokens,
            Ok(vec![Token::Number(1), Token::Greater, Token::Number(2)])
        );
        let tokens = tokenize("1 <= 2");
        assert_eq!(
            tokens,
            Ok(vec![Token::Number(1), Token::LessThan, Token::Number(2)])
        );
        let tokens = tokenize("1 == 2");
        assert_eq!(
            tokens,
            Ok(vec![Token::Number(1), Token::Equal, Token::Number(2)])
        );
        let tokens = tokenize("var >= 2");
        assert_eq!(
            tokens,
            Ok(vec![
                Token::Variable("var".to_string()),
                Token::GreaterThan,
                Token::Number(2)
            ])
        );
        let tokens = tokenize("1 < \"string\"");
        assert_eq!(
            tokens,
            Ok(vec![
                Token::Number(1),
                Token::Less,
                Token::String("string".to_string())
            ])
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(parse("1 >> 2").is_err(), true);
        assert_eq!(
            parse("1 > 2"),
            Ok(AST::Greater(
                Box::new(AST::Number(1)),
                Box::new(AST::Number(2))
            ))
        );
        assert_eq!(
            parse("1 < 2"),
            Ok(AST::Less(
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

    #[test]
    fn test_one_var_is_string() {
        assert_eq!(parse("var"), Ok(AST::String("var".into())));
        assert_eq!(parse("\"var\""), Ok(AST::String("var".into())));
        // assert_eq!(parse("!!var"), Ok(AST::Variable("var".into())));
    }

    #[test]
    fn test_execute() {
        let record = Record::new("2024-01-01 00:00:00".to_string())
            .set_data("hostname", "localhost".to_string())
            .set_data("program", "test".to_string())
            .set_data("rest", "message".to_string())
            .set_data("var1", "10".to_string())
            .set_data("var2", "20".to_string());
        assert_eq!(
            execute(
                &AST::Equal(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(false),
        );
        assert_eq!(
            execute(
                &AST::GreaterThan(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(false),
        );
        assert_eq!(
            execute(
                &AST::LessThan(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(true),
        );
        assert_eq!(
            execute(&AST::Variable("var1".to_string()), &record),
            Value::Number(10)
        );
        assert_eq!(
            execute(&AST::Variable("var2".to_string()), &record),
            Value::Number(20)
        );
        assert_eq!(
            execute(
                &AST::Not(Box::new(AST::Variable("var2".to_string()))),
                &record
            ),
            Value::Boolean(false)
        );
        assert_eq!(
            execute(
                &AST::And(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(true),
        );
        assert_eq!(
            execute(
                &AST::Or(
                    Box::new(AST::Variable("var1".to_string())),
                    Box::new(AST::Variable("var2".to_string())),
                ),
                &record,
            ),
            Value::Boolean(true),
        );
    }
}
