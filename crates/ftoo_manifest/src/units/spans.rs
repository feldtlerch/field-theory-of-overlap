//! The single lexical definition shared by admission and the complete weld.
use super::{LabelSpanReason, UnitFault, UnitSite, label_name};
use std::ops::Range;

pub(crate) struct Span<'a> {
    pub body: &'a str,
    pub bytes: Range<usize>,
    pub citation: bool,
}
fn failure(
    text: &str,
    site: &UnitSite,
    start: usize,
    end: usize,
    span_index: usize,
    reason: LabelSpanReason,
) -> Box<(UnitSite, UnitFault)> {
    let mut site = site.clone();
    site.decoded = Some(start..end);
    site.occurrence = None;
    Box::new((
        site,
        UnitFault::MalformedLabelSpan {
            span_index,
            reason,
            fragment: text[start..end].to_owned(),
        },
    ))
}
pub(crate) fn parse<'a>(
    text: &'a str,
    site: &UnitSite,
    start: usize,
    span_index: usize,
) -> Result<Span<'a>, Box<(UnitSite, UnitFault)>> {
    let width = text.as_bytes()[start..]
        .iter()
        .take_while(|b| **b == b'`')
        .count();
    if !(1..=2).contains(&width) {
        return Err(failure(
            text,
            site,
            start,
            start + width,
            span_index,
            LabelSpanReason::DelimiterWidth,
        ));
    }
    let body_start = start + width;
    for (relative, c) in text[body_start..].char_indices() {
        let at = body_start + relative;
        if c == '`' {
            let closing = text.as_bytes()[at..]
                .iter()
                .take_while(|b| **b == b'`')
                .count();
            let end = at + closing;
            if closing != width {
                return Err(failure(
                    text,
                    site,
                    start,
                    end,
                    span_index,
                    LabelSpanReason::MismatchedDelimiter,
                ));
            }
            let body = &text[body_start..at];
            if body.is_empty() || (width == 2 && body.chars().all(char::is_whitespace)) {
                return Err(failure(
                    text,
                    site,
                    start,
                    end,
                    span_index,
                    LabelSpanReason::EmptyBody,
                ));
            }
            if width == 1 && !label_name(body) {
                return Err(failure(
                    text,
                    site,
                    start,
                    end,
                    span_index,
                    LabelSpanReason::InvalidLabel,
                ));
            }
            if width == 1 && text[..start].ends_with("· ") {
                let mut site = site.clone();
                site.decoded = Some(start - 3..end);
                site.occurrence = None;
                return Err(Box::new((
                    site,
                    UnitFault::TextMintForbidden {
                        span_index,
                        label: body.to_owned(),
                    },
                )));
            }
            return Ok(Span {
                body,
                bytes: start..end,
                citation: width == 1,
            });
        }
        if matches!(c, '$' | '\\' | '<' | '>' | '\u{2028}' | '\u{2029}') || c.is_control() {
            return Err(failure(
                text,
                site,
                start,
                at + c.len_utf8(),
                span_index,
                LabelSpanReason::ForbiddenCharacter,
            ));
        }
    }
    Err(failure(
        text,
        site,
        start,
        text.len(),
        span_index,
        LabelSpanReason::Unclosed,
    ))
}
