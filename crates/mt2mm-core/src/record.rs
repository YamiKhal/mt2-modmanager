use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Label(String),
    Str(String),
    Num(String),
    Semicolon,
    Equals,
    Bare(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Record {
    pub label: Option<String>,
    pub tokens: Vec<Token>,
    pub children: Vec<Record>,
    pub had_block: bool,
    // parser state (mirrors vsRecord)
    in_block: bool,
    line_open: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    T(Token),
    Open,
    Close,
    NewLine,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub records: Vec<Record>,
    pub crlf: bool,
    pub latin1: bool,
}

#[derive(Debug)]
pub struct ParseError(pub String);


impl Token {
    pub fn text(&self) -> Option<&str> {
        match self {
            Token::Label(s) | Token::Str(s) | Token::Bare(s) | Token::Num(s) => Some(s),
            _ => None,
        }
    }

    pub fn is_semicolon(&self) -> bool {
        matches!(self, Token::Semicolon)
    }

    fn write(&self, out: &mut String) {
        match self {
            Token::Label(s) | Token::Num(s) | Token::Bare(s) => out.push_str(s),
            Token::Str(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        _ => out.push(c),
                    }
                }
                out.push('"');
            }
            Token::Semicolon => out.push(';'),
            Token::Equals => out.push('='),
        }
    }
}


fn is_ws(c: char) -> bool {
    c == ' ' || c == '\t' || c == ','
}

fn is_alpha(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_numeric(c: char) -> bool {
    c == '.' || c == '-' || c == '+' || c.is_ascii_digit()
}

fn is_alnum(c: char) -> bool {
    is_alpha(c) || is_numeric(c)
}

// How much of `s` std::stof would accept.
fn stof_prefix(s: &[char]) -> Option<usize> {
    let mut i = 0;
    if i < s.len() && (s[i] == '+' || s[i] == '-') {
        i += 1;
    }
    let int_start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    let mut digits = i - int_start;
    if i < s.len() && s[i] == '.' {
        i += 1;
        let frac_start = i;
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
        }
        digits += i - frac_start;
    }
    if digits == 0 {
        return None;
    }
    if i < s.len() && (s[i] == 'e' || s[i] == 'E') {
        let mut j = i + 1;
        if j < s.len() && (s[j] == '+' || s[j] == '-') {
            j += 1;
        }
        let exp_start = j;
        while j < s.len() && s[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_start {
            i = j;
        }
    }
    Some(i)
}

// How much of `s` std::stoi would accept.
fn stoi_prefix(s: &[char]) -> Option<usize> {
    let mut i = 0;
    if i < s.len() && (s[i] == '+' || s[i] == '-') {
        i += 1;
    }
    let start = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        None
    } else {
        Some(i)
    }
}

// Port of the engine's vsToken::ExtractFrom (github.com/vectorstorm/vectorstorm), so files split
// into tokens exactly the way the game splits them.
fn extract(s: &mut &[char]) -> Option<Tok> {
    while !s.is_empty() && is_ws(s[0]) {
        *s = &s[1..];
    }
    if s.is_empty() {
        return None;
    }
    let c = s[0];
    if c == '"' {
        let mut out = String::new();
        let mut i = 1;
        let mut escaped = false;
        while i < s.len() && s[i] != '\0' {
            let ch = s[i];
            if escaped {
                out.push(if ch == 'n' { '\n' } else { ch });
                escaped = false;
            } else if ch == '"' {
                break;
            } else if ch == '\\' {
                escaped = true;
            } else {
                out.push(ch);
            }
            i += 1;
        }
        *s = if i < s.len() { &s[i + 1..] } else { &s[s.len()..] };
        return Some(Tok::T(Token::Str(out)));
    }
    if c == '{' {
        *s = &s[1..];
        return Some(Tok::Open);
    }
    if c == '}' {
        *s = &s[1..];
        return Some(Tok::Close);
    }
    if c == ';' {
        *s = &s[1..];
        return Some(Tok::T(Token::Semicolon));
    }
    if is_alpha(c) {
        let mut i = 0;
        while i < s.len() && is_alnum(s[i]) {
            i += 1;
        }
        let label: String = s[..i].iter().collect();
        *s = &s[i..];
        return Some(Tok::T(Token::Label(label)));
    }
    if is_numeric(c) {
        // PeekNumberToken: run of numeric chars; float only if it contains '.'
        let mut run = 0;
        while run < s.len() && is_numeric(s[run]) {
            run += 1;
        }
        let has_dot = s[..run].contains(&'.');
        if has_dot {
            if let Some(n) = stof_prefix(s) {
                let t: String = s[..n].iter().collect();
                *s = &s[n..];
                return Some(Tok::T(Token::Num(t)));
            }
        }
        if let Some(n) = stoi_prefix(s) {
            let t: String = s[..n].iter().collect();
            *s = &s[n..];
            return Some(Tok::T(Token::Num(t)));
        }
    }
    if c == '#' {
        *s = &s[s.len()..];
        return None;
    }
    if c == '=' {
        *s = &s[1..];
        return Some(Tok::T(Token::Equals));
    }
    let mut i = 0;
    while i < s.len() && !is_ws(s[i]) {
        i += 1;
    }
    let t: String = s[..i].iter().collect();
    *s = &s[i..];
    Some(Tok::T(Token::Bare(t)))
}


impl Record {
    pub fn new() -> Self {
        Record { line_open: true, ..Default::default() }
    }

    pub fn leaf(label: &str, tokens: Vec<Token>) -> Self {
        Record { label: Some(label.to_string()), tokens, ..Default::default() }
    }

    // vsRecord::AppendToken
    fn append(&mut self, t: Tok) -> bool {
        if !self.in_block {
            match t {
                Tok::T(Token::Label(l)) if self.tokens.is_empty() && self.label.is_none() => {
                    self.label = Some(l);
                }
                Tok::Open => {
                    self.in_block = true;
                    self.had_block = true;
                }
                Tok::Close => return false,
                Tok::NewLine => {
                    if self.label.is_some() || !self.tokens.is_empty() {
                        if self.line_open {
                            self.line_open = false;
                            return true;
                        }
                        return false;
                    }
                }
                Tok::T(tok) => {
                    if self.line_open {
                        self.tokens.push(tok);
                    } else {
                        return false;
                    }
                }
            }
        } else {
            match t {
                Tok::Close => {
                    let taken = match self.children.last_mut() {
                        Some(c) => c.append(Tok::Close),
                        None => false,
                    };
                    if !taken {
                        self.in_block = false;
                        self.line_open = false;
                        return true;
                    }
                }
                Tok::NewLine => {
                    if let Some(c) = self.children.last_mut() {
                        c.append(Tok::NewLine);
                        return true;
                    }
                }
                other => {
                    let taken = match self.children.last_mut() {
                        Some(c) => c.append(other.clone()),
                        None => false,
                    };
                    if !taken {
                        let mut c = Record::new();
                        c.append(other);
                        self.children.push(c);
                    }
                }
            }
        }
        true
    }

    // vsRecord::ParseString
    fn parse_line(&mut self, line: &[char]) -> bool {
        let mut s = line;
        let mut valid = false;
        while !s.is_empty() {
            if let Some(t) = extract(&mut s) {
                valid = true;
                self.append(t);
            }
        }
        self.append(Tok::NewLine);
        valid
    }

    pub fn values(&self) -> Vec<&Token> {
        self.tokens.iter().filter(|t| !t.is_semicolon()).collect()
    }

    pub fn first_text(&self) -> Option<&str> {
        self.values().first().and_then(|t| t.text())
    }

    pub fn prop(&self, name: &str) -> Option<&str> {
        self.children.iter().find(|c| c.label.as_deref() == Some(name)).and_then(|c| c.first_text())
    }

    pub fn child_mut(&mut self, name: &str) -> Option<&mut Record> {
        self.children.iter_mut().find(|c| c.label.as_deref() == Some(name))
    }

    pub fn for_each_token_mut(&mut self, f: &mut dyn FnMut(Option<&str>, &mut Token)) {
        let label = self.label.clone();
        for t in self.tokens.iter_mut() {
            f(label.as_deref(), t);
        }
        for c in self.children.iter_mut() {
            c.for_each_token_mut(f);
        }
    }

    fn write(&self, out: &mut String, depth: usize) {
        let tabs = "\t".repeat(depth);
        out.push_str(&tabs);
        let mut first = true;
        if let Some(l) = &self.label {
            out.push_str(l);
            first = false;
        }
        for t in &self.tokens {
            if !first && !t.is_semicolon() {
                out.push(' ');
            }
            t.write(out);
            first = false;
        }
        if !self.children.is_empty() || self.had_block {
            out.push('\n');
            let _ = writeln!(out, "{tabs}{{");
            for c in &self.children {
                c.write(out, depth + 1);
            }
            out.push_str(&tabs);
            out.push('}');
        }
        out.push('\n');
    }
}


impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}


fn decode(bytes: &[u8]) -> (String, bool) {
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_string(), false),
        Err(_) => (bytes.iter().map(|&b| b as char).collect(), true),
    }
}

fn split_lines(text: &str) -> Vec<Vec<char>> {
    // vsFile::ReadLine: split at '\n' or NUL, drop '\r'; a final empty segment after a
    // trailing newline is not a line.
    let mut lines = Vec::new();
    let mut cur: Vec<char> = Vec::new();
    let mut any = false;
    for c in text.chars() {
        any = true;
        if c == '\n' || c == '\0' {
            lines.push(std::mem::take(&mut cur));
            any = false;
        } else if c != '\r' {
            cur.push(c);
        }
    }
    if any {
        lines.push(cur);
    }
    lines
}

fn first_tok_is_open(line: &[char]) -> bool {
    let mut s = line;
    matches!(extract(&mut s), Some(Tok::Open))
}

pub fn parse(bytes: &[u8]) -> Result<Document, ParseError> {
    let (text, latin1) = decode(bytes);
    let crlf = text.contains("\r\n");
    let lines = split_lines(&text);
    let mut i = 0;
    let mut records = Vec::new();
    loop {
        let mut r = Record::new();
        let mut valid = false;
        let mut done = false;
        let mut have_line = true;
        while have_line && (!valid || !done) {
            done = true;
            if i < lines.len() {
                let mut line = lines[i].clone();
                i += 1;
                if i < lines.len() && first_tok_is_open(&lines[i]) {
                    line.extend_from_slice(&lines[i]);
                    i += 1;
                }
                valid = r.parse_line(&line);
                if r.in_block {
                    done = false;
                }
            } else {
                have_line = false;
                if r.in_block {
                    return Err(ParseError(format!(
                        "reached end of file inside an unclosed block (record '{}')",
                        r.label.as_deref().unwrap_or("?")
                    )));
                }
            }
        }
        if !valid {
            break;
        }
        mark_blocks(&mut r);
        records.push(r);
    }
    Ok(Document { records, crlf, latin1 })
}

fn mark_blocks(r: &mut Record) {
    for c in r.children.iter_mut() {
        mark_blocks(c);
    }
    r.in_block = false;
    r.line_open = false;
}


impl Document {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut s = String::new();
        for r in &self.records {
            r.write(&mut s, 0);
        }
        if self.crlf {
            s = s.replace('\n', "\r\n");
        }
        if self.latin1 {
            s.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect()
        } else {
            s.into_bytes()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Vec<Record> {
        parse(s.as_bytes()).unwrap().records
    }

    #[test]
    fn brace_on_next_line_and_props() {
        let r = p("mmoTech\n{\n\tname \"Parties\";\n\tcost 20000;\n\targ 0.5\n}\n");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].label.as_deref(), Some("mmoTech"));
        assert_eq!(r[0].prop("name"), Some("Parties"));
        assert_eq!(r[0].prop("cost"), Some("20000"));
        assert_eq!(r[0].prop("arg"), Some("0.5"));
    }

    #[test]
    fn inline_block_nested_and_empty() {
        let r = p("gameValueMod {\n\tmmoGameValueMod\n\t{\n\t\tname \"x\";\n\t}\n}\nlogic { mmoModsWindowLogic { } }\n");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].children[0].label.as_deref(), Some("mmoGameValueMod"));
        assert_eq!(r[0].children[0].prop("name"), Some("x"));
        let inner = &r[1].children[0];
        assert_eq!(inner.label.as_deref(), Some("mmoModsWindowLogic"));
        assert!(inner.had_block && inner.children.is_empty());
        // empty block survives a rewrite
        let out = String::from_utf8(Document { records: r.clone(), crlf: false, latin1: false }.to_bytes()).unwrap();
        assert_eq!(p(&out), r);
    }

    #[test]
    fn comments_commas_and_strings() {
        let r = p("# header\nInventory\n{\n\tItem \"@A\";\n\t# Item \"@B\";\n\tItem \"@C # not a comment\"\n}\ndims 200, 160;\n");
        assert_eq!(r.len(), 2);
        let items: Vec<_> = r[0].children.iter().map(|c| c.first_text().unwrap().to_string()).collect();
        assert_eq!(items, vec!["@A", "@C # not a comment"]);
        let v: Vec<_> = r[1].values().iter().map(|t| t.text().unwrap().to_string()).collect();
        assert_eq!(v, vec!["200", "160"]);
    }

    #[test]
    fn numbers_and_labels() {
        let r = p("x 240.f -3 1.5e3 .5 label.with-dots\n");
        let v = &r[0].tokens;
        assert_eq!(v[0], Token::Num("240.".into()));
        assert_eq!(v[1], Token::Label("f".into()));
        assert_eq!(v[2], Token::Num("-3".into()));
        assert_eq!(v[3], Token::Num("1.5e3".into()));
        assert_eq!(v[4], Token::Num(".5".into()));
        assert_eq!(v[5], Token::Label("label.with-dots".into()));
    }

    #[test]
    fn escapes_roundtrip() {
        let r = p(r#"t "a\nb \"q\" c\\d""#);
        assert_eq!(r[0].first_text(), Some("a\nb \"q\" c\\d"));
        let out = Document { records: r.clone(), crlf: false, latin1: false }.to_bytes();
        assert_eq!(p(std::str::from_utf8(&out).unwrap()), r);
    }

    #[test]
    fn crlf_and_word_list() {
        let d = parse(b"Warrior\r\nDark Knight\r\n\r\nMage\r\n").unwrap();
        assert!(d.crlf);
        assert_eq!(d.records.len(), 3);
        assert_eq!(d.records[1].label.as_deref(), Some("Dark"));
        assert_eq!(d.to_bytes(), b"Warrior\r\nDark Knight\r\nMage\r\n");
    }
}
