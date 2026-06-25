use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub enum Sexp {
    Atom { text: String, span: Span },
    List { items: Vec<Sexp>, span: Span },
}

impl Sexp {
    pub fn span(&self) -> Span {
        match self {
            Sexp::Atom { span, .. } | Sexp::List { span, .. } => *span,
        }
    }

    pub fn as_text(&self, source: &str) -> String {
        match self {
            Sexp::Atom { text, .. } => text.clone(),
            Sexp::List { span, .. } => source[span.start..span.end].to_string(),
        }
    }

    pub fn as_atom(&self) -> Option<&str> {
        if let Sexp::Atom { text, .. } = self {
            Some(text)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(source: &str) -> Result<Vec<Sexp>, ParseError> {
    let mut p = Lexer {
        bytes: source.as_bytes(),
        pos: 0,
    };
    let mut items = Vec::new();
    loop {
        p.skip_ws_comments();
        if p.pos >= p.bytes.len() {
            break;
        }
        items.push(p.parse_sexp()?);
    }
    Ok(items)
}

struct Lexer<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws_comments(&mut self) {
        loop {
            match self.peek() {
                Some(b) if b.is_ascii_whitespace() => self.pos += 1,
                Some(b';') if self.bytes.get(self.pos + 1) == Some(&b';') => {
                    while let Some(b) = self.peek() {
                        if b == b'\n' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn parse_sexp(&mut self) -> Result<Sexp, ParseError> {
        self.skip_ws_comments();
        match self.peek() {
            Some(b'(') => self.parse_list(),
            Some(b')') => Err(ParseError("unexpected )".into())),
            None => Err(ParseError("unexpected eof".into())),
            _ => self.parse_atom(),
        }
    }

    fn parse_list(&mut self) -> Result<Sexp, ParseError> {
        let start = self.pos;
        self.pos += 1; // consume '('
        let mut items = Vec::new();
        loop {
            self.skip_ws_comments();
            match self.peek() {
                None => return Err(ParseError("unclosed (".into())),
                Some(b')') => {
                    self.pos += 1;
                    break;
                }
                _ => items.push(self.parse_sexp()?),
            }
        }
        Ok(Sexp::List {
            items,
            span: Span {
                start,
                end: self.pos,
            },
        })
    }

    fn parse_atom(&mut self) -> Result<Sexp, ParseError> {
        let start = self.pos;
        match self.peek() {
            Some(b'"') => self.read_string(start, 0),
            Some(b'r') => {
                if let Some(n) = self.raw_hash_count() {
                    self.read_string(start, n)
                } else {
                    self.read_bare(start)
                }
            }
            _ => self.read_bare(start),
        }
    }

    fn raw_hash_count(&self) -> Option<usize> {
        // at pos is 'r', check following '#*'
        let mut i = self.pos + 1;
        let mut n = 0;
        while self.bytes.get(i) == Some(&b'#') {
            i += 1;
            n += 1;
        }
        if self.bytes.get(i) == Some(&b'"') {
            Some(n)
        } else {
            None
        }
    }

    fn read_string(&mut self, start: usize, hashes: usize) -> Result<Sexp, ParseError> {
        if hashes == 0 {
            // regular "..."
            self.pos += 1; // opening "
            loop {
                match self.peek() {
                    None => return Err(ParseError("unterminated string".into())),
                    Some(b'\\') => self.pos += 2,
                    Some(b'"') => {
                        self.pos += 1;
                        break;
                    }
                    _ => self.pos += 1,
                }
            }
        } else {
            // r#*"...""#*
            self.pos += 1 + hashes + 1; // r + #s + "
            loop {
                match self.peek() {
                    None => return Err(ParseError("unterminated raw string".into())),
                    Some(b'"') => {
                        // check followed by hashes
                        if self.bytes[self.pos + 1..self.pos + 1 + hashes]
                            .iter()
                            .all(|b| *b == b'#')
                        {
                            self.pos += 1 + hashes;
                            break;
                        }
                        self.pos += 1;
                    }
                    _ => self.pos += 1,
                }
            }
        }
        Ok(Sexp::Atom {
            text: String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned(),
            span: Span {
                start,
                end: self.pos,
            },
        })
    }

    fn read_bare(&mut self, start: usize) -> Result<Sexp, ParseError> {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() || b == b'(' || b == b')' {
                break;
            }
            self.pos += 1;
        }
        Ok(Sexp::Atom {
            text: String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned(),
            span: Span {
                start,
                end: self.pos,
            },
        })
    }
}
