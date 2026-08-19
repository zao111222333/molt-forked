//! Lossless Tcl 8.6 syntax analysis shared by Molt runtimes and editors.
//!
//! All source locations are UTF-8 byte offsets. Tokens are sorted, do not overlap, and
//! collectively cover the complete input, which lets terminal and browser renderers consume
//! them without reparsing or repairing gaps.

#![forbid(unsafe_code)]

use std::{cmp, ops::Range};

/// A half-open UTF-8 byte range in the analyzed source.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct TextRange {
    start: usize,
    end: usize,
}

impl TextRange {
    /// Creates a range. Both endpoints must be UTF-8 boundaries in the associated source.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the inclusive starting byte offset.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Returns the exclusive ending byte offset.
    #[must_use]
    pub const fn end(self) -> usize {
        self.end
    }

    /// Returns the number of bytes in the range.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end - self.start
    }

    /// Returns whether the range is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

impl From<Range<usize>> for TextRange {
    fn from(range: Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }
}

impl From<TextRange> for Range<usize> {
    fn from(range: TextRange) -> Self {
        range.start..range.end
    }
}

/// A leaf syntax category suitable for terminal or browser highlighting.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum SyntaxKind {
    /// Text without a more specific Tcl role.
    Plain,
    /// Horizontal or vertical whitespace.
    Whitespace,
    /// A Tcl command comment.
    Comment,
    /// The literal portion of the first word in a command.
    Command,
    /// A bare word or literal segment.
    Word,
    /// Text protected by braces or quotes.
    String,
    /// A variable sigil, name, or array index.
    Variable,
    /// A backslash escape, including folded newline indentation.
    Escape,
    /// A quote, brace, bracket, parenthesis, or expansion delimiter.
    Delimiter,
    /// A command separator (`;` or a command-terminating newline).
    Separator,
    /// A numeric literal in an expression.
    Number,
    /// An expression operator.
    Operator,
    /// A mathematical function name in an expression.
    Function,
    /// Text associated with a recoverable syntax error.
    Invalid,
}

/// A highlighted source token.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SyntaxToken {
    kind: SyntaxKind,
    range: TextRange,
    depth: u16,
}

impl SyntaxToken {
    /// Returns the token category.
    #[must_use]
    pub const fn kind(self) -> SyntaxKind {
        self.kind
    }

    /// Returns the source range.
    #[must_use]
    pub const fn range(self) -> TextRange {
        self.range
    }

    /// Returns the nesting depth for rainbow delimiters.
    #[must_use]
    pub const fn depth(self) -> u16 {
        self.depth
    }
}

/// A delimiter that can make an otherwise valid prefix incomplete.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum IncompleteKind {
    /// A closing brace is required.
    Brace,
    /// A closing quote is required.
    Quote,
    /// A closing bracket is required.
    Bracket,
    /// A closing parenthesis is required for an array variable reference.
    VariableIndex,
}

/// Whether a script can be submitted to the interpreter.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ParseStatus {
    /// The script is syntactically complete.
    Complete,
    /// The script is a valid prefix but requires more input.
    Incomplete {
        /// The kind of closing delimiter that is required.
        kind: IncompleteKind,
        /// The byte offset of the unmatched opening delimiter.
        opened_at: usize,
    },
    /// The input is complete but contains a syntax error.
    Invalid,
}

impl ParseStatus {
    /// Returns true only for a prefix that should continue on another REPL line.
    #[must_use]
    pub const fn is_incomplete(self) -> bool {
        matches!(self, Self::Incomplete { .. })
    }
}

/// The class of a syntax diagnostic.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// Complete input whose token arrangement is not legal Tcl syntax.
    InvalidSyntax,
}

/// A syntax diagnostic with a precise source location.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SyntaxDiagnostic {
    kind: DiagnosticKind,
    range: TextRange,
    message: String,
}

impl SyntaxDiagnostic {
    /// Returns the diagnostic class.
    #[must_use]
    pub const fn kind(&self) -> DiagnosticKind {
        self.kind
    }

    /// Returns the source range associated with the diagnostic.
    #[must_use]
    pub const fn range(&self) -> TextRange {
        self.range
    }

    /// Returns the human-readable diagnostic.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Context rules used for command-aware highlighting.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CommandRule {
    Lexical,
    Apply,
    ExprAll,
    ScriptAll,
    Fixed(&'static [(usize, NestedLanguage)]),
    If,
    Foreach,
    Namespace,
    Subst,
    Switch,
    Try,
    Uplevel,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum NestedLanguage {
    Script,
    Expr,
}

#[derive(Debug, Clone, Copy)]
enum ProfileKind {
    Lexical,
    Tcl86,
}

/// An immutable command-context profile.
#[derive(Debug, Clone, Copy)]
pub struct SyntaxProfile {
    kind: ProfileKind,
}

impl SyntaxProfile {
    /// Creates a profile that performs Tcl lexical highlighting without command-specific roles.
    #[must_use]
    pub const fn lexical() -> Self {
        Self { kind: ProfileKind::Lexical }
    }

    fn rule(self, name: &str) -> CommandRule {
        match self.kind {
            ProfileKind::Lexical => CommandRule::Lexical,
            ProfileKind::Tcl86 => tcl_86_rule(name),
        }
    }
}

const FOR_ARGS: &[(usize, NestedLanguage)] = &[
    (1, NestedLanguage::Script),
    (2, NestedLanguage::Expr),
    (3, NestedLanguage::Script),
    (4, NestedLanguage::Script),
];
const WHILE_ARGS: &[(usize, NestedLanguage)] =
    &[(1, NestedLanguage::Expr), (2, NestedLanguage::Script)];
const PROC_ARGS: &[(usize, NestedLanguage)] = &[(3, NestedLanguage::Script)];
const CATCH_ARGS: &[(usize, NestedLanguage)] = &[(1, NestedLanguage::Script)];
const TIME_ARGS: &[(usize, NestedLanguage)] = &[(1, NestedLanguage::Script)];

fn tcl_86_rule(name: &str) -> CommandRule {
    match name {
        "apply" => CommandRule::Apply,
        "catch" => CommandRule::Fixed(CATCH_ARGS),
        "eval" => CommandRule::ScriptAll,
        "expr" => CommandRule::ExprAll,
        "for" => CommandRule::Fixed(FOR_ARGS),
        "foreach" | "lmap" => CommandRule::Foreach,
        "if" => CommandRule::If,
        "namespace" => CommandRule::Namespace,
        "proc" => CommandRule::Fixed(PROC_ARGS),
        "subst" => CommandRule::Subst,
        "switch" => CommandRule::Switch,
        "time" => CommandRule::Fixed(TIME_ARGS),
        "try" => CommandRule::Try,
        "uplevel" => CommandRule::Uplevel,
        "while" => CommandRule::Fixed(WHILE_ARGS),
        _ => CommandRule::Lexical,
    }
}

/// Tcl 8.6 built-in command context used by Molt's standard editors.
pub const TCL_86_PROFILE: SyntaxProfile = SyntaxProfile { kind: ProfileKind::Tcl86 };

/// A complete, lossless syntax analysis.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SyntaxAnalysis {
    status: ParseStatus,
    tokens: Vec<SyntaxToken>,
    diagnostics: Vec<SyntaxDiagnostic>,
}

impl SyntaxAnalysis {
    /// Returns the script submission status.
    #[must_use]
    pub const fn status(&self) -> ParseStatus {
        self.status
    }

    /// Returns sorted, non-overlapping tokens covering the full source.
    #[must_use]
    pub fn tokens(&self) -> &[SyntaxToken] {
        &self.tokens
    }

    /// Returns recoverable syntax diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[SyntaxDiagnostic] {
        &self.diagnostics
    }
}

/// Analyzes a Tcl script without evaluating it.
#[must_use]
pub fn analyze_script(source: &str, profile: &SyntaxProfile) -> SyntaxAnalysis {
    Analyzer::<true>::new(source, *profile).analyze()
}

/// Determines whether a Tcl script is complete without allocating highlighting tokens.
#[must_use]
pub fn script_status(source: &str, profile: &SyntaxProfile) -> ParseStatus {
    Analyzer::<false>::new(source, *profile).status_only()
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct Cell {
    kind: SyntaxKind,
    depth: u16,
    priority: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self { kind: SyntaxKind::Plain, depth: 0, priority: 0 }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum WordForm {
    Bare,
    Braced,
    Quoted,
}

#[derive(Debug, Clone, Copy)]
struct WordInfo {
    range: TextRange,
    inner: TextRange,
    form: WordForm,
    is_literal: bool,
    followed_by_fold: bool,
}

impl WordInfo {
    fn context_range(&self) -> TextRange {
        match self.form {
            WordForm::Braced | WordForm::Quoted => self.inner,
            WordForm::Bare => self.range,
        }
    }

    fn literal_text<'a>(&self, source: &'a str) -> Option<&'a str> {
        self.is_literal.then(|| &source[self.inner.start..self.inner.end])
    }
}

struct Analyzer<'a, const TOKENS: bool> {
    source: &'a str,
    profile: SyntaxProfile,
    cells: Vec<Cell>,
    diagnostics: Vec<SyntaxDiagnostic>,
    invalid: bool,
    incomplete: Option<(IncompleteKind, usize)>,
}

impl<'a, const TOKENS: bool> Analyzer<'a, TOKENS> {
    fn new(source: &'a str, profile: SyntaxProfile) -> Self {
        Self {
            source,
            profile,
            cells: if TOKENS { vec![Cell::default(); source.len()] } else { Vec::new() },
            diagnostics: Vec::new(),
            invalid: false,
            incomplete: None,
        }
    }

    fn parse(&mut self) {
        self.scan_script(0, self.source.len(), None, 0);
    }

    fn parse_status(&self) -> ParseStatus {
        if let Some((kind, opened_at)) = self.incomplete {
            ParseStatus::Incomplete { kind, opened_at }
        } else if self.invalid {
            ParseStatus::Invalid
        } else {
            ParseStatus::Complete
        }
    }

    fn status_only(mut self) -> ParseStatus {
        self.parse();
        self.parse_status()
    }

    fn analyze(mut self) -> SyntaxAnalysis {
        self.parse();
        let status = self.parse_status();

        let tokens = self.finish_tokens();
        SyntaxAnalysis { status, tokens, diagnostics: self.diagnostics }
    }

    fn finish_tokens(&self) -> Vec<SyntaxToken> {
        if self.source.is_empty() {
            return Vec::new();
        }

        let mut tokens = Vec::new();
        let mut start = 0;
        let mut cell = self.cells[0];
        for index in 1..self.source.len() {
            if !self.source.is_char_boundary(index) {
                continue;
            }
            if self.cells[index] != cell {
                tokens.push(SyntaxToken {
                    kind: cell.kind,
                    range: TextRange::new(start, index),
                    depth: cell.depth,
                });
                start = index;
                cell = self.cells[index];
            }
        }
        tokens.push(SyntaxToken {
            kind: cell.kind,
            range: TextRange::new(start, self.source.len()),
            depth: cell.depth,
        });
        tokens
    }

    fn paint(&mut self, range: TextRange, kind: SyntaxKind, depth: u16, priority: u8) {
        if !TOKENS {
            return;
        }
        let end = cmp::min(range.end, self.cells.len());
        for cell in &mut self.cells[cmp::min(range.start, end)..end] {
            if priority >= cell.priority {
                *cell = Cell { kind, depth, priority };
            }
        }
    }

    fn invalidate(&mut self, range: TextRange, message: impl Into<String>, depth: u16) {
        self.invalid = true;
        if !TOKENS {
            return;
        }
        self.paint(range, SyntaxKind::Invalid, depth, 100);
        self.diagnostics.push(SyntaxDiagnostic {
            kind: DiagnosticKind::InvalidSyntax,
            range,
            message: message.into(),
        });
    }

    fn mark_incomplete(&mut self, kind: IncompleteKind, opened_at: usize) {
        if self.incomplete.is_none() {
            self.incomplete = Some((kind, opened_at));
        }
    }

    fn char_at(&self, index: usize) -> Option<char> {
        self.source.get(index..)?.chars().next()
    }

    fn next_index(&self, index: usize) -> usize {
        self.char_at(index).map_or(index, |ch| index + ch.len_utf8())
    }

    fn scan_script(
        &mut self,
        start: usize,
        end: usize,
        terminator: Option<(char, usize)>,
        depth: u16,
    ) -> usize {
        let mut index = start;
        let mut command_start = true;
        let mut words = Vec::new();

        while index < end {
            let Some(ch) = self.char_at(index) else { break };
            if terminator.is_some_and(|(term, _)| ch == term) {
                self.apply_command_context(&words, depth);
                return index;
            }

            if ch == '\\' && self.is_folded_newline(index, end) {
                let next = self.scan_escape(index, end, depth);
                index = next;
                continue;
            }

            if ch == ';' || ch == '\n' {
                let next = self.next_index(index);
                self.paint(TextRange::new(index, next), SyntaxKind::Separator, depth, 3);
                self.apply_command_context(&words, depth);
                words.clear();
                command_start = true;
                index = next;
                continue;
            }

            if ch.is_whitespace() {
                let whitespace_start = index;
                while index < end {
                    let Some(next) = self.char_at(index) else { break };
                    if next == '\n' || !next.is_whitespace() {
                        break;
                    }
                    index = self.next_index(index);
                }
                self.paint(
                    TextRange::new(whitespace_start, index),
                    SyntaxKind::Whitespace,
                    depth,
                    1,
                );
                continue;
            }

            if command_start && ch == '#' {
                let comment_start = index;
                while index < end && self.char_at(index) != Some('\n') {
                    if self.char_at(index) == Some('\\')
                        && self.is_folded_newline(index, end)
                    {
                        index = self.escape_end(index, end);
                    } else {
                        index = self.next_index(index);
                    }
                }
                self.paint(
                    TextRange::new(comment_start, index),
                    SyntaxKind::Comment,
                    depth,
                    20,
                );
                continue;
            }

            command_start = false;
            let (word, next) =
                self.scan_word(index, end, depth, terminator.map(|value| value.0));
            let followed_by_fold = word.followed_by_fold;
            if TOKENS {
                if words.is_empty() {
                    self.paint_command_word(&word, depth);
                }
                words.push(word);
            }
            index = next;

            if index < end {
                let Some(next) = self.char_at(index) else { break };
                let legal = followed_by_fold
                    || next.is_whitespace()
                    || next == ';'
                    || terminator.is_some_and(|(term, _)| next == term);
                if !legal {
                    let bad_end = self.next_index(index);
                    self.invalidate(
                        TextRange::new(index, bad_end),
                        "extra characters after close-quote or close-brace",
                        depth,
                    );
                    index = bad_end;
                }
            }
        }

        self.apply_command_context(&words, depth);
        if let Some((term, opened_at)) = terminator {
            let kind = match term {
                ']' => IncompleteKind::Bracket,
                _ => IncompleteKind::Bracket,
            };
            self.mark_incomplete(kind, opened_at);
        }
        index
    }

    fn scan_word(
        &mut self,
        start: usize,
        end: usize,
        depth: u16,
        script_terminator: Option<char>,
    ) -> (WordInfo, usize) {
        match self.char_at(start) {
            Some('{') => self.scan_braced_word(start, end, depth),
            Some('"') => self.scan_quoted_word(start, end, depth),
            _ => self.scan_bare_word(start, end, depth, script_terminator),
        }
    }

    fn scan_braced_word(
        &mut self,
        start: usize,
        end: usize,
        depth: u16,
    ) -> (WordInfo, usize) {
        let open_end = self.next_index(start);
        self.paint(TextRange::new(start, open_end), SyntaxKind::Delimiter, depth, 30);
        let mut index = open_end;
        let mut braces = 1usize;

        while index < end {
            let ch = self.char_at(index).expect("index must be a character boundary");
            if ch == '\\' {
                index = self.scan_escape(index, end, depth + 1);
                continue;
            }
            if ch == '{' {
                braces += 1;
            } else if ch == '}' {
                braces -= 1;
                if braces == 0 {
                    self.paint(
                        TextRange::new(open_end, index),
                        SyntaxKind::String,
                        depth + 1,
                        2,
                    );
                    let close_end = self.next_index(index);
                    self.paint(
                        TextRange::new(index, close_end),
                        SyntaxKind::Delimiter,
                        depth,
                        30,
                    );
                    return (
                        WordInfo {
                            range: TextRange::new(start, close_end),
                            inner: TextRange::new(open_end, index),
                            form: WordForm::Braced,
                            is_literal: true,
                            followed_by_fold: false,
                        },
                        close_end,
                    );
                }
            }
            index = self.next_index(index);
        }

        self.paint(TextRange::new(open_end, end), SyntaxKind::String, depth + 1, 2);
        self.mark_incomplete(IncompleteKind::Brace, start);
        (
            WordInfo {
                range: TextRange::new(start, end),
                inner: TextRange::new(open_end, end),
                form: WordForm::Braced,
                is_literal: false,
                followed_by_fold: false,
            },
            end,
        )
    }

    fn scan_quoted_word(
        &mut self,
        start: usize,
        end: usize,
        depth: u16,
    ) -> (WordInfo, usize) {
        let open_end = self.next_index(start);
        self.paint(TextRange::new(start, open_end), SyntaxKind::Delimiter, depth, 30);
        let mut index = open_end;
        let mut literal = true;

        while index < end {
            match self.char_at(index).expect("index must be valid") {
                '"' => {
                    self.paint(
                        TextRange::new(open_end, index),
                        SyntaxKind::String,
                        depth + 1,
                        2,
                    );
                    let close_end = self.next_index(index);
                    self.paint(
                        TextRange::new(index, close_end),
                        SyntaxKind::Delimiter,
                        depth,
                        30,
                    );
                    return (
                        WordInfo {
                            range: TextRange::new(start, close_end),
                            inner: TextRange::new(open_end, index),
                            form: WordForm::Quoted,
                            is_literal: literal,
                            followed_by_fold: false,
                        },
                        close_end,
                    );
                }
                '\\' => {
                    literal = false;
                    index = self.scan_escape(index, end, depth + 1);
                }
                '$' => {
                    literal = false;
                    index = self.scan_variable(index, end, depth + 1);
                }
                '[' => {
                    literal = false;
                    index = self.scan_bracket_script(index, end, depth + 1);
                }
                _ => index = self.next_index(index),
            }
        }

        self.paint(TextRange::new(open_end, end), SyntaxKind::String, depth + 1, 2);
        self.mark_incomplete(IncompleteKind::Quote, start);
        (
            WordInfo {
                range: TextRange::new(start, end),
                inner: TextRange::new(open_end, end),
                form: WordForm::Quoted,
                is_literal: false,
                followed_by_fold: false,
            },
            end,
        )
    }

    fn scan_bare_word(
        &mut self,
        start: usize,
        end: usize,
        depth: u16,
        script_terminator: Option<char>,
    ) -> (WordInfo, usize) {
        let mut index = start;
        let mut literal = true;
        let mut followed_by_fold = false;
        while index < end {
            let ch = self.char_at(index).expect("index must be valid");
            if ch.is_whitespace()
                || ch == ';'
                || script_terminator.is_some_and(|term| ch == term)
            {
                break;
            }
            match ch {
                '\\' if self.is_folded_newline(index, end) => {
                    literal = false;
                    index = self.scan_escape(index, end, depth);
                    followed_by_fold = true;
                    break;
                }
                '\\' => {
                    literal = false;
                    index = self.scan_escape(index, end, depth);
                }
                '$' => {
                    literal = false;
                    index = self.scan_variable(index, end, depth);
                }
                '[' => {
                    literal = false;
                    index = self.scan_bracket_script(index, end, depth);
                }
                _ => index = self.next_index(index),
            }
        }
        self.paint(TextRange::new(start, index), SyntaxKind::Word, depth, 2);
        (
            WordInfo {
                range: TextRange::new(start, index),
                inner: TextRange::new(start, index),
                form: WordForm::Bare,
                is_literal: literal,
                followed_by_fold,
            },
            index,
        )
    }

    fn paint_command_word(&mut self, word: &WordInfo, depth: u16) {
        if word.is_literal {
            self.paint(word.inner, SyntaxKind::Command, depth, 4);
        }
    }

    fn scan_escape(&mut self, start: usize, end: usize, depth: u16) -> usize {
        let index = self.escape_end(start, end);
        self.paint(TextRange::new(start, index), SyntaxKind::Escape, depth, 40);
        index
    }

    fn escape_end(&self, start: usize, end: usize) -> usize {
        let mut index = self.next_index(start);
        if index < end {
            let ch = self.char_at(index).expect("index must be valid");
            index = self.next_index(index);
            if ch == '\n' {
                while index < end && matches!(self.char_at(index), Some(' ' | '\t')) {
                    index = self.next_index(index);
                }
            } else if ch == '\r' && self.char_at(index) == Some('\n') {
                index = self.next_index(index);
                while index < end && matches!(self.char_at(index), Some(' ' | '\t')) {
                    index = self.next_index(index);
                }
            } else if matches!(ch, '0'..='7') {
                let mut count = 1;
                while count < 3 && matches!(self.char_at(index), Some('0'..='7')) {
                    index = self.next_index(index);
                    count += 1;
                }
            } else if matches!(ch, 'x' | 'u' | 'U') {
                let maximum = match ch {
                    'x' => 2,
                    'u' => 4,
                    'U' => 8,
                    _ => unreachable!(),
                };
                let mut count = 0;
                while count < maximum
                    && self.char_at(index).is_some_and(|c| c.is_ascii_hexdigit())
                {
                    index = self.next_index(index);
                    count += 1;
                }
            }
        }
        index
    }

    fn is_folded_newline(&self, start: usize, end: usize) -> bool {
        let after_slash = self.next_index(start);
        if after_slash >= end {
            return false;
        }
        match self.char_at(after_slash) {
            Some('\n') => true,
            Some('\r') => {
                let after_cr = self.next_index(after_slash);
                after_cr < end && self.char_at(after_cr) == Some('\n')
            }
            _ => false,
        }
    }

    fn scan_variable(&mut self, start: usize, end: usize, depth: u16) -> usize {
        let mut index = self.next_index(start);
        self.paint(TextRange::new(start, index), SyntaxKind::Variable, depth, 50);
        if index >= end {
            return index;
        }

        if self.char_at(index) == Some('{') {
            let open = index;
            index = self.next_index(index);
            while index < end && self.char_at(index) != Some('}') {
                index = self.next_index(index);
            }
            if index < end {
                index = self.next_index(index);
            } else {
                self.mark_incomplete(IncompleteKind::Brace, open);
            }
            self.paint(TextRange::new(open, index), SyntaxKind::Variable, depth, 50);
            return index;
        }

        let name_start = index;
        while index < end {
            if self.source[index..end].starts_with("::") {
                index += 2;
            } else if self.char_at(index).is_some_and(is_var_name_char) {
                index = self.next_index(index);
            } else {
                break;
            }
        }
        self.paint(TextRange::new(name_start, index), SyntaxKind::Variable, depth, 50);

        if index < end && self.char_at(index) == Some('(') && index > name_start {
            let open = index;
            let open_end = self.next_index(index);
            self.paint(TextRange::new(open, open_end), SyntaxKind::Delimiter, depth, 50);
            index = open_end;
            while index < end {
                match self.char_at(index).expect("index must be valid") {
                    ')' => {
                        let close_end = self.next_index(index);
                        self.paint(
                            TextRange::new(index, close_end),
                            SyntaxKind::Delimiter,
                            depth,
                            50,
                        );
                        return close_end;
                    }
                    '\\' => index = self.scan_escape(index, end, depth + 1),
                    '$' => index = self.scan_variable(index, end, depth + 1),
                    '[' => index = self.scan_bracket_script(index, end, depth + 1),
                    _ => {
                        let next = self.next_index(index);
                        self.paint(
                            TextRange::new(index, next),
                            SyntaxKind::Variable,
                            depth + 1,
                            30,
                        );
                        index = next;
                    }
                }
            }
            self.mark_incomplete(IncompleteKind::VariableIndex, open);
        }
        index
    }

    fn scan_bracket_script(&mut self, start: usize, end: usize, depth: u16) -> usize {
        let open_end = self.next_index(start);
        self.paint(TextRange::new(start, open_end), SyntaxKind::Delimiter, depth, 60);
        let close = self.scan_script(open_end, end, Some((']', start)), depth + 1);
        if close < end && self.char_at(close) == Some(']') {
            let close_end = self.next_index(close);
            self.paint(
                TextRange::new(close, close_end),
                SyntaxKind::Delimiter,
                depth,
                60,
            );
            close_end
        } else {
            end
        }
    }

    fn apply_command_context(&mut self, words: &[WordInfo], depth: u16) {
        if !TOKENS {
            return;
        }
        let Some(command) = words.first().and_then(|word| word.literal_text(self.source))
        else {
            return;
        };
        // Command-aware scans enrich highlighting inside literal argument bodies, but those
        // bodies remain ordinary Tcl words until the command executes. Their internal parser
        // state must not change whether the enclosing script can be submitted.
        let outer_invalid = self.invalid;
        let outer_incomplete = self.incomplete;
        match self.profile.rule(command) {
            CommandRule::Lexical => {}
            CommandRule::Apply => {
                if let Some(lambda) = words.get(1) {
                    let lambda = self.scan_list_words(lambda.context_range(), depth + 1);
                    if let Some(body) = lambda.get(1) {
                        self.scan_nested_word(body, NestedLanguage::Script, depth + 2);
                    }
                }
            }
            CommandRule::ExprAll => {
                for word in words.iter().skip(1) {
                    self.scan_nested_word(word, NestedLanguage::Expr, depth + 1);
                }
            }
            CommandRule::ScriptAll => {
                for word in words.iter().skip(1) {
                    self.scan_nested_word(word, NestedLanguage::Script, depth + 1);
                }
            }
            CommandRule::Fixed(arguments) => {
                for &(index, language) in arguments {
                    if let Some(word) = words.get(index) {
                        self.scan_nested_word(word, language, depth + 1);
                    }
                }
            }
            CommandRule::Foreach => {
                if let Some(word) = words.last().filter(|_| words.len() > 1) {
                    self.scan_nested_word(word, NestedLanguage::Script, depth + 1);
                }
            }
            CommandRule::If => self.scan_if_context(words, depth + 1),
            CommandRule::Namespace => {
                if words.get(1).and_then(|word| word.literal_text(self.source))
                    == Some("eval")
                {
                    for word in words.iter().skip(3) {
                        self.scan_nested_word(word, NestedLanguage::Script, depth + 1);
                    }
                }
            }
            CommandRule::Subst => self.scan_subst_context(words, depth + 1),
            CommandRule::Switch => self.scan_switch_context(words, depth + 1),
            CommandRule::Try => self.scan_try_context(words, depth + 1),
            CommandRule::Uplevel => {
                let first = usize::from(
                    words
                        .get(1)
                        .and_then(|word| word.literal_text(self.source))
                        .is_some_and(looks_like_scope_level),
                ) + 1;
                for word in words.iter().skip(first) {
                    self.scan_nested_word(word, NestedLanguage::Script, depth + 1);
                }
            }
        }
        self.invalid = outer_invalid;
        self.incomplete = outer_incomplete;
    }

    fn scan_if_context(&mut self, words: &[WordInfo], depth: u16) {
        let mut index = 1;
        let mut expect_expr = true;
        while index < words.len() {
            let literal = words[index].literal_text(self.source);
            if matches!(literal, Some("then" | "elseif" | "else")) {
                expect_expr = literal == Some("elseif");
                index += 1;
                continue;
            }
            let language = if expect_expr {
                expect_expr = false;
                NestedLanguage::Expr
            } else {
                NestedLanguage::Script
            };
            self.scan_nested_word(&words[index], language, depth);
            index += 1;
        }
    }

    fn scan_try_context(&mut self, words: &[WordInfo], depth: u16) {
        if let Some(body) = words.get(1) {
            self.scan_nested_word(body, NestedLanguage::Script, depth);
        }
        let mut index = 2;
        while index < words.len() {
            match words[index].literal_text(self.source) {
                Some("finally") => {
                    if let Some(script) = words.get(index + 1) {
                        self.scan_nested_word(script, NestedLanguage::Script, depth);
                    }
                    index += 2;
                }
                Some("on" | "trap") => {
                    if let Some(script) = words.get(index + 3) {
                        self.scan_nested_word(script, NestedLanguage::Script, depth);
                    }
                    index += 4;
                }
                _ => index += 1,
            }
        }
    }

    fn scan_switch_context(&mut self, words: &[WordInfo], depth: u16) {
        let mut subject = 1;
        while words
            .get(subject)
            .and_then(|word| word.literal_text(self.source))
            .is_some_and(|word| word.starts_with('-') && word != "--")
        {
            subject += 1;
        }
        if words.get(subject).and_then(|word| word.literal_text(self.source))
            == Some("--")
        {
            subject += 1;
        }
        let arms = &words[usize::min(subject + 1, words.len())..];
        if arms.len() == 1 {
            let list_words = self.scan_list_words(arms[0].context_range(), depth);
            for body in list_words.iter().skip(1).step_by(2) {
                if body.literal_text(self.source) != Some("-") {
                    self.scan_nested_word(body, NestedLanguage::Script, depth + 1);
                }
            }
        } else {
            for body in arms.iter().skip(1).step_by(2) {
                if body.literal_text(self.source) != Some("-") {
                    self.scan_nested_word(body, NestedLanguage::Script, depth + 1);
                }
            }
        }
    }

    fn scan_list_words(&mut self, range: TextRange, depth: u16) -> Vec<WordInfo> {
        let mut words = Vec::new();
        let mut index = range.start;
        while index < range.end {
            while index < range.end
                && self.char_at(index).is_some_and(char::is_whitespace)
            {
                index = self.next_index(index);
            }
            if index >= range.end {
                break;
            }
            let (word, next) = self.scan_word(index, range.end, depth, None);
            if next == index {
                break;
            }
            words.push(word);
            index = next;
        }
        words
    }

    fn scan_subst_context(&mut self, words: &[WordInfo], depth: u16) {
        let Some(word) = words.last().filter(|_| words.len() >= 2) else {
            return;
        };
        let commands = !words[1..words.len() - 1]
            .iter()
            .any(|word| word.literal_text(self.source) == Some("-nocommands"));
        let variables = !words[1..words.len() - 1]
            .iter()
            .any(|word| word.literal_text(self.source) == Some("-novariables"));
        let backslashes = !words[1..words.len() - 1]
            .iter()
            .any(|word| word.literal_text(self.source) == Some("-nobackslashes"));
        let range = word.context_range();
        self.scan_subst(range.start, range.end, depth, commands, variables, backslashes);
    }

    fn scan_nested_word(
        &mut self,
        word: &WordInfo,
        language: NestedLanguage,
        depth: u16,
    ) {
        // Only literal words can safely be interpreted as nested source before Tcl substitution.
        if !word.is_literal && word.form != WordForm::Braced {
            return;
        }
        let range = word.context_range();
        match language {
            NestedLanguage::Script => {
                self.scan_script(range.start, range.end, None, depth);
            }
            NestedLanguage::Expr => self.scan_expr(range.start, range.end, depth),
        }
    }

    fn scan_subst(
        &mut self,
        start: usize,
        end: usize,
        depth: u16,
        commands: bool,
        variables: bool,
        backslashes: bool,
    ) {
        let mut index = start;
        while index < end {
            match self.char_at(index).expect("index must be valid") {
                '$' if variables => index = self.scan_variable(index, end, depth),
                '[' if commands => index = self.scan_bracket_script(index, end, depth),
                '\\' if backslashes => index = self.scan_escape(index, end, depth),
                _ => {
                    let next = self.next_index(index);
                    self.paint(
                        TextRange::new(index, next),
                        SyntaxKind::String,
                        depth,
                        10,
                    );
                    index = next;
                }
            }
        }
    }

    fn scan_expr(&mut self, start: usize, end: usize, depth: u16) {
        let mut index = start;
        while index < end {
            let ch = self.char_at(index).expect("index must be valid");
            if ch.is_whitespace() {
                let begin = index;
                while index < end && self.char_at(index).is_some_and(char::is_whitespace)
                {
                    index = self.next_index(index);
                }
                self.paint(
                    TextRange::new(begin, index),
                    SyntaxKind::Whitespace,
                    depth,
                    10,
                );
                continue;
            }
            match ch {
                '$' => index = self.scan_variable(index, end, depth),
                '[' => index = self.scan_bracket_script(index, end, depth),
                '\\' => index = self.scan_escape(index, end, depth),
                '"' => {
                    let (_, next) = self.scan_quoted_word(index, end, depth);
                    index = next;
                }
                '{' => {
                    let (_, next) = self.scan_braced_word(index, end, depth);
                    index = next;
                }
                '0'..='9' | '.' if self.looks_like_number(index, end) => {
                    let begin = index;
                    index = self.scan_number(index, end);
                    self.paint(
                        TextRange::new(begin, index),
                        SyntaxKind::Number,
                        depth,
                        25,
                    );
                }
                '+' | '-' | '*' | '/' | '%' | '<' | '>' | '=' | '!' | '~' | '&' | '^'
                | '|' | '?' | ':' => {
                    let begin = index;
                    index = self.next_index(index);
                    if index < end {
                        let pair = &self.source[begin..self.next_index(index)];
                        if matches!(
                            pair,
                            "**" | "<<" | ">>" | "<=" | ">=" | "==" | "!=" | "&&" | "||"
                        ) {
                            index = self.next_index(index);
                        }
                    }
                    self.paint(
                        TextRange::new(begin, index),
                        SyntaxKind::Operator,
                        depth,
                        25,
                    );
                }
                '(' | ')' | ',' => {
                    let next = self.next_index(index);
                    self.paint(
                        TextRange::new(index, next),
                        SyntaxKind::Delimiter,
                        depth,
                        25,
                    );
                    index = next;
                }
                _ if is_expr_identifier_start(ch) => {
                    let begin = index;
                    index = self.next_index(index);
                    while index < end
                        && self.char_at(index).is_some_and(is_expr_identifier_continue)
                    {
                        index = self.next_index(index);
                    }
                    let text = &self.source[begin..index];
                    let kind = if matches!(text, "eq" | "ne" | "in" | "ni") {
                        SyntaxKind::Operator
                    } else if self.char_at(index) == Some('(') {
                        SyntaxKind::Function
                    } else {
                        SyntaxKind::String
                    };
                    self.paint(TextRange::new(begin, index), kind, depth, 25);
                }
                _ => {
                    let next = self.next_index(index);
                    self.paint(
                        TextRange::new(index, next),
                        SyntaxKind::String,
                        depth,
                        10,
                    );
                    index = next;
                }
            }
        }
    }

    fn looks_like_number(&self, index: usize, end: usize) -> bool {
        match self.char_at(index) {
            Some('0'..='9') => true,
            Some('.') => {
                let next = self.next_index(index);
                next < end && matches!(self.char_at(next), Some('0'..='9'))
            }
            _ => false,
        }
    }

    fn scan_number(&self, start: usize, end: usize) -> usize {
        let mut index = start;

        if self.source[index..end].starts_with("0x")
            || self.source[index..end].starts_with("0X")
        {
            index += 2;
            while index < end
                && self.char_at(index).is_some_and(|ch| ch.is_ascii_hexdigit())
            {
                index = self.next_index(index);
            }
            return index;
        }

        while index < end && self.char_at(index).is_some_and(|ch| ch.is_ascii_digit()) {
            index = self.next_index(index);
        }
        if index < end && self.char_at(index) == Some('.') {
            index = self.next_index(index);
            while index < end && self.char_at(index).is_some_and(|ch| ch.is_ascii_digit())
            {
                index = self.next_index(index);
            }
        }
        if index < end && matches!(self.char_at(index), Some('e' | 'E')) {
            let exponent = index;
            index = self.next_index(index);
            if index < end && matches!(self.char_at(index), Some('+' | '-')) {
                index = self.next_index(index);
            }
            let digits = index;
            while index < end && self.char_at(index).is_some_and(|ch| ch.is_ascii_digit())
            {
                index = self.next_index(index);
            }
            if index == digits {
                index = exponent;
            }
        }
        index
    }
}

fn is_var_name_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn is_expr_identifier_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_expr_identifier_continue(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | ':')
}

fn looks_like_scope_level(source: &str) -> bool {
    source.starts_with('#') || source.parse::<i64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysis(source: &str) -> SyntaxAnalysis {
        analyze_script(source, &TCL_86_PROFILE)
    }

    fn pieces(source: &str, kind: SyntaxKind) -> Vec<&str> {
        analysis(source)
            .tokens()
            .iter()
            .filter(|token| token.kind() == kind)
            .map(|token| &source[token.range().start()..token.range().end()])
            .collect()
    }

    #[test]
    fn tokens_cover_source_without_overlap() {
        for source in [
            "",
            "set a 1",
            "# 注释\nputs \"值=$a\"",
            "if {$a > 1} {puts [expr {$a + 1}]}",
        ] {
            let analysis = analysis(source);
            let mut offset = 0;
            for token in analysis.tokens() {
                assert_eq!(token.range().start(), offset);
                assert!(source.is_char_boundary(token.range().start()));
                assert!(source.is_char_boundary(token.range().end()));
                offset = token.range().end();
            }
            assert_eq!(offset, source.len());
        }
    }

    #[test]
    fn distinguishes_complete_incomplete_and_invalid() {
        assert_eq!(analysis("set a 1").status(), ParseStatus::Complete);
        assert_eq!(
            analysis("set a {").status(),
            ParseStatus::Incomplete { kind: IncompleteKind::Brace, opened_at: 6 }
        );
        assert_eq!(analysis("set a {x}tail").status(), ParseStatus::Invalid);
    }

    #[test]
    fn highlights_substitutions_and_comments() {
        let source = "# hi\nputs \"value=$ns::item($index) [get]\"";
        assert_eq!(pieces(source, SyntaxKind::Comment), vec!["# hi"]);
        let variables = pieces(source, SyntaxKind::Variable).concat();
        assert!(variables.contains("$ns::item"));
        assert!(variables.contains("$index"));
        assert_eq!(pieces(source, SyntaxKind::Command), vec!["puts", "get"]);
    }

    #[test]
    fn uses_command_context_for_expressions_and_scripts() {
        let source = "if {$value >= 10} {puts ok}";
        assert!(pieces(source, SyntaxKind::Operator).contains(&">="));
        assert!(pieces(source, SyntaxKind::Command).contains(&"puts"));
    }

    #[test]
    fn highlights_subst_content_and_honors_disable_options() {
        let source = "subst {$value [list a] \\n}";
        assert!(pieces(source, SyntaxKind::Variable).contains(&"$value"));
        assert!(pieces(source, SyntaxKind::Command).contains(&"list"));
        assert!(pieces(source, SyntaxKind::Escape).contains(&"\\n"));

        let source = "subst -nocommands -novariables {$value [list a]}";
        assert!(!pieces(source, SyntaxKind::Variable).contains(&"$value"));
        assert!(!pieces(source, SyntaxKind::Command).contains(&"list"));
    }

    #[test]
    fn folds_backslash_newline_indentation_as_one_escape() {
        let source = "set a foo\\\n   bar";
        assert_eq!(pieces(source, SyntaxKind::Escape), vec!["\\\n   "]);
        assert_eq!(analysis(source).status(), ParseStatus::Complete);

        let source = "set 变量 foo\\\r\n   bar";
        assert_eq!(pieces(source, SyntaxKind::Escape), vec!["\\\r\n   "]);
        assert_eq!(analysis(source).status(), ParseStatus::Complete);
        assert_eq!(pieces("puts $变量::值", SyntaxKind::Variable), vec!["$变量::值"]);
    }

    #[test]
    fn a_folded_newline_keeps_a_comment_open() {
        let source = "# first\\\n  continued\nputs ok";
        assert_eq!(pieces(source, SyntaxKind::Comment), vec!["# first\\\n  continued"]);
        assert_eq!(pieces(source, SyntaxKind::Command), vec!["puts"]);
    }

    #[test]
    fn status_only_matches_full_analysis() {
        for source in
            ["set a 1", "set a {", "set a {x}tail", "puts [list 1]", "if {1} {puts [}"]
        {
            assert_eq!(script_status(source, &TCL_86_PROFILE), analysis(source).status());
        }
    }

    #[test]
    fn nested_literal_script_does_not_change_submission_status() {
        let source = "if {1} {puts [}";
        assert_eq!(script_status(source, &TCL_86_PROFILE), ParseStatus::Complete);
        assert_eq!(analysis(source).status(), ParseStatus::Complete);
    }

    #[test]
    fn expression_numbers_stop_before_binary_operators() {
        let source = "expr {1+2.5e-3}";
        assert_eq!(pieces(source, SyntaxKind::Number), vec!["1", "2.5e-3"]);
        assert_eq!(pieces(source, SyntaxKind::Operator), vec!["+"]);
    }

    #[test]
    fn namespace_separator_requires_a_double_colon() {
        let source = "puts $a:b $::ns::value";
        let variables = pieces(source, SyntaxKind::Variable);
        assert!(variables.contains(&"$a"));
        assert!(variables.contains(&"$::ns::value"));
        assert!(!variables.contains(&"$a:b"));
    }

    #[test]
    fn highlights_apply_and_switch_script_bodies() {
        let source =
            "apply {{x} {expr {$x + 1}}} 2; switch x {x {puts yes} default {puts no}}";
        let commands = pieces(source, SyntaxKind::Command);
        assert!(commands.contains(&"expr"));
        assert_eq!(commands.iter().filter(|command| **command == "puts").count(), 2);
        assert!(pieces(source, SyntaxKind::Operator).contains(&"+"));
    }
}
