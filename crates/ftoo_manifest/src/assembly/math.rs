use super::{
    AssemblyInputFault as Fault, AssemblyResource as Resource, MathSyntaxReason as Reason,
};
use crate::{Callee, CheckedNotation, DefinitionId, ExprDraft, PrimitiveId};
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
        if tokens.len() == 16_384 {
            return Err(limit(start..end, Resource::MathTokens, 16_384));
        }
        tokens.push(Token {
            text: source[start..end].into(),
            bytes: start..end,
        });
    }
    Ok(tokens)
}
impl Parser<'_, '_, '_> {
    fn token(&self) -> Token {
        self.tokens.get(self.at).cloned().unwrap_or(Token {
            text: String::new(),
            bytes: self.eof..self.eof,
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
        if !self.is("=") {
            return Ok(left);
        }
        self.take();
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
        while self.is("+") || self.minus() {
            let negate = self.minus();
            let sign = self.take();
            let mut right = self.product()?;
            if negate {
                let bytes = sign.bytes.start..right.bytes.end;
                right = self.primitive(PrimitiveId::Negate, vec![right], bytes)?;
            }
            let bytes = left.bytes.start..right.bytes.end;
            left = self.primitive(PrimitiveId::Add, vec![left, right], bytes)?;
        }
        Ok(left)
    }
    fn infix(&self) -> Option<PrimitiveId> {
        let text = self.token().text;
        self.catalogue
            .primitives()
            .iter()
            .find(|p| {
                matches!(p.id(), PrimitiveId::Multiply | PrimitiveId::Juxtapose)
                    && (p.operation().host_tokens().contains(&text.as_str())
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
    fn atom(&mut self) -> Result<Node> {
        let token = self.token();
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
        let primitive = self.catalogue.primitives().iter().find(|p| {
            matches!(
                p.id(),
                PrimitiveId::Alpha | PrimitiveId::Beta | PrimitiveId::Fraction
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
    parse_inner(catalogue, source).map_err(|failure| *failure)
}
/// Parse the complete authored entry without evaluating any TeX.
/// # Errors
/// Returns the first decisive decoded range and complete parser fault.
fn parse_inner(catalogue: &CheckedNotation<'_, '_>, source: &str) -> Result<ParsedAssemblyMath> {
    let mut parser = Parser {
        catalogue,
        tokens: lex(source)?,
        at: 0,
        eof: source.len(),
        depth: 0,
        nodes: 0,
        commands: Vec::new(),
        arena: Vec::new(),
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
