use super::{
    AssemblyInputFault as Fault, AssemblyResource as Resource, MathSyntaxReason as Reason,
};
use crate::{
    Callee, CheckedExpression, CheckedNotation, DefinitionId, DefinitionKind, DefinitionSpec,
    ExprDraft, NotationErrors, PrimitiveId, PrimitiveSpec, TextRun, check_document_expression,
};
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MathCommandUse {
    pub definition: DefinitionId,
    pub bytes: Range<usize>,
    pub arguments: Vec<Range<usize>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MathSourceRange {
    pub path: Vec<usize>,
    pub bytes: Range<usize>,
}
/// Source-level presentation that the expression tree cannot encode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MathSpacingIntent {
    Ordinary,
    Thin,
    Medium,
    Thick,
    Quad,
    DoubleQuad,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MathSurfaceEvent {
    Spacing {
        intent: MathSpacingIntent,
        bytes: Range<usize>,
    },
    Group {
        opening: bool,
        bytes: Range<usize>,
    },
    TerminalPunctuation {
        mark: char,
        bytes: Range<usize>,
    },
}
/// A corpus expression checked against the effective notation catalogue.
#[derive(Debug)]
pub struct CheckedCorpusMath<'catalogue, 'book, 'graph> {
    expression: CheckedExpression<'catalogue, 'book, 'graph>,
    surface: Vec<MathSurfaceEvent>,
    commands: Vec<MathCommandUse>,
    ranges: Vec<MathSourceRange>,
}
impl CheckedCorpusMath<'_, '_, '_> {
    #[must_use]
    pub const fn expression(&self) -> &CheckedExpression<'_, '_, '_> {
        &self.expression
    }
    /// Returns checked symbol occurrences, including repeats and transitive uses, sorted by definition token; constructors have no symbol use.
    pub fn effective_dependencies(&self) -> impl Iterator<Item = DefinitionId> + '_ {
        self.expression
            .uses()
            .iter()
            .map(crate::SymbolUse::definition)
    }
    #[must_use]
    pub fn surface(&self) -> &[MathSurfaceEvent] {
        &self.surface
    }
    #[must_use]
    pub fn commands(&self) -> &[MathCommandUse] {
        &self.commands
    }
    #[must_use]
    pub fn ranges(&self) -> &[MathSourceRange] {
        &self.ranges
    }
}
/// Parsed syntax still requires the existing notation expression checker.
#[derive(Debug)]
pub struct ParsedAssemblyMath {
    draft: ExprDraft,
    commands: Vec<MathCommandUse>,
    ranges: Vec<MathSourceRange>,
    surface: Vec<MathSurfaceEvent>,
}
/// A corpus parse retains presentation boundaries until notation checking.
#[derive(Debug)]
pub struct ParsedCorpusMath<'catalogue, 'book, 'graph> {
    parsed: ParsedAssemblyMath,
    catalogue: &'catalogue CheckedNotation<'book, 'graph>,
}
impl<'catalogue, 'book, 'graph> ParsedCorpusMath<'catalogue, 'book, 'graph> {
    #[must_use]
    pub const fn draft(&self) -> &ExprDraft {
        &self.parsed.draft
    }
    #[must_use]
    pub fn surface(&self) -> &[MathSurfaceEvent] {
        &self.parsed.surface
    }
    /// Check the corpus expression and retain its source presentation events.
    /// # Errors
    /// Returns notation diagnostics for an invalid checked expression.
    pub fn check_corpus(
        self,
    ) -> std::result::Result<CheckedCorpusMath<'catalogue, 'book, 'graph>, NotationErrors> {
        let parsed = self.parsed;
        let expression = check_document_expression(self.catalogue, parsed.draft)?;
        Ok(CheckedCorpusMath {
            expression,
            surface: parsed.surface,
            commands: parsed.commands,
            ranges: parsed.ranges,
        })
    }
}
impl ParsedAssemblyMath {
    /// Parse one complete corpus expression, including its terminal punctuation.
    /// # Errors
    /// Returns the first decisive source range and parser fault.
    pub fn parse_corpus<'catalogue, 'book, 'graph>(
        catalogue: &'catalogue CheckedNotation<'book, 'graph>,
        source: &str,
    ) -> std::result::Result<ParsedCorpusMath<'catalogue, 'book, 'graph>, (Range<usize>, Fault)>
    {
        parse_inner(Vocabulary::Checked(catalogue), source, false, true)
            .map(|parsed| ParsedCorpusMath { parsed, catalogue })
            .map_err(|failure| *failure)
    }
    /// Transfer the bounded syntax tree to the existing notation checker.
    #[must_use]
    pub fn into_draft(self) -> ExprDraft {
        self.draft
    }
    #[must_use]
    pub const fn draft(&self) -> &ExprDraft {
        &self.draft
    }
    #[must_use]
    pub fn commands(&self) -> &[MathCommandUse] {
        &self.commands
    }
    #[must_use]
    pub fn ranges(&self) -> &[MathSourceRange] {
        &self.ranges
    }
    #[must_use]
    pub fn surface(&self) -> &[MathSurfaceEvent] {
        &self.surface
    }
}
// Keep the larger assembly fault family off recursive parser stack frames.
// The public result still returns the original range/fault pair.
type Failure = Box<(Range<usize>, Fault)>;
type Result<T> = std::result::Result<T, Failure>;
#[derive(Clone)]
struct Token {
    text: String,
    bytes: Range<usize>,
    decoded_text: Option<(String, Range<usize>)>,
}
struct Node {
    draft: ExprDraft,
    bytes: Range<usize>,
    index: usize,
}
enum Vocabulary<'a, 'b, 'g> {
    Checked(&'a CheckedNotation<'b, 'g>),
    Construction(&'a [PrimitiveSpec], &'a [DefinitionSpec]),
}
impl Vocabulary<'_, '_, '_> {
    fn primitives(&self) -> &[PrimitiveSpec] {
        match self {
            Self::Checked(catalogue) => catalogue.primitives(),
            Self::Construction(primitives, _) => primitives,
        }
    }
    fn definition(&self, token: &str) -> Option<(DefinitionId, u8, bool)> {
        match self {
            Self::Checked(catalogue) => catalogue.definitions().iter().find_map(|entry| {
                (entry.command() == token).then(|| {
                    let spec = entry.spec();
                    (
                        spec.id(),
                        spec.signature().arity,
                        matches!(spec.kind(), DefinitionKind::Inherited { .. }),
                    )
                })
            }),
            Self::Construction(_, definitions) => definitions.iter().find_map(|spec| {
                (spec.command() == token).then(|| {
                    (
                        spec.id(),
                        spec.signature().arity,
                        matches!(spec.kind(), DefinitionKind::Inherited { .. }),
                    )
                })
            }),
        }
    }
    fn inherited(&self, id: DefinitionId) -> bool {
        match self {
            Self::Checked(catalogue) => matches!(
                catalogue.definition(id).spec().kind(),
                DefinitionKind::Inherited { .. }
            ),
            Self::Construction(_, definitions) => definitions.iter().any(|spec| {
                spec.id() == id && matches!(spec.kind(), DefinitionKind::Inherited { .. })
            }),
        }
    }
}
fn corpus_admits(callee: Callee) -> bool {
    match callee {
        Callee::Primitive(id) => matches!(
            id,
            PrimitiveId::Nabla
                | PrimitiveId::LessEqual
                | PrimitiveId::Subscript
                | PrimitiveId::Superscript
                | PrimitiveId::Multiply
                | PrimitiveId::Juxtapose
        ),
        Callee::Definition(id) => matches!(
            id,
            DefinitionId::Comovfield | DefinitionId::Rewardfield | DefinitionId::Accountability
        ),
    }
}
struct Parser<'a, 'b, 'g> {
    vocabulary: Vocabulary<'a, 'b, 'g>,
    tokens: Vec<Token>,
    at: usize,
    eof: usize,
    depth: usize,
    nodes: usize,
    commands: Vec<MathCommandUse>,
    arena: Vec<(Range<usize>, Vec<usize>)>,
    display: bool,
    corpus: bool,
}
fn limit(bytes: Range<usize>, resource: Resource, maximum: usize) -> Failure {
    Box::new((
        bytes,
        Fault::Limit {
            resource,
            limit: maximum,
            actual: maximum + 1,
        },
    ))
}
#[allow(clippy::too_many_lines)] // Keep token and text-body ranges in one bounded scan.
fn lex(source: &str) -> Result<Vec<Token>> {
    if source.len() > 262_144 {
        return Err(limit(262_144..262_145, Resource::MathBytes, 262_144));
    }
    let mut chars = source.char_indices().peekable();
    let mut tokens = Vec::new();
    while let Some((start, c)) = chars.next() {
        if matches!(c, ' ' | '\t' | '\r' | '\n') {
            continue;
        }
        let mut end = start + c.len_utf8();
        if c == '\\' {
            if let Some(&(at, next)) = chars.peek() {
                if next.is_ascii_alphabetic() {
                    while let Some(&(at, next)) = chars.peek() {
                        if !next.is_ascii_alphabetic() {
                            break;
                        }
                        end = at + 1;
                        chars.next();
                    }
                } else {
                    end = at + next.len_utf8();
                    chars.next();
                }
            }
        } else if c.is_ascii_digit() {
            while let Some(&(at, next)) = chars.peek() {
                if !next.is_ascii_digit() {
                    break;
                }
                end = at + 1;
                chars.next();
            }
        }
        if &source[start..end] == "\\text" && chars.peek().is_some_and(|(_, c)| *c == '{') {
            chars.next();
            let body_start = end + 1;
            let mut body_end = body_start;
            let mut closed = false;
            let mut previous = None;
            for (at, value) in chars.by_ref() {
                if value == '}' {
                    end = at + 1;
                    closed = true;
                    break;
                }
                if !value.is_ascii_alphabetic() && !matches!(value, ' ' | ',' | '-') {
                    return Err(Box::new((
                        at..at + value.len_utf8(),
                        Fault::MathSyntax {
                            reason: Reason::InvalidTextCharacter,
                            token: value.to_string(),
                            expected: vec!["ASCII letter, space, comma or hyphen".into()],
                        },
                    )));
                }
                if previous == Some(value) && matches!(value, ' ' | ',' | '-') {
                    return Err(Box::new((
                        at..at + 1,
                        Fault::MathSyntax {
                            reason: Reason::InvalidTextCharacter,
                            token: value.to_string(),
                            expected: vec!["no adjacent spaces, commas or hyphens".into()],
                        },
                    )));
                }
                previous = Some(value);
                body_end = at + value.len_utf8();
                if body_end - body_start > 256 {
                    return Err(limit(at..at + 1, Resource::MathTextBytes, 256));
                }
            }
            if !closed {
                return Err(Box::new((
                    start..start + 5,
                    Fault::MathSyntax {
                        reason: Reason::UnclosedText,
                        token: "\\text".into(),
                        expected: vec!["}".into()],
                    },
                )));
            }
            if source[body_start..body_end].trim_matches(' ').is_empty() {
                return Err(Box::new((
                    body_start..body_end.max(body_start + 1),
                    Fault::MathSyntax {
                        reason: Reason::EmptyText,
                        token: source[body_start..body_end].into(),
                        expected: vec!["non-space text".into()],
                    },
                )));
            }
            if tokens.len() == 16_384 {
                return Err(limit(start..end, Resource::MathTokens, 16_384));
            }
            tokens.push(Token {
                text: "\\text".into(),
                bytes: start..end,
                decoded_text: Some((source[body_start..body_end].into(), body_start..body_end)),
            });
            continue;
        }
        if tokens.len() == 16_384 {
            return Err(limit(start..end, Resource::MathTokens, 16_384));
        }
        tokens.push(Token {
            text: source[start..end].into(),
            bytes: start..end,
            decoded_text: None,
        });
    }
    Ok(tokens)
}
impl Parser<'_, '_, '_> {
    fn token(&self) -> Token {
        self.tokens.get(self.at).cloned().unwrap_or(Token {
            text: String::new(),
            bytes: self.eof..self.eof,
            decoded_text: None,
        })
    }
    fn is(&self, text: &str) -> bool {
        self.token().text == text
    }
    fn take(&mut self) -> Token {
        let token = self.token();
        self.at += usize::from(self.at < self.tokens.len());
        token
    }
    fn syntax(&self, reason: Reason, expected: &[&str]) -> Failure {
        let token = self.token();
        Box::new((
            token.bytes,
            Fault::MathSyntax {
                reason,
                token: token.text,
                expected: expected.iter().map(|s| (*s).into()).collect(),
            },
        ))
    }
    fn call(&mut self, callee: Callee, arguments: Vec<Node>, bytes: Range<usize>) -> Result<Node> {
        // Backstop: every corpus path refuses earlier, at its source token.
        if self.corpus && !corpus_admits(callee) {
            return Err(Box::new((
                bytes.clone(),
                Fault::MathSyntax {
                    reason: Reason::UnexpectedToken,
                    token: format!("{callee:?}"),
                    expected: vec!["admitted corpus operation or definition".into()],
                },
            )));
        }
        if self.nodes == 4096 {
            return Err(limit(bytes, Resource::MathNodes, 4096));
        }
        self.nodes += 1;
        let index = self.arena.len();
        self.arena
            .push((bytes.clone(), arguments.iter().map(|n| n.index).collect()));
        let draft = ExprDraft::Call {
            callee,
            arguments: arguments.into_iter().map(|n| n.draft).collect(),
        };
        Ok(Node {
            draft,
            bytes,
            index,
        })
    }
    fn primitive(&mut self, id: PrimitiveId, args: Vec<Node>, bytes: Range<usize>) -> Result<Node> {
        self.call(Callee::Primitive(id), args, bytes)
    }
    fn equality(&mut self) -> Result<Node> {
        self.equality_inner()
    }
    fn equality_inner(&mut self) -> Result<Node> {
        let left = self.sum()?;
        if self.corpus && self.is("\\le") {
            if self.depth > 0 {
                return Err(self.syntax(Reason::UnexpectedToken, &["top-level relation"]));
            }
            self.take();
            let right = self.sum()?;
            let bytes = left.bytes.start..right.bytes.end;
            return self.primitive(PrimitiveId::LessEqual, vec![left, right], bytes);
        }
        if self.display
            && self.is("\\;")
            && self.tokens.get(self.at + 1).is_some_and(|t| t.text == "=")
        {
            self.take();
        } else if self.display && self.is("=") {
            return Err(self.syntax(Reason::UnexpectedToken, &["\\;"]));
        }
        if self.corpus && self.is("=") {
            return Err(self.syntax(Reason::UnexpectedToken, &["\\le"]));
        }
        if !self.is("=") {
            return Ok(left);
        }
        let operator = self.take();
        if self.display && !self.is("\\;") {
            return Err(Box::new((
                operator.bytes,
                Fault::MathSyntax {
                    reason: Reason::UnexpectedToken,
                    token: operator.text,
                    expected: vec!["\\;".into()],
                },
            )));
        }
        if self.display {
            self.take();
        }
        let right = self.sum()?;
        let bytes = left.bytes.start..right.bytes.end;
        let result = self.primitive(PrimitiveId::Equal, vec![left, right], bytes)?;
        if self.is("=") {
            return Err(self.syntax(Reason::ChainedEquality, &["end of expression"]));
        }
        Ok(result)
    }
    fn minus(&self) -> bool {
        self.is("-") || self.is("−")
    }
    fn sum(&mut self) -> Result<Node> {
        let mut left = self.product()?;
        if self.corpus && (self.is("+") || self.minus()) {
            return Err(self.syntax(Reason::UnexpectedToken, &["\\le", "\\cdot"]));
        }
        while self.is("+")
            || self.minus()
            || (self.display
                && self.is("\\;")
                && self
                    .tokens
                    .get(self.at + 1)
                    .is_some_and(|t| matches!(t.text.as_str(), "+" | "-")))
        {
            if self.display && !self.is("\\;") {
                return Err(self.syntax(Reason::UnexpectedToken, &["\\;"]));
            }
            if self.display && self.is("\\;") {
                self.take();
            }
            let negate = self.minus();
            let sign = self.take();
            if self.display && !self.is("\\;") {
                return Err(Box::new((
                    sign.bytes,
                    Fault::MathSyntax {
                        reason: Reason::UnexpectedToken,
                        token: sign.text,
                        expected: vec!["\\;".into()],
                    },
                )));
            }
            if self.display {
                self.take();
            }
            let mut right = self.product()?;
            if negate && !self.display {
                let bytes = sign.bytes.start..right.bytes.end;
                right = self.primitive(PrimitiveId::Negate, vec![right], bytes)?;
            }
            let bytes = left.bytes.start..right.bytes.end;
            left = self.primitive(
                if negate && self.display {
                    PrimitiveId::Subtract
                } else {
                    PrimitiveId::Add
                },
                vec![left, right],
                bytes,
            )?;
        }
        Ok(left)
    }
    fn infix(&self) -> Option<PrimitiveId> {
        let text = self.token().text;
        self.vocabulary
            .primitives()
            .iter()
            .find(|p| {
                matches!(
                    p.id(),
                    PrimitiveId::Multiply | PrimitiveId::Juxtapose | PrimitiveId::Times
                ) && (p.operation().host_tokens().contains(&text.as_str())
                    || p.operation().unicode_scalar() == Some(text.as_str()))
            })
            .map(crate::PrimitiveSpec::id)
    }
    fn atom_start(&self) -> bool {
        let token = self.token();
        if self.corpus && token.text == "\\le" {
            return false;
        }
        token
            .text
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            || matches!(token.text.as_str(), "{" | "(" | "[" | "α" | "β")
            || (token.text.starts_with('\\') && self.infix().is_none())
    }
    fn product(&mut self) -> Result<Node> {
        let mut left = self.unary()?;
        loop {
            if self.display && self.is("\\;") {
                break;
            }
            let id = if let Some(id) = self.infix() {
                if self.corpus && !corpus_admits(Callee::Primitive(id)) {
                    return Err(self.syntax(Reason::UnexpectedToken, &["\\cdot"]));
                }
                self.take();
                id
            } else if self.atom_start() {
                PrimitiveId::Juxtapose
            } else {
                break;
            };
            let right = if self.corpus && id == PrimitiveId::Multiply {
                self.corpus_implicit_run()?
            } else {
                self.unary()?
            };
            let bytes = left.bytes.start..right.bytes.end;
            left = self.primitive(id, vec![left, right], bytes)?;
        }
        Ok(left)
    }
    fn corpus_implicit_run(&mut self) -> Result<Node> {
        let mut left = self.unary()?;
        while self.atom_start() && self.infix().is_none() {
            let right = self.unary()?;
            let bytes = left.bytes.start..right.bytes.end;
            left = self.primitive(PrimitiveId::Juxtapose, vec![left, right], bytes)?;
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<Node> {
        if self.corpus && self.minus() {
            return Err(self.syntax(Reason::UnexpectedToken, &["corpus atom"]));
        }
        let mut signs = Vec::new();
        while self.minus() {
            if signs.len() == 64 {
                return Err(limit(self.token().bytes, Resource::MathDepth, 64));
            }
            signs.push(self.take().bytes);
        }
        let mut node = self.scripted()?;
        for sign in signs.into_iter().rev() {
            let bytes = sign.start..node.bytes.end;
            node = self.primitive(PrimitiveId::Negate, vec![node], bytes)?;
        }
        Ok(node)
    }
    fn scripted(&mut self) -> Result<Node> {
        let mut node = self.atom()?;
        let mut sub = None;
        let mut sup = None;
        while self.is("_") || self.is("^") {
            let is_sub = self.is("_");
            if self.corpus && (sub.is_some() || sup.is_some()) {
                return Err(self.syntax(Reason::UnexpectedToken, &["one script per base"]));
            }
            if (is_sub && sub.is_some()) || (!is_sub && sup.is_some()) {
                return Err(self.syntax(
                    if is_sub {
                        Reason::RepeatedSubscript
                    } else {
                        Reason::RepeatedSuperscript
                    },
                    &["atom", "operator", "end of expression"],
                ));
            }
            self.take();
            if self.corpus {
                let argument = self.token();
                if argument.text != "{"
                    && !(argument.text.len() == 1
                        && argument.text.as_bytes()[0].is_ascii_alphanumeric())
                {
                    return Err(self.syntax(Reason::UnexpectedToken, &["{...}", "one character"]));
                }
            }
            let argument = self.atom()?;
            if is_sub {
                sub = Some(argument);
            } else {
                sup = Some(argument);
            }
        }
        if let Some(arg) = sub {
            let bytes = node.bytes.start..node.bytes.end.max(arg.bytes.end);
            node = self.primitive(PrimitiveId::Subscript, vec![node, arg], bytes)?;
        }
        if let Some(arg) = sup {
            let bytes = node.bytes.start..node.bytes.end.max(arg.bytes.end);
            node = self.primitive(PrimitiveId::Superscript, vec![node, arg], bytes)?;
        }
        Ok(node)
    }
    fn group(&mut self, closer: &str) -> Result<Node> {
        if self.depth == 64 {
            return Err(limit(self.token().bytes, Resource::MathDepth, 64));
        }
        let open = self.take();
        if self.is(closer) {
            return Err(self.syntax(Reason::EmptyGroup, &["atom"]));
        }
        self.depth += 1;
        let result = self.equality();
        self.depth -= 1;
        let mut node = result?;
        if !self.is(closer) {
            return Err(self.syntax(
                if self.at == self.tokens.len() {
                    Reason::UnclosedGroup
                } else if matches!(self.token().text.as_str(), "}" | ")" | "]") {
                    Reason::MismatchedCloser
                } else {
                    Reason::UnexpectedToken
                },
                &[closer],
            ));
        }
        let close = self.take();
        node.bytes = open.bytes.start..close.bytes.end;
        self.arena[node.index].0 = node.bytes.clone();
        Ok(node)
    }
    #[allow(clippy::too_many_lines)] // Every display-only atom checks its original token site.
    fn atom(&mut self) -> Result<Node> {
        let token = self.token();
        if let Some((value, source)) = token.decoded_text.clone() {
            if !self.display {
                return Err(self.syntax(Reason::UnexpectedToken, &["inline atom"]));
            }
            self.take();
            if self.nodes == 4096 {
                return Err(limit(token.bytes, Resource::MathNodes, 4096));
            }
            self.nodes += 1;
            let index = self.arena.len();
            self.arena.push((token.bytes.clone(), Vec::new()));
            return Ok(Node {
                draft: ExprDraft::Text {
                    runs: vec![TextRun {
                        decoded: 0..value.len(),
                        text: value,
                        source,
                    }],
                },
                bytes: token.bytes,
                index,
            });
        }
        match token.text.as_str() {
            "{" => return self.group("}"),
            "(" | "[" => {
                if self.corpus {
                    return Err(self.syntax(Reason::UnexpectedToken, &["corpus atom"]));
                }
                let (closer, id) = if token.text == "(" {
                    (")", PrimitiveId::Parentheses)
                } else {
                    ("]", PrimitiveId::Dimension)
                };
                let node = self.group(closer)?;
                let bytes = node.bytes.clone();
                return self.primitive(id, vec![node], bytes);
            }
            _ => {}
        }
        if token.text == "\\underbrace" {
            if !self.display {
                return Err(self.syntax(Reason::UnexpectedToken, &["inline atom"]));
            }
            self.take();
            if !self.is("{") {
                return Err(self.syntax(Reason::UnexpectedToken, &["{"]));
            }
            let base = self.group("}")?;
            if !self.is("_") {
                return Err(self.syntax(Reason::UnexpectedToken, &["_"]));
            }
            self.take();
            if !self.is("{") {
                return Err(self.syntax(Reason::UnexpectedToken, &["{"]));
            }
            let annotation = self.group("}")?;
            if let ExprDraft::Text { runs } = &annotation.draft
                && runs
                    .iter()
                    .any(|run| run.text.trim_matches(' ') == "pending")
            {
                return Err(Box::new((
                    runs[0].source.clone(),
                    Fault::MathSyntax {
                        reason: Reason::PendingAnnotation,
                        token: "pending".into(),
                        expected: vec!["authored annotation".into()],
                    },
                )));
            }
            let bytes = token.bytes.start..annotation.bytes.end;
            return self.primitive(PrimitiveId::Underbrace, vec![base, annotation], bytes);
        }
        let primitive = self.vocabulary.primitives().iter().find(|p| {
            (p.operation().is_symbol()
                || matches!(p.id(), PrimitiveId::Fraction | PrimitiveId::Dot))
                && (p.operation().host_tokens().contains(&token.text.as_str())
                    || p.operation().unicode_scalar() == Some(token.text.as_str()))
        });
        let definition = self.vocabulary.definition(&token.text);
        let signature = primitive
            .map(|p| (Callee::Primitive(p.id()), p.signature().arity))
            .or_else(|| definition.map(|(id, arity, _)| (Callee::Definition(id), arity)));
        if let Some((callee, arity)) = signature {
            return self.command(token, callee, arity);
        }
        self.literal(token)
    }
    fn command(&mut self, token: Token, callee: Callee, arity: u8) -> Result<Node> {
        if self.corpus && !corpus_admits(callee) {
            return Err(Box::new((
                token.bytes,
                Fault::MathSyntax {
                    reason: Reason::UnexpectedToken,
                    token: token.text,
                    expected: vec!["admitted corpus operation or definition".into()],
                },
            )));
        }
        self.take();
        if let Callee::Definition(id) = callee
            && self.vocabulary.inherited(id)
            && arity == 0
            && self.is("{")
            && self
                .tokens
                .get(self.at + 1)
                .is_none_or(|next| next.text != "}")
        {
            return Err(Box::new((
                self.token().bytes,
                Fault::MathArguments {
                    command: token.text,
                    expected: 0,
                    actual: 1,
                    argument: 0,
                },
            )));
        }
        let mut arguments = Vec::new();
        let mut ranges = Vec::new();
        for argument in 0..arity {
            if !self.is("{") {
                return Err(Box::new((
                    self.token().bytes,
                    Fault::MathArguments {
                        command: token.text,
                        expected: arity,
                        actual: arguments.len(),
                        argument: usize::from(argument),
                    },
                )));
            }
            let node = self.group("}")?;
            ranges.push(node.bytes.clone());
            arguments.push(node);
        }
        let mut end = ranges.last().map_or(token.bytes.end, |r| r.end);
        if arity == 0 && self.is("{") && self.tokens.get(self.at + 1).is_some_and(|t| t.text == "}")
        {
            self.take();
            end = self.take().bytes.end;
            if self.corpus && (self.is("_") || self.is("^")) {
                return Err(self.syntax(Reason::UnexpectedToken, &["script on the preceding atom"]));
            }
        }
        if let Callee::Definition(definition) = callee {
            self.commands.push(MathCommandUse {
                definition,
                bytes: token.bytes.clone(),
                arguments: ranges,
            });
        }
        self.call(callee, arguments, token.bytes.start..end)
    }
    fn literal(&mut self, token: Token) -> Result<Node> {
        if self
            .vocabulary
            .primitives()
            .iter()
            .any(|p| p.operation().host_tokens().contains(&token.text.as_str()))
        {
            return Err(self.syntax(
                Reason::ExpectedAtom,
                &["letter", "digits", "command", "{", "(", "["],
            ));
        }
        if token.text.starts_with('\\')
            && token.text[1..].bytes().all(|b| b.is_ascii_alphabetic())
            && token.text.len() > 1
        {
            return Err(Box::new((
                token.bytes,
                Fault::UnknownMathCommand {
                    command: token.text,
                },
            )));
        }
        if (token.text.len() == 1 && token.text.as_bytes()[0].is_ascii_alphabetic())
            || (!token.text.is_empty() && token.text.bytes().all(|b| b.is_ascii_digit()))
        {
            self.take();
            if self.nodes == 4096 {
                return Err(limit(token.bytes, Resource::MathNodes, 4096));
            }
            self.nodes += 1;
            let index = self.arena.len();
            self.arena.push((token.bytes.clone(), Vec::new()));
            return Ok(Node {
                draft: ExprDraft::Literal { value: token.text },
                bytes: token.bytes,
                index,
            });
        }
        Err(self.syntax(
            Reason::ExpectedAtom,
            &["letter", "digits", "command", "{", "(", "["],
        ))
    }
}

/// Parse the complete authored entry without evaluating any TeX.
/// # Errors
/// Returns the first decisive decoded range and complete parser fault.
pub fn parse_assembly_math(
    catalogue: &CheckedNotation<'_, '_>,
    source: &str,
) -> std::result::Result<ParsedAssemblyMath, (Range<usize>, Fault)> {
    parse_inner(Vocabulary::Checked(catalogue), source, false, false).map_err(|failure| *failure)
}
/// Parse a mathematical display with its explicit thick-space boundaries.
/// # Errors
/// Returns the first decisive decoded range and complete parser fault.
pub fn parse_assembly_display_math(
    catalogue: &CheckedNotation<'_, '_>,
    source: &str,
) -> std::result::Result<ParsedAssemblyMath, (Range<usize>, Fault)> {
    parse_inner(Vocabulary::Checked(catalogue), source, true, false).map_err(|failure| *failure)
}
/// Parse an inherited declaration body with the same grammar and bounds as document mathematics.
pub(crate) fn parse_catalogue_body(
    primitives: &[PrimitiveSpec],
    definitions: &[DefinitionSpec],
    source: &str,
) -> std::result::Result<ParsedAssemblyMath, (Range<usize>, Fault)> {
    parse_inner(
        Vocabulary::Construction(primitives, definitions),
        source,
        false,
        false,
    )
    .map_err(|failure| *failure)
}
/// Parse the complete authored entry without evaluating any TeX.
/// # Errors
/// Returns the first decisive decoded range and complete parser fault.
#[allow(clippy::too_many_lines)] // Bounded surface extraction and parsing share the same source order.
fn parse_inner(
    vocabulary: Vocabulary<'_, '_, '_>,
    source: &str,
    display: bool,
    corpus: bool,
) -> Result<ParsedAssemblyMath> {
    let mut tokens = lex(source)?;
    let mut surface = Vec::new();
    if corpus {
        let mut retained = Vec::new();
        let mut previous_end = 0;
        for (index, token) in tokens.iter().cloned().enumerate() {
            if previous_end < token.bytes.start {
                surface.push(MathSurfaceEvent::Spacing {
                    intent: MathSpacingIntent::Ordinary,
                    bytes: previous_end..token.bytes.start,
                });
            }
            previous_end = token.bytes.end;
            let intent = match token.text.as_str() {
                "\\," => Some(MathSpacingIntent::Thin),
                "\\:" => Some(MathSpacingIntent::Medium),
                "\\;" => Some(MathSpacingIntent::Thick),
                "\\quad" => Some(MathSpacingIntent::Quad),
                "\\qquad" => Some(MathSpacingIntent::DoubleQuad),
                _ => None,
            };
            if let Some(intent) = intent {
                if (index > 0 && matches!(tokens[index - 1].text.as_str(), "_" | "^"))
                    || tokens
                        .get(index + 1)
                        .is_some_and(|next| matches!(next.text.as_str(), "_" | "^"))
                {
                    return Err(Box::new((
                        token.bytes,
                        Fault::MathSyntax {
                            reason: Reason::UnexpectedToken,
                            token: token.text,
                            expected: vec!["spacing away from scripts".into()],
                        },
                    )));
                }
                surface.push(MathSurfaceEvent::Spacing {
                    intent,
                    bytes: token.bytes,
                });
            } else {
                if matches!(token.text.as_str(), "{" | "(" | "[") {
                    surface.push(MathSurfaceEvent::Group {
                        opening: true,
                        bytes: token.bytes.clone(),
                    });
                } else if matches!(token.text.as_str(), "}" | ")" | "]") {
                    surface.push(MathSurfaceEvent::Group {
                        opening: false,
                        bytes: token.bytes.clone(),
                    });
                }
                retained.push(token);
            }
        }
        if previous_end < source.len() {
            surface.push(MathSurfaceEvent::Spacing {
                intent: MathSpacingIntent::Ordinary,
                bytes: previous_end..source.len(),
            });
        }
        tokens = retained;
    }
    let mut parser = Parser {
        vocabulary,
        tokens,
        at: 0,
        eof: source.len(),
        depth: 0,
        nodes: 0,
        commands: Vec::new(),
        arena: Vec::new(),
        display,
        corpus,
    };
    let root = parser.equality()?;
    if corpus && matches!(parser.token().text.as_str(), "," | ".") {
        let token = parser.take();
        if parser.at == parser.tokens.len() {
            surface.push(MathSurfaceEvent::TerminalPunctuation {
                mark: token.text.chars().next().expect("punctuation token"),
                bytes: token.bytes,
            });
        } else {
            return Err(Box::new((
                token.bytes,
                Fault::MathSyntax {
                    reason: Reason::UnexpectedToken,
                    token: token.text,
                    expected: vec!["terminal punctuation".into()],
                },
            )));
        }
    }
    if parser.at != parser.tokens.len() {
        return Err(parser.syntax(Reason::UnexpectedToken, &["end of expression"]));
    }
    surface.sort_by_key(|event| match event {
        MathSurfaceEvent::Spacing { bytes, .. }
        | MathSurfaceEvent::Group { bytes, .. }
        | MathSurfaceEvent::TerminalPunctuation { bytes, .. } => bytes.start,
    });
    let mut ranges = Vec::new();
    let mut stack = vec![(root.index, Vec::new())];
    while let Some((node, path)) = stack.pop() {
        let (bytes, children) = &parser.arena[node];
        ranges.push(MathSourceRange {
            path: path.clone(),
            bytes: bytes.clone(),
        });
        for (index, child) in children.iter().enumerate().rev() {
            let mut path = path.clone();
            path.push(index);
            stack.push((*child, path));
        }
    }
    parser.commands.sort_by_key(|c| c.bytes.start);
    Ok(ParsedAssemblyMath {
        draft: root.draft,
        commands: parser.commands,
        ranges,
        surface,
    })
}

#[cfg(test)]
mod inherited_tests {
    use super::*;
    use crate::{addressing, check_expression, notation, seed};

    #[test]
    fn corpus_equation_keeps_operations_dependencies_and_surface() {
        let graph = seed().unwrap();
        let book = addressing(&graph).unwrap();
        let catalogue = notation(&book).unwrap();
        let paper = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../documents/paper/parts/partA.tex"
        ));
        let source = paper
            .lines()
            .find(|line| line.contains(r"\nabla_{\comovfield}\rewardfield"))
            .unwrap();
        let parsed = ParsedAssemblyMath::parse_corpus(&catalogue, source).unwrap();
        let primitive = |id, arguments| ExprDraft::Call {
            callee: Callee::Primitive(id),
            arguments,
        };
        let definition = |id| ExprDraft::Call {
            callee: Callee::Definition(id),
            arguments: vec![],
        };
        let gradient = |field| {
            primitive(
                PrimitiveId::Juxtapose,
                vec![
                    primitive(
                        PrimitiveId::Subscript,
                        vec![
                            primitive(PrimitiveId::Nabla, vec![]),
                            definition(DefinitionId::Comovfield),
                        ],
                    ),
                    definition(field),
                ],
            )
        };
        assert_eq!(
            parsed.draft(),
            &primitive(
                PrimitiveId::LessEqual,
                vec![
                    primitive(
                        PrimitiveId::Multiply,
                        vec![
                            gradient(DefinitionId::Rewardfield),
                            gradient(DefinitionId::Accountability)
                        ],
                    ),
                    ExprDraft::Literal { value: "0".into() },
                ],
            )
        );
        assert!(parsed.surface().iter().any(|event| matches!(event, MathSurfaceEvent::Spacing { intent: MathSpacingIntent::Ordinary, bytes } if &source[bytes.clone()] == "    ")));
        assert!(parsed.surface().iter().any(|event| matches!(event, MathSurfaceEvent::TerminalPunctuation { mark: ',', bytes } if &source[bytes.clone()] == ",")));
        assert_eq!(
            parsed
                .surface()
                .iter()
                .filter(|event| matches!(event, MathSurfaceEvent::Group { .. }))
                .count(),
            4
        );
        let checked = parsed.check_corpus().unwrap();
        let dependencies: Vec<_> = checked.effective_dependencies().collect();
        for id in [
            DefinitionId::Comovfield,
            DefinitionId::Rewardfield,
            DefinitionId::Accountability,
        ] {
            assert!(dependencies.contains(&id));
        }
    }

    #[test]
    fn corpus_spacing_is_named_and_unsupported_forms_refuse_at_source() {
        let graph = seed().unwrap();
        let book = addressing(&graph).unwrap();
        let catalogue = notation(&book).unwrap();
        for (token, intent) in [
            (r"\,", MathSpacingIntent::Thin),
            (r"\:", MathSpacingIntent::Medium),
            (r"\;", MathSpacingIntent::Thick),
            (r"\quad", MathSpacingIntent::Quad),
            (r"\qquad", MathSpacingIntent::DoubleQuad),
        ] {
            let source = format!(r"\nabla{token}\rewardfield");
            let parsed = ParsedAssemblyMath::parse_corpus(&catalogue, &source).unwrap();
            assert!(parsed.surface().iter().any(|event| matches!(event, MathSurfaceEvent::Spacing { intent: actual, bytes } if *actual == intent && &source[bytes.clone()] == token)));
        }
        let superscript = ParsedAssemblyMath::parse_corpus(&catalogue, r"\nabla^2").unwrap();
        assert!(matches!(
            superscript.draft(),
            ExprDraft::Call {
                callee: Callee::Primitive(PrimitiveId::Superscript),
                ..
            }
        ));
        let comma = ParsedAssemblyMath::parse_corpus(&catalogue, r"\nabla,").unwrap();
        let period = ParsedAssemblyMath::parse_corpus(&catalogue, r"\nabla.").unwrap();
        assert!(matches!(
            comma.surface().last(),
            Some(MathSurfaceEvent::TerminalPunctuation { mark: ',', .. })
        ));
        assert!(matches!(
            period.surface().last(),
            Some(MathSurfaceEvent::TerminalPunctuation { mark: '.', .. })
        ));
        assert_ne!(
            comma.surface(),
            ParsedAssemblyMath::parse_corpus(&catalogue, r"{\nabla},")
                .unwrap()
                .surface()
        );
        for (source, token, reason) in [
            (r"\nabla\!\rewardfield", r"\!", Some(Reason::ExpectedAtom)),
            (r"\sqrt{0}", r"\sqrt", None),
            (r"\nabla\ge0", r"\ge", None),
            (r"\readout", r"\readout", Some(Reason::UnexpectedToken)),
            (r"\nabla+0", "+", Some(Reason::UnexpectedToken)),
            (r"\nabla=0", "=", Some(Reason::UnexpectedToken)),
            (r"\DisplayClass{dynamics}", r"\DisplayClass", None),
            (r"\nabla\label{eq:f:opp}", r"\label", None),
            (
                r"\nabla\times\rewardfield",
                r"\times",
                Some(Reason::UnexpectedToken),
            ),
            (r"\alpha", r"\alpha", Some(Reason::UnexpectedToken)),
            (r"(\nabla)", "(", Some(Reason::UnexpectedToken)),
            (r"{\nabla\le0}", r"\le", Some(Reason::UnexpectedToken)),
            (r"\nabla^23", "23", Some(Reason::UnexpectedToken)),
            (r"\nabla\,_{c}", r"\,", Some(Reason::UnexpectedToken)),
            (r"\nabla_\,{c}", r"\,", Some(Reason::UnexpectedToken)),
            (r"\nabla{}_{c}", "_", Some(Reason::UnexpectedToken)),
            (r"\nabla_{c}^{2}", "^", Some(Reason::UnexpectedToken)),
            (
                r"\nabla_\comovfield",
                r"\comovfield",
                Some(Reason::UnexpectedToken),
            ),
        ] {
            let (range, fault) = ParsedAssemblyMath::parse_corpus(&catalogue, source).unwrap_err();
            assert_eq!(&source[range], token, "{source}");
            match (fault, reason) {
                (
                    Fault::MathSyntax {
                        reason: actual,
                        token: actual_token,
                        ..
                    },
                    Some(expected),
                ) => {
                    assert_eq!(actual, expected, "{source}");
                    assert_eq!(actual_token, token, "{source}");
                }
                (Fault::UnknownMathCommand { command }, None) => assert_eq!(command, token),
                (actual, expected) => panic!("{source}: {actual:?} did not match {expected:?}"),
            }
        }
    }

    #[test]
    fn source_names_parse_with_located_refusals() {
        let graph = seed().unwrap();
        let book = addressing(&graph).unwrap();
        let catalogue = notation(&book).unwrap();
        for id in [
            DefinitionId::Comovfield,
            DefinitionId::Readout,
            DefinitionId::Willvirtue,
            DefinitionId::Gensym,
        ] {
            let source = format!("\\{}", id.token());
            let parsed = parse_assembly_math(&catalogue, &source).unwrap();
            assert_eq!(parsed.commands()[0].bytes, 0..source.len());
            check_expression(&catalogue, parsed.into_draft()).unwrap();
        }
        let (bytes, fault) = parse_assembly_math(&catalogue, r"\unknown").unwrap_err();
        assert_eq!(bytes, 0..8);
        assert!(matches!(fault, Fault::UnknownMathCommand { .. }));
        let (bytes, fault) = parse_assembly_math(&catalogue, r"\gensym{x}").unwrap_err();
        assert_eq!(bytes, 7..8);
        assert!(matches!(fault, Fault::MathArguments { expected: 0, .. }));
    }
}

#[cfg(test)]
mod text_tests {
    use super::*;
    use crate::{
        NotationFault, NotationResource, addressing, check_document_expression, check_expression,
        notation, seed,
    };

    #[test]
    #[allow(clippy::too_many_lines)] // One fixture checks each text-grammar refusal at its range.
    fn math_text_is_decoded_not_tex() {
        let graph = seed().unwrap();
        let book = addressing(&graph).unwrap();
        let catalogue = notation(&book).unwrap();
        let source = r"\underbrace{\dot{\sigma}}_{\text{change in overlap}}";
        let parsed = parse_assembly_display_math(&catalogue, source).unwrap();
        let ExprDraft::Call { arguments, .. } = parsed.draft() else {
            panic!("underbrace root");
        };
        let ExprDraft::Text { runs } = &arguments[1] else {
            panic!("text annotation");
        };
        assert_eq!(runs[0].text, "change in overlap");
        assert_eq!(&source[runs[0].source.clone()], "change in overlap");
        assert_eq!(runs[0].decoded, 0..17);
        assert!(parsed.ranges().iter().any(|r| r.path == [1] && &source[r.bytes.clone()] == r"{\text{change in overlap}}"));
        for (source, reason, fragment) in [
            (r"\text{a@tag{x}}", Reason::InvalidTextCharacter, "@"),
            (r"\text{a\alpha}", Reason::InvalidTextCharacter, "\\"),
            ("\\text{a\nb}", Reason::InvalidTextCharacter, "\n"),
            (r"\text{a{b}", Reason::InvalidTextCharacter, "{"),
            (r"\text{a--b}", Reason::InvalidTextCharacter, "-"),
            (r"\text{a,,b}", Reason::InvalidTextCharacter, ","),
            (r"\text{a  b}", Reason::InvalidTextCharacter, " "),
            (r"\text{}", Reason::EmptyText, "}"),
            (r"\text{ }", Reason::EmptyText, " "),
            (r"\text{abc", Reason::UnclosedText, r"\text"),
        ] {
            let (range, fault) = parse_assembly_display_math(&catalogue, source).unwrap_err();
            assert!(source[range].contains(fragment), "{source:?}: {fault:?}");
            assert!(matches!(fault, Fault::MathSyntax { reason: actual, .. } if actual == reason));
        }
        for (source, pair) in [
            (r"\text{a--b}", "--"),
            (r"\text{a,,b}", ",,"),
            (r"\text{a  b}", "  "),
        ] {
            let (range, _) = parse_assembly_display_math(&catalogue, source).unwrap_err();
            let second = source.find(pair).unwrap() + 1;
            assert_eq!(range, second..second + 1);
        }
        for source in [r"\text{a}", r"\underbrace{x}_{\text{a}}"] {
            let (range, fault) = parse_assembly_math(&catalogue, source).unwrap_err();
            assert_eq!(
                &source[range],
                if source.starts_with(r"\text") {
                    r"\text{a}"
                } else {
                    r"\underbrace"
                }
            );
            assert!(matches!(
                fault,
                Fault::MathSyntax {
                    reason: Reason::UnexpectedToken,
                    ..
                }
            ));
        }
        let (range, fault) = parse_assembly_display_math(&catalogue, r"\text{a}}").unwrap_err();
        assert_eq!(range, 8..9);
        assert!(matches!(
            fault,
            Fault::MathSyntax {
                reason: Reason::UnexpectedToken,
                ..
            }
        ));
        for forbidden in [
            "$", "`", "<", ">", "&", "|", "_", "^", "~", "*", ":", "1", "'", ".", "\t",
        ] {
            let source = format!("\\text{{a{forbidden}b}}");
            let (range, fault) = parse_assembly_display_math(&catalogue, &source).unwrap_err();
            let start = source.find(forbidden).unwrap();
            assert_eq!(range, start..start + forbidden.len(), "{source}");
            assert!(matches!(
                fault,
                Fault::MathSyntax {
                    reason: Reason::InvalidTextCharacter,
                    ..
                }
            ));
        }
        let (range, fault) = parse_assembly_display_math(&catalogue, "x=y").unwrap_err();
        assert_eq!(range, 1..2);
        assert!(matches!(
            fault,
            Fault::MathSyntax {
                reason: Reason::UnexpectedToken,
                ..
            }
        ));
        let (range, fault) = parse_assembly_display_math(&catalogue, r"x\;=\;y+z").unwrap_err();
        assert_eq!(range, 7..8);
        assert!(matches!(
            fault,
            Fault::MathSyntax {
                reason: Reason::UnexpectedToken,
                ..
            }
        ));
        let (range, fault) =
            parse_assembly_display_math(&catalogue, r"\underbrace{x}_{\text{pending}}")
                .unwrap_err();
        let start = r"\underbrace{x}_{\text{pending}}".find("pending").unwrap();
        assert_eq!(range, start..start + 7);
        assert!(matches!(
            fault,
            Fault::MathSyntax {
                reason: Reason::PendingAnnotation,
                ..
            }
        ));
        let parsed = parse_assembly_display_math(&catalogue, source).unwrap();
        let standalone = check_expression(&catalogue, parsed.draft().clone()).unwrap_err();
        assert!(
            standalone
                .diagnostics()
                .iter()
                .any(|d| matches!(d.fault, NotationFault::TextOutsideDocument))
        );
        check_document_expression(&catalogue, parsed.into_draft()).unwrap();
        let merged = check_document_expression(
            &catalogue,
            ExprDraft::Text {
                runs: vec![
                    TextRun {
                        text: "change ".into(),
                        source: 0..7,
                        decoded: 0..7,
                    },
                    TextRun {
                        text: "in overlap".into(),
                        source: 7..17,
                        decoded: 7..17,
                    },
                ],
            },
        )
        .unwrap();
        let ExprDraft::Text { runs } = merged.draft() else {
            panic!("merged text");
        };
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].text, "change in overlap");
        assert_eq!(runs[0].source, 0..17);
        let too_long = "a".repeat(257);
        let at_limit = "a".repeat(256);
        let accepted =
            parse_assembly_display_math(&catalogue, &format!("\\text{{{at_limit}}}")).unwrap();
        check_document_expression(&catalogue, accepted.into_draft()).unwrap();
        let (range, fault) =
            parse_assembly_display_math(&catalogue, &format!("\\text{{{too_long}}}")).unwrap_err();
        assert_eq!(range, 262..263);
        assert!(matches!(
            fault,
            Fault::Limit {
                resource: Resource::MathTextBytes,
                limit: 256,
                ..
            }
        ));
        let invalid = ExprDraft::Text {
            runs: vec![TextRun {
                text: too_long,
                source: 0..257,
                decoded: 0..257,
            }],
        };
        let error = check_document_expression(&catalogue, invalid).unwrap_err();
        assert!(error.diagnostics().iter().any(|d| matches!(
            d.fault,
            NotationFault::NotationLimit {
                resource: NotationResource::TextBytes,
                limit: 256,
                ..
            }
        )));
        let pending = ExprDraft::Call {
            callee: Callee::Primitive(PrimitiveId::Underbrace),
            arguments: vec![
                ExprDraft::Literal { value: "x".into() },
                ExprDraft::Text {
                    runs: vec![TextRun {
                        text: " pending ".into(),
                        source: 0..9,
                        decoded: 0..9,
                    }],
                },
            ],
        };
        let error = check_document_expression(&catalogue, pending).unwrap_err();
        assert!(
            error
                .diagnostics()
                .iter()
                .any(|d| matches!(d.fault, NotationFault::PendingAnnotation))
        );
    }
}
