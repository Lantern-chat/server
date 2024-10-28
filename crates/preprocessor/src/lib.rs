#![allow(clippy::single_char_add_str)]

use std::{
    collections::{HashMap, HashSet},
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    #[error("{0}")]
    IoError(#[from] io::Error),

    #[error("Unexpected Eof")]
    UnexpectedEof,
    #[error("Unexpected Endif")]
    UnexpectedEndif,
    #[error("Unexpected Else")]
    UnexpectedElse,

    #[error("Max Substitution Depth Reached")]
    MaxSubstitutionDepthReached,

    #[error("Included file \"{0}\" not found")]
    IncludeNotFound(String),

    #[error("Undefined Symbol: \"{0}\"")]
    UndefinedSymbol(String),

    #[error("Invalid Pragma: \"{0}\"")]
    InvalidPragma(String),

    #[error("Only defines are allowed in this context")]
    OnlyDefinesAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Active,
    Inactive,
}

pub type Defines = HashMap<String, String>;

pub struct Preprocessor {
    pub defines: Defines,
    include: Vec<PathBuf>,
    out: String,
    stack: Vec<Item>,
    pub max_substitution_depth: usize,
    single_line_comment: &'static str,
    onces: HashSet<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context<'a> {
    file: &'a Path,
    line: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("Error at {file}:{line}: {kind}")]
pub struct Error {
    kind: ErrorKind,
    file: PathBuf,
    line: usize,
}

impl Context<'_> {
    pub fn new(file: &Path) -> Context {
        Context { file, line: 0 }
    }

    pub fn error(&self, kind: ErrorKind) -> Error {
        Error::new(*self, kind)
    }
}

impl Error {
    pub fn new(ctx: Context, kind: ErrorKind) -> Self {
        Error {
            kind,
            file: ctx.file.to_owned(),
            line: ctx.line,
        }
    }
}

impl Preprocessor {
    pub fn new(include: Vec<PathBuf>) -> Self {
        Preprocessor {
            max_substitution_depth: 32,
            single_line_comment: "//",
            include,

            defines: HashMap::new(),
            out: String::new(),
            stack: Vec::new(),
            onces: HashSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.defines.clear();
        self.out.clear();
        self.stack.clear();
        self.onces.clear();
    }

    pub fn single_line_comment(&mut self, s: &'static str) -> &mut Self {
        self.single_line_comment = s;
        self
    }

    pub fn define(&mut self, key: String, value: String) -> &mut Self {
        self.defines.insert(key, value);
        self
    }

    pub fn remove_define(&mut self, key: &str) -> &mut Self {
        self.defines.remove(key);
        self
    }

    pub fn process_file(&mut self, path: impl AsRef<Path>) -> Result<String, Error> {
        let mut ctx = Context::new(path.as_ref());

        let src = match std::fs::read_to_string(ctx.file) {
            Ok(src) => src,
            Err(e) => return Err(ctx.error(ErrorKind::IoError(e))),
        };

        let cwd = ctx.file.parent().unwrap_or("./".as_ref());

        self.process_inner(&mut ctx, cwd, &src)?;

        if !self.stack.is_empty() {
            return Err(ctx.error(ErrorKind::UnexpectedEof));
        }

        Ok(std::mem::take(&mut self.out))
    }

    pub fn process(&mut self, mut ctx: Context, src: &str) -> Result<String, Error> {
        self.process_inner(&mut ctx, "./".as_ref(), src)?;

        if !self.stack.is_empty() {
            return Err(ctx.error(ErrorKind::UnexpectedEof));
        }

        Ok(std::mem::take(&mut self.out))
    }

    fn active(&self) -> bool {
        self.stack.last() != Some(&Item::Inactive)
    }

    fn process_inner(&mut self, ctx: &mut Context, cwd: &Path, src: &str) -> Result<(), Error> {
        let mut define_only = false;

        for line in src.lines() {
            ctx.line += 1;

            // skip comments
            let Some(mut t) = line.trim_start().split(self.single_line_comment).next() else {
                continue; // split() will always succeed once
            };

            if !t.starts_with('#') {
                if self.active() {
                    // only check for defines if we're active, since define-only is only activated when active as well
                    if define_only && !t.trim().is_empty() {
                        return Err(ctx.error(ErrorKind::OnlyDefinesAllowed));
                    }

                    Self::substitute_append(
                        ctx,
                        0,
                        self.max_substitution_depth,
                        &self.defines,
                        &mut self.out,
                        line,
                    )?;

                    self.out.push_str("\n");
                }

                continue;
            }

            // skip # and whitespace
            t = t[1..].trim_start();

            // split #cmd and arguments
            let (cmd, mut args) = t.split_once(char::is_whitespace).unwrap_or((t, ""));

            // trim any single-line comments from args
            if let Some(args_uncommented) = args.split(self.single_line_comment).next() {
                args = args_uncommented.trim();
            }

            // split on first whitespace, or if no whitespace pass whole token
            match (cmd, args) {
                ("include", path) if self.active() => self.process_include(ctx, cwd, path)?,
                ("define", define) if self.active() => self.process_define(define.trim()),
                ("pragma", pragma) if self.active() => match pragma {
                    "define-only" => define_only = true,
                    "once" => {
                        if self.onces.contains(ctx.file) {
                            return Ok(()); // skip this file
                        }

                        self.onces.insert(ctx.file.to_owned());
                    }
                    _ => return Err(ctx.error(ErrorKind::InvalidPragma(pragma.to_owned()))),
                },
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
                    if self.active() && self.eval_if(ctx, cond)? {
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
                        None => return Err(ctx.error(ErrorKind::UnexpectedElse)),
                    });
                }
                ("endif", _) => match self.stack.pop() {
                    Some(_) => {}
                    None => return Err(ctx.error(ErrorKind::UnexpectedEndif)),
                },
                _ => {}
            }
        }

        Ok(())
    }

    fn process_include(&mut self, ctx: &Context, cwd: &Path, path: &str) -> Result<(), Error> {
        let path = path.trim_matches(|c: char| c.is_whitespace() || matches!(c, '<' | '"'));

        let include_paths = self.include.iter().map(|p| p.as_path()).chain(std::iter::once(cwd));

        for base in include_paths {
            let full_path = match base.join(path).canonicalize() {
                Ok(p) => p,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(ctx.error(ErrorKind::IoError(e))),
            };

            let src = match std::fs::read_to_string(&full_path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(ctx.error(ErrorKind::IoError(e))),
                Ok(file) => file,
            };

            return self.process_inner(
                &mut Context::new(&full_path),
                full_path.parent().unwrap_or("./".as_ref()),
                &src,
            );
        }

        Err(ctx.error(ErrorKind::IncludeNotFound(path.to_owned())))
    }

    fn process_define(&mut self, define: &str) {
        let (name, value) = match define.split_once(char::is_whitespace) {
            None => (define, String::new()),
            Some((name, value)) => (name, value.trim_start().to_owned()),
        };

        self.defines.insert(name.to_owned(), value);
    }

    fn substitute_append(
        ctx: &Context,
        depth: usize,
        max_depth: usize,
        defines: &Defines,
        out: &mut String,
        line: &str,
    ) -> Result<(), Error> {
        if depth > max_depth {
            return Err(ctx.error(ErrorKind::MaxSubstitutionDepthReached));
        }

        use unicode_segmentation::UnicodeSegmentation;

        for word in line.split_word_bounds() {
            if word.starts_with(|c: char| c.is_alphabetic() || c == '_') {
                if let Some(replacement) = defines.get(word) {
                    // recurse to replace any replacements
                    Self::substitute_append(ctx, depth + 1, max_depth, defines, out, replacement)?;
                    continue;
                }
            }

            *out += word;
        }

        Ok(())
    }

    fn eval_if(&mut self, ctx: &Context, cond: &str) -> Result<bool, Error> {
        let mut resolved_cond = String::with_capacity(cond.len());

        Self::substitute_append(
            ctx,
            0,
            self.max_substitution_depth,
            &self.defines,
            &mut resolved_cond,
            cond,
        )?;

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
                    Err(_) => {
                        return Err(ctx.error(ErrorKind::UndefinedSymbol(word.to_owned())));
                    }
                },
                _ if !word.chars().all(char::is_whitespace) => {
                    return Err(ctx.error(ErrorKind::UndefinedSymbol(word.to_owned())));
                }
                _ => Symbol::Whitespace,
            });
        }

        unimplemented!()
    }
}
