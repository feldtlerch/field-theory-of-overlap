use super::{
    AssemblyInputFault as Fault, AssemblyResource as Resource, MathSyntaxReason as Reason,
};
use crate::{
    Callee, CheckedNotation, DefinitionId, DefinitionKind, ExprDraft, PrimitiveId, TextRun,
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
/// Parsed syntax still requires the existing notation expression checker.
#[derive(Debug)]
pub struct ParsedAssemblyMath {
    draft: ExprDraft,
    commands: Vec<MathCommandUse>,
    ranges: Vec<MathSourceRange>,
}
impl ParsedAssemblyMath {
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
struct Parser<'a, 'b, 'g> {
    catalogue: &'a CheckedNotation<'b, 'g>,
    tokens: Vec<Token>,
    at: usize,
    eof: usize,
    depth: usize,
    nodes: usize,
    commands: Vec<MathCommandUse>,
    arena: Vec<(Range<usize>, Vec<usize>)>,
    display: bool,
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
        if self.display
            && self.is("\\;")
            && self.tokens.get(self.at + 1).is_some_and(|t| t.text == "=")
        {
            self.take();
        } else if self.display && self.is("=") {
            return Err(self.syntax(Reason::UnexpectedToken, &["\\;"]));
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
        self.catalogue
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
                self.take();
                id
            } else if self.atom_start() {
                PrimitiveId::Juxtapose
            } else {
                break;
            };
            let right = self.unary()?;
            let bytes = left.bytes.start..right.bytes.end;
            left = self.primitive(id, vec![left, right], bytes)?;
        }
        Ok(left)
    }
    fn unary(&mut self) -> Result<Node> {
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
        let primitive = self.catalogue.primitives().iter().find(|p| {
            matches!(
                p.id(),
                PrimitiveId::Alpha
                    | PrimitiveId::Beta
                    | PrimitiveId::Fraction
                    | PrimitiveId::Omega
                    | PrimitiveId::Sigma
                    | PrimitiveId::Gamma
                    | PrimitiveId::Dot
            ) && (p.operation().host_tokens().contains(&token.text.as_str())
                || p.operation().unicode_scalar() == Some(token.text.as_str()))
        });
        let definition = self
            .catalogue
            .definitions()
            .iter()
            .find(|d| d.command() == token.text);
        let signature = primitive
            .map(|p| (Callee::Primitive(p.id()), p.signature().arity))
            .or_else(|| {
                definition.map(|d| {
                    (
                        Callee::Definition(d.spec().id()),
                        d.spec().signature().arity,
                    )
                })
            });
        if let Some((callee, arity)) = signature {
            return self.command(token, callee, arity);
        }
        self.literal(token)
    }
    fn command(&mut self, token: Token, callee: Callee, arity: u8) -> Result<Node> {
        self.take();
        if let Callee::Definition(id) = callee
            && matches!(
                self.catalogue.definition(id).spec().kind(),
                DefinitionKind::Inherited { .. }
            )
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
            .catalogue
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
    parse_inner(catalogue, source, false).map_err(|failure| *failure)
}
/// Parse a mathematical display with its explicit thick-space boundaries.
/// # Errors
/// Returns the first decisive decoded range and complete parser fault.
pub fn parse_assembly_display_math(
    catalogue: &CheckedNotation<'_, '_>,
    source: &str,
) -> std::result::Result<ParsedAssemblyMath, (Range<usize>, Fault)> {
    parse_inner(catalogue, source, true).map_err(|failure| *failure)
}
/// Parse the complete authored entry without evaluating any TeX.
/// # Errors
/// Returns the first decisive decoded range and complete parser fault.
fn parse_inner(
    catalogue: &CheckedNotation<'_, '_>,
    source: &str,
    display: bool,
) -> Result<ParsedAssemblyMath> {
    let mut parser = Parser {
        catalogue,
        tokens: lex(source)?,
        at: 0,
        eof: source.len(),
        depth: 0,
        nodes: 0,
        commands: Vec::new(),
        arena: Vec::new(),
        display,
    };
    let root = parser.equality()?;
    if parser.at != parser.tokens.len() {
        return Err(parser.syntax(Reason::UnexpectedToken, &["end of expression"]));
    }
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
    })
}

#[cfg(test)]
mod inherited_tests {
    use super::*;
    use crate::{addressing, check_expression, notation, seed};

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
