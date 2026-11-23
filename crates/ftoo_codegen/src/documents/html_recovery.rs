use super::{
    diagnostics::{AssemblyFault, DocumentError},
    failure,
    html::{Fonts, MathRun, Node},
    plan::{DocumentPlan, Inline, StyleKind},
};
use ftoo_manifest::{AssemblySite, AssemblyStage, RuleId};
use std::collections::BTreeSet;
use std::ops::Range;

const MAX_BYTES: usize = 16_777_216;
const MAX_NODES: usize = 100_000;
const MAX_DEPTH: usize = 64;

struct Parser<'a> {
    source: &'a str,
    at: usize,
    nodes: usize,
    library: bool,
}
impl Parser<'_> {
    fn consume(&mut self, expected: &str) -> bool {
        if self.source[self.at..].starts_with(expected) {
            self.at += expected.len();
            true
        } else {
            false
        }
    }

    fn token(&mut self, allowed: impl Fn(char) -> bool) -> Option<String> {
        let start = self.at;
        while let Some(character) = self.source[self.at..].chars().next() {
            if !allowed(character) {
                break;
            }
            self.at += character.len_utf8();
        }
        (self.at > start).then(|| self.source[start..self.at].into())
    }

    fn decode(&mut self, until: char) -> Option<String> {
        let mut result = String::new();
        loop {
            let character = self.source[self.at..].chars().next()?;
            if character == until {
                break;
            }
            if character == '&' {
                let rest = &self.source[self.at..];
                let (entity, value) = [
                    ("&amp;", '&'),
                    ("&lt;", '<'),
                    ("&gt;", '>'),
                    ("&quot;", '"'),
                    ("&#39;", '\''),
                    ("&#x27;", '\''),
                ]
                .into_iter()
                .find(|(entity, _)| {
                    rest.starts_with(entity) && (self.library || *entity != "&#x27;")
                })?;
                result.push(value);
                self.at += entity.len();
            } else {
                if matches!(character, '>' | '"' | '\'') {
                    return None;
                }
                result.push(character);
                self.at += character.len_utf8();
            }
        }
        Some(result)
    }

    fn element(&mut self, depth: usize) -> Option<Node> {
        if depth > MAX_DEPTH || self.nodes >= MAX_NODES || !self.consume("<") {
            return None;
        }
        self.nodes += 1;
        let name = self.token(|c| c.is_ascii_lowercase() || c.is_ascii_digit())?;
        let mut attributes = Vec::new();
        while self.consume(" ") {
            if self.library && self.source[self.at..].starts_with("/>") {
                break;
            }
            let key = self.token(|c| c.is_ascii_lowercase() || c == '-')?;
            if self.library {
                self.consume(" ");
            }
            if !self.consume("=\"") || attributes.iter().any(|(prior, _)| prior == &key) {
                return None;
            }
            let value = self.decode('"')?;
            if !self.consume("\"") {
                return None;
            }
            attributes.push((key, value));
        }
        let empty = self.library && self.consume("/>");
        if !empty && !self.consume(">") {
            return None;
        }
        let mut children = Vec::new();
        if empty {
        } else if name == "style" {
            let end = self.source[self.at..].find("</style>")? + self.at;
            let raw = &self.source[self.at..end];
            if raw.contains('<') || raw.contains('&') {
                return None;
            }
            children.push(Node::Text(raw.into()));
            self.at = end + "</style>".len();
        } else if name != "meta" {
            loop {
                if self.consume(&format!("</{name}>")) {
                    break;
                }
                if self.source[self.at..].starts_with('<') {
                    children.push(self.element(depth + 1)?);
                } else {
                    if self.nodes >= MAX_NODES {
                        return None;
                    }
                    self.nodes += 1;
                    children.push(Node::Text(self.decode('<')?));
                }
            }
        }
        Some(Node::Element {
            name,
            attributes,
            children,
        })
    }
}

pub(super) fn library_fragment(source: &str) -> Option<Node> {
    if source.len() > 262_144 {
        return None;
    }
    let mut parser = Parser {
        source,
        at: 0,
        nodes: 0,
        library: true,
    };
    let root = parser.element(0)?;
    (parser.at == source.len()).then_some(root)
}

// The admitted coefficient profile is recorded at reg:documents:html-mathematics-observations.
pub(super) fn math_profile(fragment: &Node, expanded: &str) -> Option<Node> {
    fn node<'a>(value: &'a Node, name: &str, attrs: &[(&str, &str)]) -> Option<&'a [Node]> {
        let Node::Element {
            name: actual,
            attributes,
            children,
        } = value
        else {
            return None;
        };
        (actual == name
            && attributes
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .eq(attrs.iter().copied()))
        .then_some(children)
    }
    fn e(name: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
        Node::Element {
            name: name.into(),
            attributes: attrs
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            children,
        }
    }
    fn leaf(name: &str, attrs: &[(&str, &str)], value: &str) -> Node {
        e(name, attrs, vec![Node::Text(value.into())])
    }
    let sub = || {
        e(
            "msub",
            &[],
            vec![leaf("mi", &[], "c"), leaf("mn", &[], "1")],
        )
    };
    let expected = match expanded {
        "({\\underbrace{{c}_{1}}_{\\text{pending}}}={\\frac{1}{3}})" => e(
            "mrow",
            &[],
            vec![
                leaf("mo", &[("stretchy", "false")], "("),
                e(
                    "munder",
                    &[],
                    vec![
                        e(
                            "munder",
                            &[],
                            vec![sub(), leaf("mo", &[("stretchy", "true")], "⏟")],
                        ),
                        leaf("mtext", &[], "pending"),
                    ],
                ),
                leaf("mo", &[], "="),
                e(
                    "mfrac",
                    &[],
                    vec![leaf("mn", &[], "1"), leaf("mn", &[], "3")],
                ),
                leaf("mo", &[("stretchy", "false")], ")"),
            ],
        ),
        "{c}_{1}" => e("mrow", &[], vec![sub()]),
        _ => return None,
    };
    let span = node(fragment, "span", &[("class", "katex")])?;
    let [math] = span else { return None };
    let semantics = node(
        math,
        "math",
        &[("xmlns", "http://www.w3.org/1998/Math/MathML")],
    )?;
    let [semantics] = semantics else { return None };
    let children = node(semantics, "semantics", &[])?;
    let [visible, annotation] = children else {
        return None;
    };
    if visible != &expected {
        return None;
    }
    let text = node(
        annotation,
        "annotation",
        &[("encoding", "application/x-tex")],
    )?;
    if !matches!(text, [Node::Text(value)] if value == expanded) {
        return None;
    }
    Some(math.clone())
}

fn mismatch(plan: &DocumentPlan, aspect: &str) -> DocumentError {
    failure(
        RuleId::M055,
        AssemblyStage::Emission,
        AssemblySite::Source(plan.coverage.site.clone()),
        AssemblyFault::UnknownToken {
            value: aspect.into(),
            expected: vec!["checked HTML plan realization".into()],
        },
    )
}

fn public_text(plan: &DocumentPlan, known: &[&str], value: &str) -> Result<(), DocumentError> {
    if !ftoo_manifest::reader_label_literals(value).is_empty()
        || known.iter().any(|label| value.contains(label))
    {
        return Err(mismatch(plan, "HTML reader label literal"));
    }
    Ok(())
}

fn public_run_text(
    plan: &DocumentPlan,
    known: &[&str],
    value: &str,
    paper: &[Range<usize>],
) -> Result<(), DocumentError> {
    let label_character = |c: char| c.is_alphanumeric() || matches!(c, ':' | '-' | '_');
    if ftoo_manifest::reader_label_literals(value)
        .iter()
        .any(|range| !paper.contains(range))
        || known.iter().any(|label| {
            value
                .match_indices(label)
                .any(|(start, _)| !paper.contains(&(start..start + label.len())))
        })
        || paper.iter().any(|range| {
            value[..range.start]
                .chars()
                .next_back()
                .is_some_and(label_character)
                || {
                    let after = &value[range.end..];
                    after.chars().next().is_some_and(label_character) && !after.starts_with(": ")
                }
        })
    {
        return Err(mismatch(plan, "HTML reader label literal"));
    }
    Ok(())
}

fn element<'a>(
    plan: &DocumentPlan,
    known: &[&str],
    node: &'a Node,
    name: &str,
    attrs: &[(&str, &str)],
    aspect: &str,
) -> Result<&'a [Node], DocumentError> {
    let Node::Element {
        name: actual_name,
        attributes,
        children,
    } = node
    else {
        return Err(mismatch(plan, aspect));
    };
    for (_, value) in attributes {
        public_text(plan, known, value)?;
    }
    if actual_name != name
        || attributes.len() != attrs.len()
        || attributes
            .iter()
            .zip(attrs)
            .any(|((key, value), (expected_key, expected_value))| {
                key != expected_key || value != expected_value
            })
    {
        return Err(mismatch(plan, aspect));
    }
    Ok(children)
}

fn text(
    plan: &DocumentPlan,
    known: &[&str],
    nodes: &[Node],
    expected: &str,
    aspect: &str,
) -> Result<(), DocumentError> {
    let [Node::Text(actual)] = nodes else {
        return Err(mismatch(plan, aspect));
    };
    public_text(plan, known, actual)?;
    if actual != expected {
        return Err(mismatch(plan, aspect));
    }
    Ok(())
}

fn flat(runs: &[Inline]) -> String {
    let mut result = String::new();
    for run in runs {
        match run {
            Inline::Text(value) | Inline::Mention { text: value, .. } => result.push_str(value),
            Inline::Tag { handle, .. } => result.push_str(handle),
            Inline::PaperTag { value, .. } => result.push_str(&value.spelling),
            Inline::Citation { identity, .. } => result.push_str(&identity.handle),
            Inline::Styled { children, .. } => result.push_str(&flat(children)),
            Inline::Math { .. } | Inline::BibliographyCitation { .. } => {
                unreachable!("unrealizable heading refused before recovery")
            }
        }
    }
    result
}

fn bibliography_citation(
    plan: &DocumentPlan,
    known: &[&str],
    node: &Node,
    kind: ftoo_manifest::BibliographyCitationKind,
    entries: &[super::bibliography::Entry],
    label: &str,
) -> Result<(), DocumentError> {
    let parenthetical = kind == ftoo_manifest::BibliographyCitationKind::Parenthetical;
    let children = element(
        plan,
        known,
        node,
        "span",
        &[(
            "data-kind",
            if parenthetical {
                "parenthetical"
            } else {
                "textual"
            },
        )],
        "HTML bibliography citation kind",
    )?;
    let expected_nodes = if entries.is_empty() {
        usize::from(parenthetical)
    } else {
        2 * entries.len() - 1 + 2 * usize::from(parenthetical)
    };
    if children.len() != expected_nodes {
        return Err(mismatch(plan, "HTML bibliography citation link inventory"));
    }
    if label != super::bibliography::citation_label(kind, entries) {
        return Err(mismatch(plan, "HTML bibliography citation planned label"));
    }
    let mut at = 0;
    if parenthetical {
        let node = children
            .get(at)
            .ok_or_else(|| mismatch(plan, "HTML bibliography citation link inventory"))?;
        text(
            plan,
            known,
            std::slice::from_ref(node),
            "(",
            "HTML bibliography citation punctuation",
        )?;
        at += 1;
    }
    for (index, entry) in entries.iter().enumerate() {
        if index != 0 {
            let node = children
                .get(at)
                .ok_or_else(|| mismatch(plan, "HTML bibliography citation link inventory"))?;
            text(
                plan,
                known,
                std::slice::from_ref(node),
                "; ",
                "HTML bibliography citation punctuation",
            )?;
            at += 1;
        }
        let node = children
            .get(at)
            .ok_or_else(|| mismatch(plan, "HTML bibliography citation link inventory"))?;
        let href = format!("#{}", entry.anchor);
        let link = element(
            plan,
            known,
            node,
            "a",
            &[("href", &href)],
            "HTML bibliography citation destination",
        )?;
        text(
            plan,
            known,
            link,
            &super::bibliography::screen_citation_piece(kind, entry),
            "HTML bibliography citation label",
        )?;
        at += 1;
    }
    if parenthetical {
        let node = children
            .get(at)
            .ok_or_else(|| mismatch(plan, "HTML bibliography citation link inventory"))?;
        text(
            plan,
            known,
            std::slice::from_ref(node),
            ")",
            "HTML bibliography citation punctuation",
        )?;
        at += 1;
    }
    if at != children.len() {
        return Err(mismatch(plan, "HTML bibliography citation link inventory"));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Each inline kind has its own recovery comparison.
fn runs(
    plan: &DocumentPlan,
    known: &[&str],
    source: &[Inline],
    actual: &[Node],
    inside_style: bool,
    block: usize,
    prepared: &[MathRun],
) -> Result<(), DocumentError> {
    let mut at = 0;
    let mut math_at = 0;
    let mut pending = String::new();
    let mut paper = Vec::new();
    let flush = |pending: &mut String,
                 paper: &mut Vec<Range<usize>>,
                 at: &mut usize|
     -> Result<(), DocumentError> {
        if !pending.is_empty() {
            let Some(Node::Text(value)) = actual.get(*at) else {
                return Err(mismatch(
                    plan,
                    if inside_style {
                        "HTML style boundary"
                    } else {
                        "HTML inline text or identity"
                    },
                ));
            };
            public_run_text(plan, known, value, paper)?;
            if value != pending {
                return Err(mismatch(
                    plan,
                    if inside_style {
                        "HTML style boundary"
                    } else {
                        "HTML inline text or identity"
                    },
                ));
            }
            pending.clear();
            paper.clear();
            *at += 1;
        }
        Ok(())
    };
    for (run_index, run) in source.iter().enumerate() {
        match run {
            Inline::Text(value) => {
                pending.push_str(value);
            }
            Inline::Tag { handle, .. } => {
                pending.push_str(handle);
            }
            Inline::PaperTag { value, .. } => {
                let start = pending.len();
                pending.push_str(&value.spelling);
                if value.key == value.spelling {
                    paper.push(start..pending.len());
                }
            }
            Inline::Citation { identity, .. } => {
                pending.push_str(&identity.handle);
            }
            Inline::Mention { text: value, .. } => {
                flush(&mut pending, &mut paper, &mut at)?;
                let node = actual
                    .get(at)
                    .ok_or_else(|| mismatch(plan, "HTML mention boundary"))?;
                let children = element(plan, known, node, "code", &[], "HTML mention boundary")?;
                text(plan, known, children, value, "HTML mention text")?;
                at += 1;
            }
            Inline::Styled { kind, children, .. } => {
                flush(&mut pending, &mut paper, &mut at)?;
                let node = actual
                    .get(at)
                    .ok_or_else(|| mismatch(plan, "HTML style boundary"))?;
                let (name, attrs): (&str, &[(&str, &str)]) = match kind {
                    StyleKind::Emphasis => ("em", &[]),
                    StyleKind::Strong => ("strong", &[]),
                    StyleKind::Smallcaps => ("span", &[("style", "font-variant:small-caps")]),
                    StyleKind::Regular => {
                        ("span", &[("style", "font-weight:normal;font-style:normal")])
                    }
                };
                let actual_children =
                    element(plan, known, node, name, attrs, "HTML style boundary")?;
                runs(plan, known, children, actual_children, true, block, &[])?;
                at += 1;
            }
            Inline::Math { markdown, site, .. } => {
                flush(&mut pending, &mut paper, &mut at)?;
                let expected = prepared
                    .iter()
                    .filter(|item| item.block == block)
                    .nth(math_at)
                    .ok_or_else(|| math_mismatch(site, "HTML mathematical occurrence"))?;
                math_at += 1;
                if expected.run != run_index || expected.source != *markdown {
                    return Err(math_mismatch(site, "HTML mathematical source occurrence"));
                }
                let node = actual
                    .get(at)
                    .ok_or_else(|| math_mismatch(site, "HTML mathematical inventory"))?;
                math_text(plan, known, site, node)?;
                let annotation = math_annotation(node)
                    .ok_or_else(|| math_mismatch(site, "HTML mathematical annotation"))?;
                if annotation != markdown {
                    return Err(math_mismatch(site, "HTML mathematical annotation"));
                }
                if node != &expected.node {
                    return Err(math_mismatch(site, "HTML mathematical presentation"));
                }
                at += 1;
            }
            Inline::BibliographyCitation {
                kind,
                entries,
                label,
                ..
            } => {
                flush(&mut pending, &mut paper, &mut at)?;
                let node = actual
                    .get(at)
                    .ok_or_else(|| mismatch(plan, "HTML bibliography citation position"))?;
                bibliography_citation(plan, known, node, *kind, entries, label)?;
                at += 1;
            }
        }
    }
    flush(&mut pending, &mut paper, &mut at)?;
    if at != actual.len() {
        return Err(mismatch(plan, "HTML inline boundary"));
    }
    Ok(())
}

fn math_mismatch(site: &ftoo_manifest::UnitSite, aspect: &str) -> DocumentError {
    failure(
        RuleId::M055,
        AssemblyStage::Emission,
        AssemblySite::Source(site.clone()),
        AssemblyFault::UnknownToken {
            value: aspect.into(),
            expected: vec!["checked mathematical occurrence".into()],
        },
    )
}
fn math_text(
    plan: &DocumentPlan,
    known: &[&str],
    site: &ftoo_manifest::UnitSite,
    node: &Node,
) -> Result<(), DocumentError> {
    match node {
        Node::Text(value) => public_text(plan, known, value)
            .map_err(|_| math_mismatch(site, "HTML reader label literal")),
        Node::Element {
            attributes,
            children,
            ..
        } => {
            for (_, value) in attributes {
                public_text(plan, known, value)
                    .map_err(|_| math_mismatch(site, "HTML reader label literal"))?;
            }
            for child in children {
                math_text(plan, known, site, child)?;
            }
            Ok(())
        }
    }
}
fn math_annotation(node: &Node) -> Option<&str> {
    let Node::Element { children: math, .. } = node else {
        return None;
    };
    let [
        Node::Element {
            children: semantics,
            ..
        },
    ] = math.as_slice()
    else {
        return None;
    };
    let [
        _,
        Node::Element {
            children: annotation,
            ..
        },
    ] = semantics.as_slice()
    else {
        return None;
    };
    let [Node::Text(value)] = annotation.as_slice() else {
        return None;
    };
    Some(value)
}

fn base64_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn decode_font(source: &str) -> Option<Vec<u8>> {
    let bytes = source.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let mut output = Vec::with_capacity(bytes.len() / 4 * 3);
    for (index, group) in bytes.as_chunks::<4>().0.iter().enumerate() {
        let final_group = index + 1 == bytes.len() / 4;
        let padding = group.iter().rev().take_while(|&&c| c == b'=').count();
        if padding > 2 || (!final_group && padding > 0) {
            return None;
        }
        let mut value = 0_u32;
        for &c in group {
            value = (value << 6) | u32::from(if c == b'=' { 0 } else { base64_value(c)? });
        }
        output.push(u8::try_from((value >> 16) & 255).ok()?);
        if padding < 2 {
            output.push(u8::try_from((value >> 8) & 255).ok()?);
        }
        if padding == 0 {
            output.push(u8::try_from(value & 255).ok()?);
        }
    }
    Some(output)
}

fn stylesheet(plan: &DocumentPlan, fonts: &Fonts, nodes: &[Node]) -> Result<(), DocumentError> {
    let [Node::Text(raw)] = nodes else {
        return Err(mismatch(plan, "HTML stylesheet raw text"));
    };
    let mut lines = raw.lines();
    for (face, bytes) in &fonts.0 {
        let line = lines
            .next()
            .ok_or_else(|| mismatch(plan, "HTML font inventory"))?;
        let prefix = format!(
            "@font-face{{font-family:'{}';src:url(data:font/otf;base64,",
            face.family
        );
        let suffix = format!(
            ");font-weight:{};font-style:{}}}",
            face.weight,
            if face.italic { "italic" } else { "normal" }
        );
        let encoded = line
            .strip_prefix(&prefix)
            .and_then(|tail| tail.strip_suffix(&suffix))
            .ok_or_else(|| mismatch(plan, "HTML font family or face declaration"))?;
        let decoded = decode_font(encoded).ok_or_else(|| mismatch(plan, "HTML font data"))?;
        if &decoded != bytes {
            return Err(mismatch(plan, "HTML font byte identity"));
        }
    }
    let layout = "body{font-family:'Pagella',serif;max-width:46rem;margin:auto;padding:0 1rem;line-height:1.5}h1{font-weight:700}a:focus-visible,summary:focus-visible{outline:3px solid #a34500;outline-offset:3px}.refrain{text-align:center;font-style:italic}.attribution{text-align:right;font-size:smaller}.gap{color:#444}.licenses pre{white-space:pre-wrap;overflow-wrap:anywhere;font-family:'Pagella',serif}";
    if lines.next() != Some(layout) {
        return Err(mismatch(plan, "HTML stylesheet layout and fallback"));
    }
    let mono = fonts.0.iter().any(|(face, _)| face.family == "Inconsolata");
    if mono
        && lines.next()
            != Some("code{font-family:'Inconsolata',monospace;font-weight:400;font-style:normal}")
    {
        return Err(mismatch(plan, "HTML monospace face"));
    }
    let math = fonts.0.iter().any(|(face, _)| face.is_math());
    if math
        && lines.next()
            != Some(
                "math{font-family:'Pagella Math';font-weight:400;font-style:normal}math mtext{font-family:'Pagella',serif}",
            )
    {
        return Err(mismatch(plan, "HTML mathematical face"));
    }
    if lines.next().is_some() {
        return Err(mismatch(plan, "HTML stylesheet inventory"));
    }
    Ok(())
}

fn check_faces(plan: &DocumentPlan, fonts: &Fonts) -> Result<(), DocumentError> {
    fn visit(runs: &[Inline], bold: bool, italic: bool, used: &mut BTreeSet<&'static str>) {
        for run in runs {
            match run {
                Inline::Styled { kind, children, .. } => {
                    let (bold, italic) = match kind {
                        StyleKind::Emphasis => (bold, true),
                        StyleKind::Strong => (true, italic),
                        StyleKind::Smallcaps => (bold, italic),
                        StyleKind::Regular => (false, false),
                    };
                    used.insert(match (bold, italic) {
                        (false, false) => "texgyrepagella-regular.otf",
                        (false, true) => "texgyrepagella-italic.otf",
                        (true, false) => "texgyrepagella-bold.otf",
                        (true, true) => "texgyrepagella-bolditalic.otf",
                    });
                    visit(children, bold, italic, used);
                }
                Inline::Mention { .. } => {
                    used.insert("Inconsolatazi4-Regular.otf");
                }
                Inline::Math { .. } => {
                    used.insert("texgyrepagella-math.otf");
                }
                _ => {}
            }
        }
    }
    let mut used = BTreeSet::from(["texgyrepagella-regular.otf", "texgyrepagella-bold.otf"]);
    if plan.screen.as_ref().is_some_and(|screen| {
        plan.blocks.iter().enumerate().any(|(index, block)| {
            !block.paragraph && block.depth.is_none() && !screen.html_gaps.contains_key(&index)
        })
    }) {
        used.insert("texgyrepagella-italic.otf");
    }
    visit(&plan.coverage.runs, false, false, &mut used);
    for runs in &plan.coverage.audit {
        visit(runs, false, false, &mut used);
    }
    for (index, block) in plan.blocks.iter().enumerate() {
        if plan
            .screen
            .as_ref()
            .is_some_and(|screen| screen.html_gaps.contains_key(&index))
        {
            continue;
        }
        visit(
            &block.runs,
            block.depth.is_some(),
            !block.paragraph && block.depth.is_none(),
            &mut used,
        );
    }
    let actual: BTreeSet<_> = fonts.0.iter().map(|(face, _)| face.file).collect();
    if used != actual {
        return Err(mismatch(plan, "HTML font inventory"));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Each block and gap retains a separate plan comparison.
fn check_main(
    plan: &DocumentPlan,
    known: &[&str],
    prepared: &[MathRun],
    nodes: &[Node],
) -> Result<(), DocumentError> {
    let screen = plan
        .screen
        .as_ref()
        .ok_or_else(|| mismatch(plan, "HTML screen plan"))?;
    let heading = nodes
        .first()
        .ok_or_else(|| mismatch(plan, "HTML heading"))?;
    let heading = element(
        plan,
        known,
        heading,
        "h1",
        &[("id", "content")],
        "HTML heading",
    )?;
    runs(
        plan,
        known,
        &plan.blocks[0].runs,
        heading,
        false,
        0,
        prepared,
    )?;
    let mut at = 1;
    for (index, block) in plan.blocks.iter().enumerate().skip(1) {
        if !screen.gaps[index - 1].message.is_empty() {
            let gap = nodes
                .get(at)
                .ok_or_else(|| mismatch(plan, "HTML gap inventory"))?;
            let gap = element(
                plan,
                known,
                gap,
                "p",
                &[("class", "gap")],
                "HTML gap boundary",
            )?;
            text(
                plan,
                known,
                gap,
                &screen.gaps[index - 1].message,
                "HTML gap sentence",
            )?;
            at += 1;
        }
        let body = nodes
            .get(at)
            .ok_or_else(|| mismatch(plan, "HTML block inventory"))?;
        if let Some(gap) = screen.html_gaps.get(&index) {
            let body = element(
                plan,
                known,
                body,
                "p",
                &[("class", "gap")],
                "HTML withheld block boundary",
            )?;
            text(
                plan,
                known,
                body,
                &gap.message,
                "HTML withheld block sentence",
            )?;
            at += 1;
            continue;
        }
        if block.paragraph {
            let body = element(plan, known, body, "p", &[], "HTML paragraph boundary")?;
            runs(plan, known, &block.runs, body, false, index, prepared)?;
            at += 1;
            continue;
        }
        let refrain_index = super::plan::refrain_index(&plan.blocks, index)
            .ok_or_else(|| mismatch(plan, "HTML refrain index"))?;
        let refrain = &screen.refrains[refrain_index];
        let attrs = if refrain.quotation.is_some() {
            vec![("class", "refrain")]
        } else {
            vec![("class", "refrain"), ("id", refrain.fragment.as_str())]
        };
        let body = element(plan, known, body, "p", &attrs, "HTML refrain fragment")?;
        runs(plan, known, &block.runs, body, false, index, prepared)?;
        at += 1;
        let attribution = nodes
            .get(at)
            .ok_or_else(|| mismatch(plan, "HTML attribution"))?;
        let attribution = element(
            plan,
            known,
            attribution,
            "p",
            &[("class", "attribution")],
            "HTML attribution boundary",
        )?;
        if let Some(quotation) = &refrain.quotation {
            let [link] = attribution else {
                return Err(mismatch(plan, "HTML quotation link"));
            };
            let href = format!(
                "../{}/{}.html#{}",
                quotation.owner_document, quotation.language, quotation.fragment
            );
            let link = element(
                plan,
                known,
                link,
                "a",
                &[("href", &href)],
                "HTML quotation href",
            )?;
            text(plan, known, link, &refrain.handle, "HTML quotation handle")?;
        } else {
            text(
                plan,
                known,
                attribution,
                &refrain.handle,
                "HTML refrain handle",
            )?;
        }
        at += 1;
    }
    let last = nodes
        .get(at)
        .ok_or_else(|| mismatch(plan, "HTML final gap"))?;
    let last = element(
        plan,
        known,
        last,
        "p",
        &[("class", "gap")],
        "HTML final gap",
    )?;
    text(
        plan,
        known,
        last,
        &screen
            .gaps
            .last()
            .ok_or_else(|| mismatch(plan, "HTML final gap plan"))?
            .message,
        "HTML final gap sentence",
    )?;
    at += 1;
    for section in &screen.end_matter {
        match section {
            super::plan::ScreenEndMatter::References(entries) => {
                let heading = nodes
                    .get(at)
                    .ok_or_else(|| mismatch(plan, "HTML bibliography heading position"))?;
                let heading = element(
                    plan,
                    known,
                    heading,
                    "h2",
                    &[("id", "references")],
                    "HTML bibliography heading anchor",
                )?;
                text(
                    plan,
                    known,
                    heading,
                    "References",
                    "HTML bibliography heading text",
                )?;
                at += 1;
                for entry in entries {
                    let row = nodes
                        .get(at)
                        .ok_or_else(|| mismatch(plan, "HTML bibliography entry inventory"))?;
                    let row = element(
                        plan,
                        known,
                        row,
                        "p",
                        &[],
                        "HTML bibliography entry position",
                    )?;
                    let [anchor, content] = row else {
                        return Err(mismatch(plan, "HTML bibliography entry inventory"));
                    };
                    if !element(
                        plan,
                        known,
                        anchor,
                        "a",
                        &[("id", &entry.anchor)],
                        "HTML bibliography entry anchor",
                    )?
                    .is_empty()
                    {
                        return Err(mismatch(plan, "HTML bibliography entry anchor"));
                    }
                    text(
                        plan,
                        known,
                        std::slice::from_ref(content),
                        &super::bibliography::screen_text(entry),
                        "HTML bibliography entry text",
                    )?;
                    at += 1;
                }
            }
        }
    }
    if nodes.len() != at {
        return Err(mismatch(plan, "HTML end-matter inventory"));
    }
    Ok(())
}

fn check_licences(
    plan: &DocumentPlan,
    known: &[&str],
    fonts: &Fonts,
    node: &Node,
    label: &str,
) -> Result<(), DocumentError> {
    let nodes = element(
        plan,
        known,
        node,
        "details",
        &[("class", "licenses")],
        "HTML licence section",
    )?;
    let summary = nodes
        .first()
        .ok_or_else(|| mismatch(plan, "HTML licence heading"))?;
    let summary = element(plan, known, summary, "summary", &[], "HTML licence heading")?;
    text(plan, known, summary, label, "HTML licence heading")?;
    let mut files = vec![
        (
            "GUST-FONT-LICENSE.txt",
            include_str!("../../../../licenses/fonts/GUST-FONT-LICENSE.txt"),
        ),
        (
            "LPPL-1.3c.txt",
            include_str!("../../../../licenses/fonts/LPPL-1.3c.txt"),
        ),
        (
            "README-TeX-Gyre-Pagella.txt",
            include_str!("../../../../licenses/fonts/README-TeX-Gyre-Pagella.txt"),
        ),
        (
            "MANIFEST-TeX-Gyre-Pagella.txt",
            include_str!("../../../../licenses/fonts/MANIFEST-TeX-Gyre-Pagella.txt"),
        ),
    ];
    if fonts.0.iter().any(|(face, _)| face.family == "Inconsolata") {
        files.extend([
            (
                "OFL-1.1.txt",
                include_str!("../../../../licenses/fonts/OFL-1.1.txt"),
            ),
            (
                "OFL-Inconsolata.txt",
                include_str!("../../../../licenses/fonts/OFL-Inconsolata.txt"),
            ),
            (
                "Apache-2.0.txt",
                include_str!("../../../../licenses/fonts/Apache-2.0.txt"),
            ),
            (
                "README-Inconsolata.txt",
                include_str!("../../../../licenses/fonts/README-Inconsolata.txt"),
            ),
        ]);
    }
    if fonts.0.iter().any(|(face, _)| face.is_math()) {
        files.extend([
            (
                "README-TeX-Gyre-Math.txt",
                include_str!("../../../../licenses/fonts/README-TeX-Gyre-Math.txt"),
            ),
            (
                "README-TeX-Gyre-Pagella-Math.txt",
                include_str!("../../../../licenses/fonts/README-TeX-Gyre-Pagella-Math.txt"),
            ),
        ]);
    }
    files.extend(fonts.0.iter().map(|(face, _)| (face.file, face.notice)));
    if nodes.len() != files.len() + 1 {
        return Err(mismatch(plan, "HTML licence inventory"));
    }
    for (node, (file, expected)) in nodes[1..].iter().zip(files) {
        let children = element(
            plan,
            known,
            node,
            "pre",
            &[("data-file", file)],
            "HTML licence file",
        )?;
        if !matches!(children, [Node::Text(actual)] if actual == expected) {
            return Err(mismatch(plan, "HTML licence bytes"));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)] // Independent document sections report separate mismatches.
pub(super) fn check(
    plan: &DocumentPlan,
    fonts: &Fonts,
    prepared: &[MathRun],
    known_labels: &[&str],
    source: &str,
) -> Result<(), DocumentError> {
    if source.len() > MAX_BYTES {
        return Err(mismatch(plan, "HTML byte limit"));
    }
    if let Some(raw) = source
        .split_once("<style>")
        .and_then(|(_, rest)| rest.split_once("</style>"))
        .map(|(raw, _)| raw)
        && (raw.contains('<') || raw.contains('&'))
    {
        return Err(mismatch(plan, "HTML stylesheet raw text"));
    }
    let mut parser = Parser {
        source,
        at: 0,
        nodes: 0,
        library: false,
    };
    if !parser.consume("<!doctype html>") {
        return Err(mismatch(plan, "HTML doctype"));
    }
    let actual = parser
        .element(0)
        .ok_or_else(|| mismatch(plan, "HTML syntax"))?;
    if parser.at != source.len() {
        return Err(mismatch(plan, "HTML trailing bytes"));
    }
    let root = element(
        plan,
        known_labels,
        &actual,
        "html",
        &[("lang", &plan.language)],
        "HTML language",
    )?;
    let [head, body] = root else {
        return Err(mismatch(plan, "HTML document structure"));
    };
    let head = element(plan, known_labels, head, "head", &[], "HTML head")?;
    let [meta, viewport, title, style] = head else {
        return Err(mismatch(plan, "HTML head inventory"));
    };
    if !element(
        plan,
        known_labels,
        meta,
        "meta",
        &[("charset", "utf-8")],
        "HTML charset",
    )?
    .is_empty()
    {
        return Err(mismatch(plan, "HTML charset"));
    }
    if !element(
        plan,
        known_labels,
        viewport,
        "meta",
        &[
            ("name", "viewport"),
            ("content", "width=device-width, initial-scale=1"),
        ],
        "HTML viewport",
    )?
    .is_empty()
    {
        return Err(mismatch(plan, "HTML viewport"));
    }
    let heading = &plan.blocks[0];
    let heading_text = flat(&heading.runs);
    let (contents_label, licences_label) = match plan.language.as_str() {
        "en" => ("Contents", "Font licences"),
        _ => return Err(mismatch(plan, "HTML screen labels")),
    };
    let title = element(plan, known_labels, title, "title", &[], "HTML title")?;
    text(plan, known_labels, title, &heading_text, "HTML title text")?;
    let style = element(
        plan,
        known_labels,
        style,
        "style",
        &[],
        "HTML stylesheet element",
    )?;
    check_faces(plan, fonts)?;
    stylesheet(plan, fonts, style)?;
    let body = element(plan, known_labels, body, "body", &[], "HTML body")?;
    let [coverage, nav, main, licences] = body else {
        return Err(mismatch(plan, "HTML body inventory"));
    };
    let coverage = element(
        plan,
        known_labels,
        coverage,
        "section",
        &[("id", "coverage")],
        "HTML coverage section",
    )?;
    if coverage.len() != plan.coverage.audit.len() + 2 {
        return Err(mismatch(plan, "HTML coverage inventory"));
    }
    let declaration = element(
        plan,
        known_labels,
        &coverage[0],
        "p",
        &[],
        "HTML coverage declaration",
    )?;
    text(
        plan,
        known_labels,
        declaration,
        &plan.coverage.declaration,
        "HTML coverage declaration",
    )?;
    let audit = std::iter::once(&plan.coverage.runs).chain(plan.coverage.audit.iter());
    for (index, source_runs) in audit.enumerate() {
        let paragraph = element(
            plan,
            known_labels,
            &coverage[index + 1],
            "p",
            &[],
            "HTML coverage paragraph",
        )?;
        runs(
            plan,
            known_labels,
            source_runs,
            paragraph,
            false,
            usize::MAX,
            &[],
        )?;
    }
    let nav = element(
        plan,
        known_labels,
        nav,
        "nav",
        &[("aria-label", contents_label)],
        "HTML navigation",
    )?;
    let [link] = nav else {
        return Err(mismatch(plan, "HTML navigation inventory"));
    };
    let link = element(
        plan,
        known_labels,
        link,
        "a",
        &[("href", "#content")],
        "HTML navigation href",
    )?;
    text(
        plan,
        known_labels,
        link,
        &heading_text,
        "HTML navigation title",
    )?;
    let main = element(plan, known_labels, main, "main", &[], "HTML main")?;
    check_main(plan, known_labels, prepared, main)?;
    check_licences(plan, known_labels, fonts, licences, licences_label)?;
    Ok(())
}
