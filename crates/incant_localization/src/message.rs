use crate::{DateLength, LocalizationError, MAX_MESSAGE_BYTES};
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) enum Node {
    Text(String),
    Argument(String),
    Number(String),
    Date(String, DateLength),
    Pound,
    Select {
        name: String,
        branches: BTreeMap<String, Vec<Node>>,
    },
    Plural {
        name: String,
        ordinal: bool,
        offset: u32,
        exact: Vec<(f64, Vec<Node>)>,
        branches: BTreeMap<String, Vec<Node>>,
    },
}
pub(crate) fn parse(source: &str) -> Result<Vec<Node>, LocalizationError> {
    if source.len() > MAX_MESSAGE_BYTES {
        return Err(LocalizationError::Pattern {
            offset: 0,
            message: "pattern exceeds 8192 bytes".into(),
        });
    }
    let mut parser = Parser {
        source,
        pos: 0,
        nodes: 0,
    };
    parser.message(0, false, false)
}
struct Parser<'a> {
    source: &'a str,
    pos: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn error(&self, message: &str) -> LocalizationError {
        LocalizationError::Pattern {
            offset: self.pos,
            message: message.into(),
        }
    }
    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }
    fn take(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }
    fn spaces(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.take();
        }
    }
    fn expect(&mut self, c: char) -> Result<(), LocalizationError> {
        self.spaces();
        if self.take() == Some(c) {
            Ok(())
        } else {
            Err(self.error("unexpected delimiter"))
        }
    }
    fn token(&mut self) -> Result<String, LocalizationError> {
        self.spaces();
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || "_.:-=+".contains(c))
        {
            self.take();
        }
        if self.pos == start || self.pos - start > 128 {
            return Err(self.error("invalid or oversized token"));
        }
        Ok(self.source[start..self.pos].into())
    }
    fn push(&mut self, nodes: &mut Vec<Node>, node: Node) -> Result<(), LocalizationError> {
        self.nodes += 1;
        if self.nodes > 1024 {
            return Err(self.error("pattern exceeds 1024 nodes"));
        }
        nodes.push(node);
        Ok(())
    }
    fn message(
        &mut self,
        depth: usize,
        nested: bool,
        plural: bool,
    ) -> Result<Vec<Node>, LocalizationError> {
        if depth > 16 {
            return Err(self.error("pattern nesting exceeds 16"));
        }
        let mut nodes = vec![];
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c == '}' {
                if nested {
                    break;
                } else {
                    return Err(self.error("unmatched closing brace"));
                }
            }
            if c == '{' || (c == '#' && plural) {
                if !text.is_empty() {
                    self.push(&mut nodes, Node::Text(std::mem::take(&mut text)))?;
                }
                self.take();
                let node = if c == '#' {
                    Node::Pound
                } else {
                    self.argument(depth + 1, plural)?
                };
                self.push(&mut nodes, node)?;
            } else if c == '\'' {
                self.take();
                if self.peek() == Some('\'') {
                    self.take();
                    text.push('\'');
                } else if self
                    .peek()
                    .is_some_and(|c| c == '{' || c == '}' || (c == '#' && plural))
                {
                    // ICU's apostrophe-friendly mode: a trailing quote closes at EOF.
                    while let Some(q) = self.take() {
                        if q == '\'' {
                            if self.peek() == Some('\'') {
                                self.take();
                                text.push('\'');
                            } else {
                                break;
                            }
                        } else {
                            text.push(q);
                        }
                    }
                } else {
                    text.push('\'');
                }
            } else {
                text.push(self.take().unwrap());
            }
        }
        if nested && self.peek().is_none() {
            return Err(self.error("unclosed branch"));
        }
        if !text.is_empty() {
            self.push(&mut nodes, Node::Text(text))?;
        }
        Ok(nodes)
    }
    fn argument(
        &mut self,
        depth: usize,
        enclosing_plural: bool,
    ) -> Result<Node, LocalizationError> {
        let name = self.token()?;
        if !crate::types::key(&name) {
            return Err(self.error("invalid argument name"));
        }
        self.spaces();
        if self.peek() == Some('}') {
            self.take();
            return Ok(Node::Argument(name));
        }
        self.expect(',')?;
        let kind = self.token()?;
        self.spaces();
        match kind.as_str() {
            "number" => {
                self.expect('}')?;
                Ok(Node::Number(name))
            }
            "date" => {
                let length = if self.peek() == Some(',') {
                    self.take();
                    match self.token()?.as_str() {
                        "short" => DateLength::Short,
                        "medium" => DateLength::Medium,
                        "long" => DateLength::Long,
                        _ => return Err(self.error("unsupported date style")),
                    }
                } else {
                    DateLength::Medium
                };
                self.expect('}')?;
                Ok(Node::Date(name, length))
            }
            "select" | "plural" | "selectordinal" => {
                self.expect(',')?;
                self.spaces();
                let is_plural = kind != "select";
                let mut offset = 0;
                if is_plural && self.source[self.pos..].starts_with("offset:") {
                    self.pos += 7;
                    let token = self.token()?;
                    offset = token
                        .parse::<u32>()
                        .map_err(|_| self.error("offset must be an unsigned integer"))?;
                }
                let mut exact = vec![];
                let mut branches = BTreeMap::new();
                loop {
                    self.spaces();
                    if self.peek() == Some('}') {
                        self.take();
                        break;
                    }
                    let selector = self.token()?;
                    self.expect('{')?;
                    let nodes = self.message(depth, true, is_plural || enclosing_plural)?;
                    self.expect('}')?;
                    if is_plural && selector.starts_with('=') {
                        let number = selector[1..]
                            .parse::<f64>()
                            .map_err(|_| self.error("invalid exact plural selector"))?;
                        if !number.is_finite()
                            || number.abs() > 9_007_199_254_740_991.
                            || exact.iter().any(|(n, _)| *n == number)
                        {
                            return Err(self.error("invalid or duplicate exact selector"));
                        }
                        exact.push((number, nodes));
                    } else {
                        if is_plural
                            && !["zero", "one", "two", "few", "many", "other"]
                                .contains(&selector.as_str())
                        {
                            return Err(self.error("unknown plural category"));
                        }
                        if !crate::types::key(&selector)
                            || branches.insert(selector, nodes).is_some()
                        {
                            return Err(self.error("invalid or duplicate selector"));
                        }
                    }
                    if exact.len() + branches.len() > 64 {
                        return Err(self.error("at most 64 branches"));
                    }
                }
                if !branches.contains_key("other") {
                    return Err(self.error("other branch is required"));
                }
                if is_plural {
                    Ok(Node::Plural {
                        name,
                        ordinal: kind == "selectordinal",
                        offset,
                        exact,
                        branches,
                    })
                } else {
                    Ok(Node::Select { name, branches })
                }
            }
            _ => Err(self.error("unsupported MessageFormat argument type")),
        }
    }
}
