use super::{InsertionReason, LocatedString, MathOccurrence, UnitFault, UnitSite, local_name};

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
pub(super) fn scan(source: &LocatedString) -> Result<(bool, Vec<MathOccurrence>), ScanFault> {
    let text = &source.value;
    let mut chars = text.char_indices().peekable();
    let mut prose = false;
    let mut occurrences = Vec::new();
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
            prose |= !c.is_whitespace();
            continue;
        }
        if chars.peek().is_some_and(|(_, c)| *c == '@') {
            chars.next();
            prose = true;
            continue;
        }
        let index = occurrences.len();
        prefix(source, start, index, &mut chars)?;
        let name_start = start + 6;
        let mut closed = false;
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
                let name = &text[name_start..position];
                if !local_name(name) {
                    return Err(malformed(
                        source,
                        start,
                        position + 1,
                        index,
                        InsertionReason::BadMathName,
                    ));
                }
                occurrences.push(MathOccurrence {
                    name: name.to_owned(),
                    site: at(source, start, position + 1, Some(index)),
                });
                closed = true;
                break;
            }
            let prefix = &text[name_start..position + c.len_utf8()];
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
        if !closed {
            return Err(malformed(
                source,
                start,
                text.len(),
                index,
                InsertionReason::Unclosed,
            ));
        }
    }
    Ok((prose, occurrences))
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
) -> Result<(), ScanFault> {
    for expected in "math{".chars() {
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
    Ok(())
}
