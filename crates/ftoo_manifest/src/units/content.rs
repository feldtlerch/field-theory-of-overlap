use super::{
    BibliographyCitationKind, BibliographyCitationOccurrence, CheckedText, HandleOccurrence,
    InsertionReason, LocatedString, MathOccurrence, ReferenceOccurrence, TagOccurrence, UnitFault,
    UnitSite, VirtueOccurrence, local_name,
};
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct StyleOccurrence {
    kind: &'static str,
    body: Range<usize>,
    site: UnitSite,
}
impl CheckedText {
    pub fn style_occurrences(
        &self,
    ) -> impl Iterator<Item = (&'static str, Range<usize>, &UnitSite)> {
        self.styles
            .iter()
            .map(|style| (style.kind, style.body.clone(), &style.site))
    }
}

fn at(source: &LocatedString, start: usize, end: usize, occurrence: Option<usize>) -> UnitSite {
    let mut site = source.site.clone();
    site.decoded = Some(start..end);
    site.occurrence = occurrence;
    site
}
type ScanFault = Box<(UnitSite, UnitFault)>;
fn malformed(
    source: &LocatedString,
    start: usize,
    end: usize,
    index: usize,
    reason: InsertionReason,
) -> ScanFault {
    Box::new((
        at(source, start, end, Some(index)),
        UnitFault::MalformedInsertion {
            reason,
            fragment: source.value[start..end].to_owned(),
        },
    ))
}
pub(super) struct ScannedText {
    pub prose: bool,
    pub math: Vec<MathOccurrence>,
    pub tags: Vec<TagOccurrence>,
    pub handles: Vec<HandleOccurrence>,
    pub references: Vec<ReferenceOccurrence>,
    pub virtues: Vec<VirtueOccurrence>,
    pub bibliography_citations: Vec<BibliographyCitationOccurrence>,
    pub styles: Vec<StyleOccurrence>,
}
fn insertion_index(scanned: &ScannedText, active_style: bool) -> usize {
    scanned.math.len()
        + scanned.tags.len()
        + scanned.handles.len()
        + scanned.references.len()
        + scanned.virtues.len()
        + scanned.bibliography_citations.len()
        + scanned.styles.len()
        + usize::from(active_style)
}
enum Insertion {
    Math(MathOccurrence),
    Tag(TagOccurrence),
    Handle(HandleOccurrence),
    Reference(ReferenceOccurrence),
    Virtue(VirtueOccurrence),
    BibliographyCitation(BibliographyCitationOccurrence),
}
pub(crate) enum AssemblyScanFault {
    Content(ScanFault),
    Limit(UnitSite),
}
impl From<ScanFault> for AssemblyScanFault {
    fn from(value: ScanFault) -> Self {
        Self::Content(value)
    }
}
pub(super) fn scan(source: &LocatedString) -> Result<ScannedText, ScanFault> {
    scan_limited(source, None, true).map_err(|fault| match fault {
        AssemblyScanFault::Content(error) => error,
        AssemblyScanFault::Limit(_) => unreachable!("no limit supplied to statement admission"),
    })
}
#[allow(clippy::too_many_lines)] // One scanner owns insertion and style boundaries together.
pub(super) fn scan_limited(
    source: &LocatedString,
    maximum: Option<usize>,
    styles: bool,
) -> Result<ScannedText, AssemblyScanFault> {
    let mut chars = source.value.char_indices().peekable();
    let mut span_index = 0;
    let mut scanned = ScannedText {
        prose: false,
        math: Vec::new(),
        tags: Vec::new(),
        handles: Vec::new(),
        references: Vec::new(),
        virtues: Vec::new(),
        bibliography_citations: Vec::new(),
        styles: Vec::new(),
    };
    let mut active_style: Option<(&'static str, usize, usize, usize)> = None;
    let mut last_style: Option<(&'static str, usize)> = None;
    while let Some((start, c)) = chars.next() {
        if c == '`' {
            let span = super::spans::parse(&source.value, &source.site, start, span_index)?;
            if maximum.is_some_and(|limit| {
                span_index
                    + scanned.math.len()
                    + scanned.tags.len()
                    + scanned.handles.len()
                    + scanned.references.len()
                    + scanned.virtues.len()
                    + scanned.bibliography_citations.len()
                    + scanned.styles.len()
                    + usize::from(active_style.is_some())
                    == limit
            }) {
                return Err(AssemblyScanFault::Limit(at(
                    source,
                    span.bytes.start,
                    span.bytes.end,
                    None,
                )));
            }
            span_index += 1;
            while chars.peek().is_some_and(|(at, _)| *at < span.bytes.end) {
                chars.next();
            }
            continue;
        }
        if c == '}'
            && let Some((kind, opener, body_start, index)) = active_style.take()
        {
            let body = &source.value[body_start..start];
            if body.is_empty() {
                return Err(malformed(
                    source,
                    opener,
                    start + 1,
                    index,
                    InsertionReason::EmptyStyle,
                )
                .into());
            }
            if body.chars().next().is_some_and(char::is_whitespace)
                || body.chars().last().is_some_and(char::is_whitespace)
            {
                return Err(malformed(
                    source,
                    opener,
                    start + 1,
                    index,
                    InsertionReason::BoundaryWhitespace,
                )
                .into());
            }
            scanned.styles.push(StyleOccurrence {
                kind,
                body: body_start..start,
                site: at(source, opener, start + 1, Some(index)),
            });
            last_style = Some((kind, start + 1));
            continue;
        }
        if c == '{'
            && let Some((_, _, _, index)) = active_style
        {
            return Err(malformed(
                source,
                start,
                start + 1,
                index,
                InsertionReason::NestedBrace,
            )
            .into());
        }
        if let Some((kind, _, _, index)) = active_style
            && matches!(kind, "emphasis" | "strong")
            && source.value[start..].starts_with(if kind == "strong" { "**" } else { "*" })
            && c == '*'
        {
            return Err(malformed(
                source,
                start,
                start + if kind == "strong" { 2 } else { 1 },
                index,
                InsertionReason::OwnCloser,
            )
            .into());
        }
        if c != '@' {
            if matches!(c, '\'' | '’')
                && let Some((next, following)) = chars.peek()
                && matches!(*following, '\'' | '’')
            {
                return Err(AssemblyScanFault::Content(Box::new((
                    at(source, start, *next + following.len_utf8(), None),
                    UnitFault::UnsupportedTextCharacter {
                        value: format!("{c}{following}"),
                    },
                ))));
            }
            if matches!(c, '$' | '\\' | '<' | '>') || (c.is_control() && !matches!(c, '\t' | '\n'))
            {
                return Err(AssemblyScanFault::Content(Box::new((
                    at(source, start, start + c.len_utf8(), None),
                    UnitFault::UnsupportedTextCharacter {
                        value: c.to_string(),
                    },
                ))));
            }
            scanned.prose |= !c.is_whitespace();
            continue;
        }
        if chars.peek().is_some_and(|(_, c)| *c == '@') {
            chars.next();
            scanned.prose = true;
            continue;
        }
        if let Some((kind, width)) = styles.then_some(()).and_then(|()| {
            ["emphasis", "strong", "smallcaps", "regular"]
                .into_iter()
                .find_map(|kind| {
                    source.value[start..]
                        .starts_with(&format!("@{kind}{{"))
                        .then_some((kind, kind.len() + 2))
                })
        }) {
            if active_style.is_some() {
                return Err(malformed(
                    source,
                    start,
                    start + width,
                    insertion_index(&scanned, true),
                    InsertionReason::NestedStyle,
                )
                .into());
            }
            if last_style.is_some_and(|(prior, end)| prior == kind && end == start) {
                return Err(malformed(
                    source,
                    start,
                    start + width,
                    insertion_index(&scanned, false),
                    InsertionReason::AdjacentStyle,
                )
                .into());
            }
            let index = insertion_index(&scanned, false);
            let body_start = start + width;
            if maximum.is_some_and(|limit| span_index + index == limit) {
                return Err(AssemblyScanFault::Limit(at(
                    source,
                    start,
                    body_start,
                    Some(index),
                )));
            }
            while chars.peek().is_some_and(|(at, _)| *at < body_start) {
                chars.next();
            }
            active_style = Some((kind, start, body_start, index));
            continue;
        }
        let index = insertion_index(&scanned, active_style.is_some());
        if !styles
            && (source.value[start..].starts_with("@see{")
                || source.value[start..].starts_with("@virtue{"))
        {
            return Err(malformed(
                source,
                start,
                start + 5,
                index,
                InsertionReason::UnknownForm,
            )
            .into());
        }
        let insertion = insertion(source, start, index, &mut chars)?;
        if !styles
            && matches!(
                &insertion,
                Insertion::BibliographyCitation(_) | Insertion::Handle(_)
            )
        {
            return Err(malformed(
                source,
                start,
                source.value[start..]
                    .find('}')
                    .map_or(source.value.len(), |end| start + end + 1),
                index,
                InsertionReason::UnknownForm,
            )
            .into());
        }
        if active_style.is_some()
            && matches!(
                &insertion,
                Insertion::Math(_)
                    | Insertion::Reference(_)
                    | Insertion::Virtue(_)
                    | Insertion::BibliographyCitation(_)
                    | Insertion::Handle(_)
            )
        {
            return Err(malformed(
                source,
                start,
                source.value[start..]
                    .find('}')
                    .map_or(source.value.len(), |end| start + end + 1),
                index,
                if matches!(&insertion, Insertion::Math(_)) {
                    InsertionReason::StyledMath
                } else if matches!(&insertion, Insertion::Reference(_) | Insertion::Virtue(_)) {
                    InsertionReason::StyledReference
                } else if matches!(&insertion, Insertion::Handle(_)) {
                    InsertionReason::StyledHandle
                } else {
                    InsertionReason::StyledBibliographyCitation
                },
            )
            .into());
        }
        if maximum.is_some_and(|limit| {
            span_index + insertion_index(&scanned, active_style.is_some()) == limit
        }) {
            let site = match &insertion {
                Insertion::Math(value) => value.site.clone(),
                Insertion::Tag(value) => value.site.clone(),
                Insertion::Handle(value) => value.site.clone(),
                Insertion::Reference(value) => value.site.clone(),
                Insertion::Virtue(value) => value.site.clone(),
                Insertion::BibliographyCitation(value) => value.site.clone(),
            };
            return Err(AssemblyScanFault::Limit(site));
        }
        match insertion {
            Insertion::Math(value) => scanned.math.push(value),
            Insertion::Tag(value) => scanned.tags.push(value),
            Insertion::Handle(value) => scanned.handles.push(value),
            Insertion::Reference(value) => scanned.references.push(value),
            Insertion::Virtue(value) => scanned.virtues.push(value),
            Insertion::BibliographyCitation(value) => scanned.bibliography_citations.push(value),
        }
    }
    if let Some((_, opener, _, index)) = active_style {
        return Err(malformed(
            source,
            opener,
            source.value.len(),
            index,
            InsertionReason::Unclosed,
        )
        .into());
    }
    Ok(scanned)
}
fn bibliography_insertion(
    source: &LocatedString,
    start: usize,
    index: usize,
    chars: &mut impl Iterator<Item = (usize, char)>,
    kind: BibliographyCitationKind,
    width: usize,
) -> Result<Insertion, ScanFault> {
    let key_start = start + width;
    for (position, _) in chars.by_ref() {
        if position + 1 == key_start {
            break;
        }
    }
    for (position, c) in chars.by_ref() {
        if c == '{' {
            return Err(malformed(
                source,
                start,
                position + 1,
                index,
                InsertionReason::NestedBrace,
            ));
        }
        if c == '}' {
            let keys: Vec<_> = source.value[key_start..position].split(',').collect();
            let valid = !keys.is_empty()
                && keys.iter().all(|key| {
                    key.starts_with("a:")
                        && !key[2..].is_empty()
                        && key[2..]
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-')
                });
            let mut seen = std::collections::BTreeSet::new();
            if !valid || keys.iter().any(|key| !seen.insert(*key)) {
                return Err(malformed(
                    source,
                    start,
                    position + 1,
                    index,
                    InsertionReason::BadBibliographyKey,
                ));
            }
            return Ok(Insertion::BibliographyCitation(
                BibliographyCitationOccurrence {
                    kind,
                    keys: keys.into_iter().map(str::to_owned).collect(),
                    site: at(source, start, position + 1, Some(index)),
                },
            ));
        }
    }
    Err(malformed(
        source,
        start,
        source.value.len(),
        index,
        InsertionReason::Unclosed,
    ))
}

#[allow(clippy::too_many_lines)] // The insertion scanner shares one delimiter and site calculation across all forms.
fn insertion(
    source: &LocatedString,
    start: usize,
    index: usize,
    chars: &mut impl Iterator<Item = (usize, char)>,
) -> Result<Insertion, ScanFault> {
    let virtue = source.value[start..].starts_with("@virtue{");
    if virtue || source.value[start..].starts_with("@handle{") {
        for _ in 0..7 {
            chars.next();
        }
        for (position, c) in chars.by_ref() {
            if c == '{' {
                return Err(malformed(
                    source,
                    start,
                    position + 1,
                    index,
                    InsertionReason::NestedBrace,
                ));
            }
            if c == '}' {
                let argument = &source.value[start + 8..position];
                if virtue
                    && (argument.is_empty()
                        || !argument.bytes().all(|byte| byte.is_ascii_lowercase()))
                {
                    return Err(malformed(
                        source,
                        start,
                        position + 1,
                        index,
                        InsertionReason::UnknownForm,
                    ));
                }
                if virtue {
                    return Ok(Insertion::Virtue(VirtueOccurrence {
                        word: argument.into(),
                        site: at(source, start, position + 1, Some(index)),
                    }));
                }
                if !local_name(argument) {
                    return Err(malformed(
                        source,
                        start,
                        position + 1,
                        index,
                        InsertionReason::BadMathName,
                    ));
                }
                return Ok(Insertion::Handle(HandleOccurrence {
                    selector: argument.to_owned(),
                    site: at(source, start, position + 1, Some(index)),
                }));
            }
        }
        return Err(malformed(
            source,
            start,
            source.value.len(),
            index,
            InsertionReason::Unclosed,
        ));
    }
    if source.value[start..].starts_with("@see{") {
        for _ in 0..4 {
            chars.next();
        }
        for (position, c) in chars.by_ref() {
            if c == '{' {
                return Err(malformed(
                    source,
                    start,
                    position + 1,
                    index,
                    InsertionReason::NestedBrace,
                ));
            }
            if c == '}' {
                let label = &source.value[start + 5..position];
                if !label.contains(':')
                    || !label
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphabetic)
                    || !label.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_')
                    })
                {
                    return Err(malformed(
                        source,
                        start,
                        position + 1,
                        index,
                        InsertionReason::UnknownForm,
                    ));
                }
                return Ok(Insertion::Reference(ReferenceOccurrence {
                    label: label.into(),
                    site: at(source, start, position + 1, Some(index)),
                }));
            }
        }
        return Err(malformed(
            source,
            start,
            source.value.len(),
            index,
            InsertionReason::Unclosed,
        ));
    }
    let bibliography = if source.value[start..].starts_with("@parencite{") {
        Some((BibliographyCitationKind::Parenthetical, "@parencite{".len()))
    } else if source.value[start..].starts_with("@textcite{") {
        Some((BibliographyCitationKind::Textual, "@textcite{".len()))
    } else {
        None
    };
    if let Some((kind, width)) = bibliography {
        return bibliography_insertion(source, start, index, chars, kind, width);
    }
    let is_tag = prefix(source, start, index, chars)?;
    let name_start = start + if is_tag { 5 } else { 6 };
    for (position, c) in chars.by_ref() {
        if c == '{' {
            return Err(malformed(
                source,
                start,
                position + 1,
                index,
                InsertionReason::NestedBrace,
            ));
        }
        if c == '}' {
            let name = &source.value[name_start..position];
            let site = at(source, start, position + 1, Some(index));
            if is_tag {
                return Ok(Insertion::Tag(TagOccurrence {
                    input: name.to_owned(),
                    site,
                }));
            }
            if !local_name(name) {
                return Err(malformed(
                    source,
                    start,
                    position + 1,
                    index,
                    InsertionReason::BadMathName,
                ));
            }
            return Ok(Insertion::Math(MathOccurrence {
                name: name.to_owned(),
                site,
            }));
        }
        if !is_tag {
            let prefix = &source.value[name_start..position + c.len_utf8()];
            let valid = if position == name_start {
                c.is_ascii_lowercase()
            } else {
                c.is_ascii_lowercase()
                    || c.is_ascii_digit()
                    || (c == '-' && !prefix[..prefix.len() - 1].ends_with('-'))
            };
            if !valid {
                return Err(malformed(
                    source,
                    start,
                    position + c.len_utf8(),
                    index,
                    InsertionReason::BadMathName,
                ));
            }
        }
    }
    Err(malformed(
        source,
        start,
        source.value.len(),
        index,
        InsertionReason::Unclosed,
    ))
}
pub(super) fn math(name: &str, source: &LocatedString) -> Option<(UnitSite, UnitFault)> {
    if source.value.chars().all(char::is_whitespace) {
        return Some((
            source.site.clone(),
            UnitFault::BlankMath {
                name: name.to_owned(),
            },
        ));
    }
    for (start, c) in source.value.char_indices() {
        let sequence = ["\\(", "\\)", "\\[", "\\]", "\\begin", "\\end"]
            .into_iter()
            .find(|token| source.value[start..].starts_with(token));
        let token = if let Some(sequence) = sequence {
            Some(sequence.to_owned())
        } else if matches!(c, '$' | '`' | '@') || (c.is_control() && !matches!(c, '\t' | '\n')) {
            Some(c.to_string())
        } else {
            None
        };
        if let Some(token) = token {
            return Some((
                at(source, start, start + token.len(), None),
                UnitFault::UnsupportedMathSyntax {
                    name: name.to_owned(),
                    token,
                },
            ));
        }
    }
    None
}

fn prefix(
    source: &LocatedString,
    start: usize,
    index: usize,
    chars: &mut impl Iterator<Item = (usize, char)>,
) -> Result<bool, ScanFault> {
    let (is_tag, tail) = match chars.next() {
        Some((_, 'm')) => (false, "ath{"),
        Some((_, 't')) => (true, "ag{"),
        Some((position, c)) => {
            return Err(malformed(
                source,
                start,
                position + c.len_utf8(),
                index,
                InsertionReason::UnknownForm,
            ));
        }
        None => {
            return Err(malformed(
                source,
                start,
                source.value.len(),
                index,
                InsertionReason::Unclosed,
            ));
        }
    };
    for expected in tail.chars() {
        match chars.next() {
            Some((position, actual)) if actual != expected => {
                return Err(malformed(
                    source,
                    start,
                    position + actual.len_utf8(),
                    index,
                    InsertionReason::UnknownForm,
                ));
            }
            None => {
                return Err(malformed(
                    source,
                    start,
                    source.value.len(),
                    index,
                    InsertionReason::Unclosed,
                ));
            }
            _ => (),
        }
    }
    Ok(is_tag)
}
