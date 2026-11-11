use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VdfNode {
    Text(String),
    Map(Vec<(String, VdfNode)>),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VdfError {
    #[error("unterminated string")]
    UnterminatedString,
    #[error("unexpected end of file")]
    UnexpectedEnd,
    #[error("unexpected closing brace")]
    UnexpectedClose,
}

#[derive(Debug, PartialEq, Eq)]
enum Token {
    Text(String),
    Open,
    Close,
}

impl VdfNode {
    pub fn get(&self, key: &str) -> Option<&VdfNode> {
        let Self::Map(entries) = self else {
            return None;
        };
        entries
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .map(|(_, node)| node)
    }

    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Map(_) => None,
        }
    }

    pub fn entries(&self) -> &[(String, VdfNode)] {
        match self {
            Self::Map(entries) => entries,
            Self::Text(_) => &[],
        }
    }
}

pub fn parse_vdf(source: &str) -> Result<VdfNode, VdfError> {
    let tokens = tokenize(source)?;
    let mut position = 0;
    let entries = parse_entries(&tokens, &mut position, true)?;
    Ok(VdfNode::Map(entries))
}

fn tokenize(source: &str) -> Result<Vec<Token>, VdfError> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => tokens.push(Token::Open),
            '}' => tokens.push(Token::Close),
            '"' => tokens.push(Token::Text(read_quoted(&mut chars)?)),
            '/' if chars.peek() == Some(&'/') => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        break;
                    }
                }
            }
            c if c.is_whitespace() => {}
            c => {
                let mut word = String::from(c);
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() || matches!(next, '{' | '}' | '"') {
                        break;
                    }
                    word.push(next);
                    chars.next();
                }
                tokens.push(Token::Text(word));
            }
        }
    }
    Ok(tokens)
}

fn read_quoted(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<String, VdfError> {
    let mut text = String::new();
    loop {
        match chars.next().ok_or(VdfError::UnterminatedString)? {
            '"' => return Ok(text),
            '\\' => match chars.next().ok_or(VdfError::UnterminatedString)? {
                'n' => text.push('\n'),
                't' => text.push('\t'),
                other => text.push(other),
            },
            other => text.push(other),
        }
    }
}

fn parse_entries(
    tokens: &[Token],
    position: &mut usize,
    top_level: bool,
) -> Result<Vec<(String, VdfNode)>, VdfError> {
    let mut entries = Vec::new();
    loop {
        match tokens.get(*position) {
            None if top_level => return Ok(entries),
            None | Some(Token::Open) => return Err(VdfError::UnexpectedEnd),
            Some(Token::Close) if top_level => return Err(VdfError::UnexpectedClose),
            Some(Token::Close) => {
                *position += 1;
                return Ok(entries);
            }
            Some(Token::Text(key)) => {
                *position += 1;
                match tokens.get(*position) {
                    Some(Token::Text(value)) => {
                        entries.push((key.clone(), VdfNode::Text(value.clone())));
                        *position += 1;
                    }
                    Some(Token::Open) => {
                        *position += 1;
                        let children = parse_entries(tokens, position, false)?;
                        entries.push((key.clone(), VdfNode::Map(children)));
                    }
                    _ => return Err(VdfError::UnexpectedEnd),
                }
            }
        }
    }
}
