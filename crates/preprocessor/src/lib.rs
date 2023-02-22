use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    IoError(#[from] io::Error),

    #[error("Unexpected Eof")]
    UnexpectedEof,
    #[error("Unexpected Endif on line {0}")]
    UnexpectedEndif(usize),
    #[error("Unexpected Else on line {0}")]
    UnexpectedElse(usize),

    #[error("Max Substitution Depth Reached")]
    MaxSubstitutionDepthReached,

    #[error("Included file \"{0}\" not found")]
    IncludeNotFound(String),

    #[error("Undefined Symbol: \"{0}\"")]
    UndefinedSymbol(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Active,
    Inactive,
}

pub type Defines = HashMap<String, String>;

pub struct Context {
    pub defines: Defines,
    include: Vec<PathBuf>,
    out: String,
    stack: Vec<Item>,
    pub max_substitution_depth: usize,
    single_line_comment: &'static str,
}

impl Context {
    pub fn new(include: Vec<PathBuf>) -> Self {
        Context {
            defines: HashMap::new(),
            include,
            out: String::new(),
            stack: Vec::new(),
            max_substitution_depth: 32,
            single_line_comment: "//",
        }
    }

    pub fn single_line_comment(&mut self, s: &'static str) -> &mut Self {
        self.single_line_comment = s;
        self
    }

    pub fn define(&mut self, key: String, value: String) -> &mut Self {
        self.defines.insert(key, value);
        self
    }

    pub fn process_file(&mut self, path: impl AsRef<Path>) -> Result<String, Error> {
        self.process(&std::fs::read_to_string(path)?)
    }

    pub fn process(&mut self, src: &str) -> Result<String, Error> {
        self.process_inner("./".as_ref(), src)?;

        if !self.stack.is_empty() {
            return Err(Error::UnexpectedEof);
        }

        Ok(std::mem::take(&mut self.out))
    }

    fn active(&self) -> bool {
        self.stack.last() != Some(&Item::Inactive)
    }

    fn process_inner(&mut self, cwd: &Path, src: &str) -> Result<(), Error> {
        let mut ln = 0;

        for line in src.lines() {
            ln += 1;

            let Some(mut t) = line.trim_start().split(self.single_line_comment).next() else {
                continue; // split() will always succeed once
            };

            if !t.starts_with('#') {
                if self.active() {
                    Self::substitute_append(0, self.max_substitution_depth, &self.defines, &mut self.out, line)?;
                    self.out.push_str("\n");
                }

                continue;
            }

            // skip # and whitespace
            t = t[1..].trim_start();

            // split on first whitespace, or if no whitespace pass whole token
            match t.split_once(char::is_whitespace).unwrap_or((t, "")) {
                ("include", path) if self.active() => self.process_include(cwd, path)?,
                ("define", define) if self.active() => self.process_define(define.trim()),
                ("undef", define) if self.active() => {
                    self.defines.remove(define.trim());
                }
                (which @ ("ifndef" | "ifdef"), define) => {
                    if self.active() {
                        let flip = which == "ifndef";
                        self.stack.push(match flip ^ self.defines.contains_key(define.trim()) {
                            true => Item::Active,
                            false => Item::Inactive,
                        });
                    } else {
                        self.stack.push(Item::Inactive);
                    }
                }
                ("if", cond) => {
                    if self.active() && self.eval_if(cond)? {
                        self.stack.push(Item::Active);
                    } else {
                        self.stack.push(Item::Inactive);
                    }
                }
                ("else", _) => {
                    let last = self.stack.pop();

                    self.stack.push(match last {
                        Some(Item::Active) => Item::Inactive,
                        Some(Item::Inactive) => Item::Active,
                        None => return Err(Error::UnexpectedElse(ln)),
                    });
                }
                ("endif", _) => match self.stack.pop() {
                    Some(_) => {}
                    None => return Err(Error::UnexpectedEndif(ln)),
                },
                _ => {}
            }
        }

        Ok(())
    }

    fn process_include(&mut self, cwd: &Path, path: &str) -> Result<(), Error> {
        let path = path.trim_matches(|c: char| c.is_whitespace() || matches!(c, '<' | '"'));

        let include_paths = self.include.iter().map(|p| p.as_path()).chain(std::iter::once(cwd));

        for base in include_paths {
            let full_path = base.join(path);

            let src = match std::fs::read_to_string(&full_path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
                Ok(file) => file,
            };

            return self.process_inner(full_path.parent().unwrap_or("./".as_ref()), &src);
        }

        Err(Error::IncludeNotFound(path.to_owned()))
    }

    fn process_define(&mut self, define: &str) {
        let (name, value) = match define.split_once(char::is_whitespace) {
            None => (define, String::new()),
            Some((name, value)) => (name, value.trim_start().to_owned()),
        };

        self.defines.insert(name.to_owned(), value);
    }

    fn substitute_append(
        depth: usize,
        max_depth: usize,
        defines: &Defines,
        out: &mut String,
        line: &str,
    ) -> Result<(), Error> {
        if depth > max_depth {
            return Err(Error::MaxSubstitutionDepthReached);
        }

        use unicode_segmentation::UnicodeSegmentation;

        for word in line.split_word_bounds() {
            if word.starts_with(char::is_alphabetic) {
                if let Some(replacement) = defines.get(word) {
                    // recurse to replace any replacements
                    Self::substitute_append(depth + 1, max_depth, defines, out, &replacement)?;
                    continue;
                }
            }

            *out += word;
        }

        Ok(())
    }

    fn eval_if(&mut self, cond: &str) -> Result<bool, Error> {
        let mut resolved_cond = String::with_capacity(cond.len());

        Self::substitute_append(0, self.max_substitution_depth, &self.defines, &mut resolved_cond, cond)?;

        use unicode_segmentation::UnicodeSegmentation;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum Symbol {
            OpenParen,
            CloseParen,
            Or,
            And,
            Less,
            Greater,
            Equal,
            Not,
            Subtract,
            Add,
            Divide,
            Multiply,
            Remainder,
            Number(i64),
            Whitespace,
        }

        let mut tokens = Vec::new();

        for word in resolved_cond.split_word_bounds() {
            tokens.push(match word {
                "(" => Symbol::OpenParen,
                ")" => Symbol::CloseParen,
                "<" => Symbol::Less,
                ">" => Symbol::Greater,
                "=" => Symbol::Equal,
                "!" => Symbol::Not,
                "-" => Symbol::Subtract,
                "+" => Symbol::Add,
                "*" => Symbol::Multiply,
                "/" => Symbol::Divide,
                "%" => Symbol::Remainder,
                "|" => Symbol::Or,
                "&" => Symbol::And,
                _ if word.starts_with(char::is_numeric) => match word.parse() {
                    Ok(num) => Symbol::Number(num),
                    Err(_) => return Err(Error::UndefinedSymbol(word.to_owned())),
                },
                _ if !word.chars().all(char::is_whitespace) => {
                    return Err(Error::UndefinedSymbol(word.to_owned()));
                }
                _ => Symbol::Whitespace,
            });
        }

        unimplemented!()
    }
}
