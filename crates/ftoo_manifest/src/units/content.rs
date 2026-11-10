use super::{
    InsertionReason, LocatedString, MathOccurrence, TagOccurrence, UnitFault, UnitSite, local_name,
};

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
}
enum Insertion {
    Math(MathOccurrence),
    Tag(TagOccurrence),
}
pub(super) fn scan(source: &LocatedString) -> Result<ScannedText, ScanFault> {
    let mut chars = source.value.char_indices().peekable();
    let mut scanned = ScannedText {
        prose: false,
        math: Vec::new(),
        tags: Vec::new(),
    };
    while let Some((start, c)) = chars.next() {
        if c != '@' {
            if matches!(c, '`' | '$' | '\\' | '<' | '>')
                || (c.is_control() && !matches!(c, '\t' | '\n'))
            {
                return Err(Box::new((
                    at(source, start, start + c.len_utf8(), None),
                    UnitFault::UnsupportedTextCharacter {
                        value: c.to_string(),
                    },
                )));
            }
            scanned.prose |= !c.is_whitespace();
            continue;
        }
        if chars.peek().is_some_and(|(_, c)| *c == '@') {
            chars.next();
            scanned.prose = true;
            continue;
        }
        let index = scanned.math.len() + scanned.tags.len();
        match insertion(source, start, index, &mut chars)? {
            Insertion::Math(value) => scanned.math.push(value),
            Insertion::Tag(value) => scanned.tags.push(value),
        }
    }
    Ok(scanned)
}
fn insertion(
    source: &LocatedString,
    start: usize,
    index: usize,
    chars: &mut impl Iterator<Item = (usize, char)>,
) -> Result<Insertion, ScanFault> {
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
