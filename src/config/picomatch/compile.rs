// Token grammar adapted from picomatch 2.3.2; see LICENSES/picomatch-MIT.txt.
use super::{
    ConfigError,
    safety::{self, Repetition},
};

const STAR: &str = "[^/]*?";
const GLOBSTAR: &str = r"(?:(?:(?!(?:^|\/)\.{1,2}(?:\/|$)).)*?)";
const ONE: &str = "(?=.)";
const NO_DOTS: &str = r"(?!\.{1,2}(?:\/|$))";
const NO_DOT_SLASH: &str = r"(?!\.{0,1}(?:\/|$))";

#[derive(Clone, Default)]
struct Token {
    kind: &'static str,
    value: String,
    output: Option<String>,
    prev: usize,
    star: bool,
    posix: bool,
    dots: bool,
    comma: bool,
    output_index: usize,
    tokens_index: usize,
    extglob: bool,
}

impl Token {
    fn new(kind: &'static str, value: impl Into<String>, output: Option<String>) -> Self {
        Self {
            kind,
            value: value.into(),
            output,
            ..Self::default()
        }
    }
    fn rendered(&self) -> &str {
        self.output.as_deref().unwrap_or(&self.value)
    }
}

struct Extglob {
    operator: char,
    prev: usize,
    parens: i32,
    output: String,
    start: usize,
    tokens_index: usize,
    inner: String,
}

pub(super) struct Compiled {
    pub output: String,
    pub negated: bool,
    pub negative_extglob: bool,
}

struct Compiler {
    input: Vec<char>,
    index: usize,
    start: usize,
    output: String,
    arena: Vec<Token>,
    order: Vec<usize>,
    prev: usize,
    parens: i32,
    brackets: usize,
    brace_count: usize,
    braces: Vec<usize>,
    stack: Vec<&'static str>,
    extglobs: Vec<Extglob>,
    quote: bool,
    backtrack: bool,
    negated: bool,
    negative_extglob: bool,
    posix: bool,
    depth: usize,
}

pub(super) fn escape(text: &str) -> String {
    text.chars()
        .flat_map(|ch| {
            if r"\-[]{}()*+?.^$|".contains(ch) {
                vec!['\\', ch]
            } else {
                vec![ch]
            }
        })
        .collect()
}

fn fast(input: &str) -> Option<String> {
    if input.starts_with(['*', '!']) || input.contains(['/', '(', ')', '[', ']', '{', '}', '"']) {
        return None;
    }
    let mut chars = input.chars().peekable();
    let mut output = String::new();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                output.push(ch);
                if let Some(next) = chars.next() {
                    output.push(next);
                }
            }
            '?' => output.push_str("[^/]"),
            '*' => {
                output.push_str(STAR);
                while chars.peek() == Some(&'*') {
                    chars.next();
                }
            }
            _ => output.push_str(&escape(&ch.to_string())),
        }
    }
    let mut collapsed = String::new();
    let mut chars = output.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            collapsed.push(ch);
            continue;
        }
        let mut count = 1;
        while chars.peek() == Some(&'\\') {
            chars.next();
            count += 1;
        }
        collapsed.push('\\');
        if count % 2 == 0 {
            collapsed.push('\\');
        }
    }
    Some(collapsed)
}

pub(super) fn compile(pattern: &str, posix: bool, depth: usize) -> Result<Compiled, ConfigError> {
    if pattern.encode_utf16().take(65_537).count() > 65_536 || depth > 64 {
        return Err(ConfigError::invalid(
            pattern,
            "glob length or recursive nesting limit exceeded",
        ));
    }
    let input = match pattern {
        "***" => "*",
        "**/**" | "**/**/**" => "**",
        _ => pattern,
    };
    let input = input.strip_prefix("./").unwrap_or(input);
    if depth == 0 {
        if pattern.starts_with(['.', '*'])
            && let Some(output) = common_fast(input)
        {
            return Ok(Compiled {
                output: format!("{output}\\/?"),
                negated: false,
                negative_extglob: false,
            });
        }
        if let Some(output) = fast(input) {
            return Ok(Compiled {
                output,
                negated: false,
                negative_extglob: false,
            });
        }
    }
    let mut compiler = Compiler {
        input: input.chars().collect(),
        index: 0,
        start: 0,
        output: String::new(),
        arena: vec![Token::new("bos", "", Some(String::new()))],
        order: vec![0],
        prev: 0,
        parens: 0,
        brackets: 0,
        brace_count: 0,
        braces: Vec::new(),
        stack: Vec::new(),
        extglobs: Vec::new(),
        quote: false,
        backtrack: false,
        negated: false,
        negative_extglob: false,
        posix,
        depth,
    };
    compiler.run()?;
    Ok(Compiled {
        output: compiler.output,
        negated: compiler.negated,
        negative_extglob: compiler.negative_extglob,
    })
}

fn common_fast(input: &str) -> Option<String> {
    Some(match input {
        "*" => format!("(?!(?:^|\\/)\\.{{1,2}}(?:\\/|$)){ONE}{STAR}"),
        ".*" => format!("\\.{ONE}{STAR}"),
        "*.*" => format!("(?!(?:^|\\/)\\.{{1,2}}(?:\\/|$)){STAR}\\.{ONE}{STAR}"),
        "*/*" => format!("(?!(?:^|\\/)\\.{{1,2}}(?:\\/|$)){STAR}\\/{ONE}{NO_DOTS}{STAR}"),
        "**" => format!("(?!(?:^|\\/)\\.{{1,2}}(?:\\/|$)){GLOBSTAR}"),
        "**/*" => format!("(?:(?!(?:^|\\/)\\.{{1,2}}(?:\\/|$)){GLOBSTAR}\\/)?{NO_DOTS}{ONE}{STAR}"),
        "**/*.*" => format!(
            "(?:(?!(?:^|\\/)\\.{{1,2}}(?:\\/|$)){GLOBSTAR}\\/)?{NO_DOTS}{STAR}\\.{ONE}{STAR}"
        ),
        "**/.*" => format!("(?:(?!(?:^|\\/)\\.{{1,2}}(?:\\/|$)){GLOBSTAR}\\/)?\\.{ONE}{STAR}"),
        _ => {
            let (prefix, extension) = input.rsplit_once('.')?;
            if extension.is_empty()
                || !extension
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return None;
            }
            format!("{}\\.{extension}", common_fast(prefix)?)
        }
    })
}

impl Compiler {
    fn peek(&self, n: usize) -> char {
        self.input.get(self.index + n).copied().unwrap_or('\0')
    }
    fn remaining(&self) -> String {
        self.input[self.index..].iter().collect()
    }
    fn advance(&mut self) -> char {
        let ch = self.peek(0);
        self.index = (self.index + 1).min(self.input.len());
        ch
    }
    fn push(&mut self, mut token: Token) {
        if self.arena[self.prev].kind == "globstar" {
            let brace = self.brace_count > 0 && matches!(token.kind, "comma" | "brace");
            let extglob = token.extglob
                || !self.extglobs.is_empty() && matches!(token.kind, "pipe" | "paren");
            if !matches!(token.kind, "slash" | "paren") && !brace && !extglob {
                let length = self.arena[self.prev].rendered().len();
                self.output
                    .truncate(self.output.len().saturating_sub(length));
                let prev = &mut self.arena[self.prev];
                prev.kind = "star";
                prev.value = "*".into();
                prev.output = Some(STAR.into());
                self.output.push_str(STAR);
            }
        }
        if token.kind != "paren"
            && let Some(extglob) = self.extglobs.last_mut()
        {
            extglob.inner.push_str(&token.value);
        }
        self.output.push_str(token.rendered());
        if self.arena[self.prev].kind == "text" && token.kind == "text" {
            let prev = &mut self.arena[self.prev];
            prev.value.push_str(&token.value);
            prev.output.get_or_insert_default().push_str(&token.value);
            return;
        }
        token.prev = self.prev;
        self.prev = self.arena.len();
        self.arena.push(token);
        self.order.push(self.prev);
    }
    fn emit(&mut self, kind: &'static str, value: impl Into<String>, output: impl Into<String>) {
        self.push(Token::new(kind, value, Some(output.into())));
    }
    fn increment(&mut self, kind: &'static str) -> Result<(), ConfigError> {
        if self.stack.len() >= 64 {
            return Err(ConfigError::invalid("glob", "glob nesting limit exceeded"));
        }
        self.stack.push(kind);
        match kind {
            "parens" => self.parens += 1,
            "brackets" => self.brackets += 1,
            "braces" => self.brace_count += 1,
            _ => unreachable!(),
        }
        Ok(())
    }
    fn decrement(&mut self, kind: &str) {
        self.stack.pop();
        match kind {
            "parens" => self.parens -= 1,
            "brackets" => self.brackets = self.brackets.saturating_sub(1),
            "braces" => self.brace_count = self.brace_count.saturating_sub(1),
            _ => unreachable!(),
        }
    }
    fn extglob_open(&mut self, operator: char) -> Result<(), ConfigError> {
        let extglob = Extglob {
            operator,
            prev: self.prev,
            parens: self.parens,
            output: self.output.clone(),
            start: self.index - 1,
            tokens_index: self.order.len(),
            inner: String::new(),
        };
        self.increment("parens")?;
        self.emit(
            match operator {
                '?' => "qmark",
                '+' => "plus",
                '*' => "star",
                '!' => "negate",
                _ => unreachable!(),
            },
            operator.to_string(),
            if self.output.is_empty() { ONE } else { "" },
        );
        let open = self.advance();
        let mut token = Token::new(
            "paren",
            open.to_string(),
            Some(if operator == '!' { "(?:(?!(?:" } else { "(?:" }.into()),
        );
        token.extglob = true;
        self.push(token);
        self.extglobs.push(extglob);
        Ok(())
    }
    fn extglob_close(&mut self, extglob: Extglob) -> Result<(), ConfigError> {
        let body: String = self.input[extglob.start + 2..self.index - 1]
            .iter()
            .collect();
        if matches!(extglob.operator, '+' | '*') {
            let replacement = match safety::analyze(&body) {
                Repetition::Safe => None,
                Repetition::Literal => Some(escape(
                    &self.input[extglob.start..self.index]
                        .iter()
                        .collect::<String>(),
                )),
                Repetition::Collapsed(output) => Some(format!(
                    "{}{output}",
                    if extglob.output.is_empty() { ONE } else { "" }
                )),
            };
            if let Some(replacement) = replacement {
                let open = self.order[extglob.tokens_index];
                self.arena[open].kind = "text";
                self.arena[open].value = self.input[extglob.start..self.index].iter().collect();
                self.arena[open].output = Some(replacement.clone());
                for &index in &self.order[extglob.tokens_index + 1..] {
                    self.arena[index].value.clear();
                    self.arena[index].output = Some(String::new());
                }
                self.output = extglob.output + &replacement;
                self.backtrack = true;
                let mut token = Token::new("paren", ")", Some(String::new()));
                token.extglob = true;
                self.push(token);
                self.decrement("parens");
                return Ok(());
            }
        }
        let mut output = match extglob.operator {
            '?' => ")?".into(),
            '+' => ")+".into(),
            '*' => ")*".into(),
            '!' => format!(")){STAR})"),
            _ => unreachable!(),
        };
        if extglob.operator == '!' {
            let star = if extglob.inner.len() > 1 && extglob.inner.contains('/') {
                GLOBSTAR
            } else {
                STAR
            };
            let rest = self.remaining();
            if star != STAR || rest.is_empty() || rest.chars().all(|ch| ch == ')') {
                output = format!(")$)){star}");
            }
            if extglob.inner.contains('*')
                && rest.starts_with('.')
                && rest.len() > 1
                && !rest[1..].contains(['\\', '/', '.'])
            {
                let expression = compile(&rest, self.posix, self.depth + 1)?.output;
                output = format!("){expression}){star})");
            }
            if self.arena[extglob.prev].kind == "bos" {
                self.negative_extglob = true;
            }
        }
        let mut token = Token::new("paren", ")", Some(output));
        token.extglob = true;
        self.push(token);
        self.decrement("parens");
        Ok(())
    }

    fn run(&mut self) -> Result<(), ConfigError> {
        while self.index < self.input.len() {
            if self.output.len() > 1_048_576 || self.arena.len() > 65_536 {
                return Err(ConfigError::invalid(
                    "glob",
                    "glob compile allocation limit exceeded",
                ));
            }
            let ch = self.advance();
            if ch == '\0' {
                continue;
            }
            let mut value = ch.to_string();
            if ch == '\\' {
                let next = self.peek(0);
                if matches!(next, '/' | '.' | ';') {
                    continue;
                }
                if next == '\0' {
                    self.emit("text", "\\\\", "\\\\");
                    continue;
                }
                let slashes = self.input[self.index..]
                    .iter()
                    .take_while(|&&ch| ch == '\\')
                    .count();
                if slashes > 2 {
                    self.index += slashes;
                    if slashes % 2 != 0 {
                        value.push('\\');
                    }
                }
                let next = self.advance();
                if next != '\0' {
                    value.push(next);
                }
                if self.brackets == 0 {
                    self.push(Token::new("text", value, None));
                    continue;
                }
            }
            if self.brackets > 0
                && (ch != ']' || matches!(self.arena[self.prev].value.as_str(), "[" | "[^"))
            {
                if ch == ':' {
                    let previous = self.arena[self.prev].value.clone();
                    let inner = previous.get(1..).unwrap_or("");
                    if inner.contains('[') {
                        self.arena[self.prev].posix = true;
                        if inner.contains(':')
                            && let Some(index) = previous.rfind('[')
                            && let Some(class) = posix_class(&previous[index + 2..])
                        {
                            self.arena[self.prev].value = format!("{}{class}", &previous[..index]);
                            self.backtrack = true;
                            self.advance();
                            if self.arena[0].output.as_ref().is_none_or(|s| s.is_empty())
                                && self.order.get(1) == Some(&self.prev)
                            {
                                self.arena[0].output = Some(ONE.into());
                            }
                            continue;
                        }
                    }
                }
                if ch == '[' && self.peek(0) != ':'
                    || ch == '-' && self.peek(0) == ']'
                    || ch == ']' && matches!(self.arena[self.prev].value.as_str(), "[" | "[^")
                {
                    value.insert(0, '\\');
                }
                if self.posix && ch == '!' && self.arena[self.prev].value == "[" {
                    value = "^".into();
                }
                self.arena[self.prev].value.push_str(&value);
                self.output.push_str(&value);
                continue;
            }
            if self.quote && ch != '"' {
                value = escape(&value);
                self.arena[self.prev].value.push_str(&value);
                self.output.push_str(&value);
                continue;
            }
            if ch == '"' {
                self.quote = !self.quote;
                continue;
            }
            match ch {
                '(' => {
                    self.increment("parens")?;
                    self.push(Token::new("paren", value, None));
                }
                ')' => {
                    if let Some(extglob) = self.extglobs.pop_if(|e| self.parens == e.parens + 1) {
                        self.extglob_close(extglob)?;
                        continue;
                    }
                    self.emit("paren", value, if self.parens != 0 { ")" } else { r"\)" });
                    self.decrement("parens");
                }
                '[' => {
                    if !self.input[self.index..].contains(&']') {
                        self.emit("bracket", r"\[", r"\[");
                    } else {
                        self.increment("brackets")?;
                        self.push(Token::new("bracket", value, None));
                    }
                }
                ']' => {
                    if self.brackets == 0
                        || self.arena[self.prev].kind == "bracket"
                            && self.arena[self.prev].value.len() == 1
                    {
                        self.emit("text", value, r"\]");
                        continue;
                    }
                    self.decrement("brackets");
                    let previous = self.arena[self.prev].value[1..].to_owned();
                    if !self.arena[self.prev].posix
                        && previous.starts_with('^')
                        && !previous.contains('/')
                    {
                        value.insert(0, '/');
                    }
                    self.arena[self.prev].value.push_str(&value);
                    self.output.push_str(&value);
                    if previous.contains([
                        '-', '*', '+', '?', '.', '^', '$', '{', '}', '(', ')', '|', '[', ']', '\\',
                    ]) {
                        continue;
                    }
                    let previous = self.arena[self.prev].value.clone();
                    self.output
                        .truncate(self.output.len().saturating_sub(previous.len()));
                    self.arena[self.prev].value = format!("(?:{}|{previous})", escape(&previous));
                    self.output.push_str(&self.arena[self.prev].value);
                }
                '{' => {
                    self.increment("braces")?;
                    let mut token = Token::new("brace", value, Some("(".into()));
                    token.output_index = self.output.len();
                    token.tokens_index = self.order.len();
                    self.push(token);
                    self.braces.push(self.prev);
                }
                '}' => {
                    let Some(brace) = self.braces.last().copied() else {
                        self.push(Token::new("text", value, None));
                        continue;
                    };
                    let mut output = ")".to_owned();
                    if self.arena[brace].dots {
                        let mut range = Vec::new();
                        while let Some(index) = self.order.pop() {
                            if self.arena[index].kind == "brace" {
                                break;
                            }
                            if self.arena[index].kind != "dots" {
                                range.insert(0, self.arena[index].value.clone());
                            }
                        }
                        range.sort();
                        output = format!("[{}]", range.join("-"));
                        if regress::Regex::from_unicode(
                            super::regex_units(&output).into_iter(),
                            regress::Flags::default(),
                        )
                        .is_err()
                        {
                            output = range
                                .iter()
                                .map(|v| escape(v))
                                .collect::<Vec<_>>()
                                .join("..");
                        }
                        self.backtrack = true;
                    }
                    if !self.arena[brace].comma && !self.arena[brace].dots {
                        self.output.truncate(self.arena[brace].output_index);
                        self.arena[brace].value = r"\{".into();
                        self.arena[brace].output = Some(r"\{".into());
                        value = r"\}".into();
                        output = value.clone();
                        for &index in &self.order[self.arena[brace].tokens_index..] {
                            self.output.push_str(self.arena[index].rendered());
                        }
                    }
                    self.emit("brace", value, output);
                    self.decrement("braces");
                    self.braces.pop();
                }
                '|' => self.push(Token::new("text", value, None)),
                ',' => {
                    let mut output = value.clone();
                    if self.stack.last() == Some(&"braces")
                        && let Some(&brace) = self.braces.last()
                    {
                        self.arena[brace].comma = true;
                        output = "|".into();
                    }
                    self.emit("comma", value, output);
                }
                '/' => {
                    if self.arena[self.prev].kind == "dot" && self.index - 1 == self.start + 1 {
                        self.start = self.index;
                        self.output.clear();
                        self.order.pop();
                        self.prev = 0;
                        continue;
                    }
                    self.emit("slash", value, r"\/");
                }
                '.' => {
                    if self.brace_count > 0 && self.arena[self.prev].kind == "dot" {
                        let prev = &mut self.arena[self.prev];
                        if prev.value == "." {
                            prev.output = Some(r"\.".into());
                        }
                        prev.kind = "dots";
                        prev.output.get_or_insert_default().push('.');
                        prev.value.push('.');
                        self.arena[*self.braces.last().unwrap()].dots = true;
                        continue;
                    }
                    self.emit(
                        if self.brace_count as i32 + self.parens == 0
                            && !matches!(self.arena[self.prev].kind, "bos" | "slash")
                        {
                            "text"
                        } else {
                            "dot"
                        },
                        value,
                        r"\.",
                    );
                }
                '?' => {
                    let group = self.arena[self.prev].value == "(";
                    if !group && self.peek(0) == '(' && self.peek(1) != '?' {
                        self.extglob_open('?')?;
                        continue;
                    }
                    if self.arena[self.prev].kind == "paren" {
                        let next = self.peek(0);
                        let invalid_name = next == '<'
                            && !matches!(self.peek(1), '!' | '=')
                            && !self.remaining().contains('>');
                        self.emit(
                            "text",
                            value,
                            if group && !"!=<:".contains(next) || invalid_name {
                                r"\?"
                            } else {
                                "?"
                            },
                        );
                    } else {
                        self.emit("qmark", value, "[^/]");
                    }
                }
                '!' => {
                    if self.peek(0) == '('
                        && (self.peek(1) != '?' || !"!=<:".contains(self.peek(2)))
                    {
                        self.extglob_open('!')?;
                        continue;
                    }
                    if self.index == 1 {
                        let mut count = 1;
                        while self.peek(0) == '!' && (self.peek(1) != '(' || self.peek(2) == '?') {
                            self.advance();
                            self.start += 1;
                            count += 1;
                        }
                        if count % 2 == 1 {
                            self.negated = true;
                            self.start += 1;
                        }
                        continue;
                    }
                    self.plain(value);
                }
                '+' => {
                    if self.peek(0) == '(' && self.peek(1) != '?' {
                        self.extglob_open('+')?;
                        continue;
                    }
                    if self.arena[self.prev].value == "(" {
                        self.emit("plus", value, r"\+");
                    } else if matches!(self.arena[self.prev].kind, "bracket" | "paren" | "brace")
                        || self.parens > 0
                    {
                        self.push(Token::new("plus", value, None));
                    } else {
                        self.push(Token::new("plus", r"\+", None));
                    }
                }
                '@' => {
                    if self.peek(0) == '(' && self.peek(1) != '?' {
                        let mut token = Token::new("at", value, Some(String::new()));
                        token.extglob = true;
                        self.push(token);
                    } else {
                        self.push(Token::new("text", value, None));
                    }
                }
                '*' => self.star()?,
                _ => {
                    if matches!(ch, '$' | '^') {
                        value.insert(0, '\\');
                    }
                    self.plain(value);
                }
            }
        }
        for (count, ch) in [
            (self.brackets, '['),
            (self.parens.max(0) as usize, '('),
            (self.brace_count, '{'),
        ] {
            for _ in 0..count {
                if let Some(index) = self.output.rfind(ch)
                    && (index == 0 || self.output.as_bytes()[index - 1] != b'\\')
                {
                    self.output.insert(index, '\\');
                }
            }
        }
        if matches!(self.arena[self.prev].kind, "star" | "bracket") {
            self.emit("maybe_slash", "", r"\/?");
        }
        if self.backtrack {
            self.output = self
                .order
                .iter()
                .map(|&index| self.arena[index].rendered())
                .collect();
        }
        Ok(())
    }

    fn plain(&mut self, mut value: String) {
        while self.index < self.input.len() && !r"@![].,$*+?^{}()|\/".contains(self.peek(0)) {
            value.push(self.advance());
        }
        self.push(Token::new("text", value, None));
    }

    fn star(&mut self) -> Result<(), ConfigError> {
        if self.arena[self.prev].kind == "globstar" || self.arena[self.prev].star {
            let prev = &mut self.arena[self.prev];
            prev.kind = "star";
            prev.star = true;
            prev.value.push('*');
            prev.output = Some(STAR.into());
            self.backtrack = true;
            return Ok(());
        }
        if self.peek(0) == '(' && self.peek(1) != '?' && self.peek(1) != '\0' {
            return self.extglob_open('*');
        }
        if self.arena[self.prev].kind == "star" {
            let prior = self.arena[self.prev].prev;
            let before = self.arena[prior].prev;
            let start = matches!(self.arena[prior].kind, "slash" | "bos");
            let after_star = matches!(self.arena[before].kind, "star" | "globstar");
            let brace = self.brace_count > 0 && matches!(self.arena[prior].kind, "comma" | "brace");
            let extglob =
                !self.extglobs.is_empty() && matches!(self.arena[prior].kind, "pipe" | "paren");
            if !start && self.arena[prior].kind != "paren" && !brace && !extglob {
                self.emit("star", "*", "");
                return Ok(());
            }
            while self.input[self.index..].starts_with(&['/', '*', '*'])
                && matches!(self.peek(3), '\0' | '/')
            {
                self.index += 3;
            }
            if self.arena[prior].kind == "bos" && self.index == self.input.len() {
                let prev = &mut self.arena[self.prev];
                prev.kind = "globstar";
                prev.value.push('*');
                prev.output = Some(GLOBSTAR.into());
                self.output = GLOBSTAR.into();
                return Ok(());
            }
            if self.arena[prior].kind == "slash"
                && self.arena[before].kind != "bos"
                && !after_star
                && self.index == self.input.len()
            {
                let length =
                    self.arena[prior].rendered().len() + self.arena[self.prev].rendered().len();
                self.output
                    .truncate(self.output.len().saturating_sub(length));
                self.arena[prior].output = Some(format!("(?:{}", self.arena[prior].rendered()));
                self.arena[self.prev].kind = "globstar";
                self.arena[self.prev].output = Some(format!("{GLOBSTAR}|$)"));
                self.arena[self.prev].value.push('*');
                self.output.push_str(self.arena[prior].rendered());
                self.output.push_str(self.arena[self.prev].rendered());
                return Ok(());
            }
            if self.arena[prior].kind == "slash"
                && self.arena[before].kind != "bos"
                && self.peek(0) == '/'
            {
                let end = if self.peek(1) != '\0' { "|$" } else { "" };
                let length =
                    self.arena[prior].rendered().len() + self.arena[self.prev].rendered().len();
                self.output
                    .truncate(self.output.len().saturating_sub(length));
                self.arena[prior].output = Some(format!("(?:{}", self.arena[prior].rendered()));
                self.arena[self.prev].kind = "globstar";
                self.arena[self.prev].output = Some(format!("{GLOBSTAR}\\/|\\/{end})"));
                self.arena[self.prev].value.push('*');
                self.output.push_str(self.arena[prior].rendered());
                self.output.push_str(self.arena[self.prev].rendered());
                self.advance();
                self.emit("slash", "/", "");
                return Ok(());
            }
            if self.arena[prior].kind == "bos" && self.peek(0) == '/' {
                self.arena[self.prev].kind = "globstar";
                self.arena[self.prev].value.push('*');
                self.arena[self.prev].output = Some(format!("(?:^|\\/|{GLOBSTAR}\\/)"));
                self.output = self.arena[self.prev].rendered().into();
                self.advance();
                self.emit("slash", "/", "");
                return Ok(());
            }
            let length = self.arena[self.prev].rendered().len();
            self.output
                .truncate(self.output.len().saturating_sub(length));
            self.arena[self.prev].kind = "globstar";
            self.arena[self.prev].output = Some(GLOBSTAR.into());
            self.arena[self.prev].value.push('*');
            self.output.push_str(GLOBSTAR);
            return Ok(());
        }
        if self.index - 1 == self.start || matches!(self.arena[self.prev].kind, "slash" | "dot") {
            let guard = if self.arena[self.prev].kind == "dot" {
                NO_DOT_SLASH
            } else {
                NO_DOTS
            };
            self.output.push_str(guard);
            self.arena[self.prev]
                .output
                .get_or_insert_default()
                .push_str(guard);
            if self.peek(0) != '*' {
                self.output.push_str(ONE);
                self.arena[self.prev]
                    .output
                    .get_or_insert_default()
                    .push_str(ONE);
            }
        }
        self.emit("star", "*", STAR);
        Ok(())
    }
}

fn posix_class(name: &str) -> Option<&'static str> {
    Some(match name {
        "alnum" => "a-zA-Z0-9",
        "alpha" => "a-zA-Z",
        "ascii" => r"\x00-\x7F",
        "blank" => " \t",
        "cntrl" => r"\x00-\x1F\x7F",
        "digit" => "0-9",
        "graph" => r"\x21-\x7E",
        "lower" => "a-z",
        "print" => r"\x20-\x7E ",
        "punct" => r##"\-!"#$%&'()\*+,./:;<=>?@[\]^_`{|}~"##,
        "space" => " \t\r\n\x0b\x0c",
        "upper" => "A-Z",
        "word" => "A-Za-z0-9_",
        "xdigit" => "A-Fa-f0-9",
        _ => return None,
    })
}
