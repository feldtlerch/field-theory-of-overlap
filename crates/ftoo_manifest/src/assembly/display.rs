use super::{
    AddressFamily, AssemblyInputDiagnostic, AssemblyInputFault, AssemblyResource, AssemblySite,
    AssemblyStage, BodyAddressMember, BodyAddressSide, CheckedSkeletons, DisplayKind,
    DisplaySyntaxReason, DocumentTarget,
};
use crate::{LocatedString, PathPart, UnitSite, WeldedUnits};
use std::{collections::BTreeMap, ops::Range};

/// Untrusted companion data, with both target alternatives retained together.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayDraft {
    pub unit: LocatedString,
    pub kind: DisplayKind,
    pub bodies: BTreeMap<DocumentTarget, DisplayTargetDraft>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayTargetDraft {
    pub site: UnitSite,
    pub languages: BTreeMap<String, LocatedString>,
}
/// A scanned region; insertion coordinates are local to its immutable text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayRegion {
    source: LocatedString,
    math: Vec<crate::MathOccurrence>,
    tags: Vec<crate::TagOccurrence>,
    bytes: Range<usize>,
    insertion_start: usize,
}
impl DisplayRegion {
    #[must_use]
    pub fn source(&self) -> &LocatedString {
        &self.source
    }
    #[must_use]
    pub fn math(&self) -> &[crate::MathOccurrence] {
        &self.math
    }
    #[must_use]
    pub fn tags(&self) -> &[crate::TagOccurrence] {
        &self.tags
    }
    /// # Panics
    /// Panics only if the shared scanner disagrees with its own admission, which callers cannot arrange.
    #[must_use]
    pub fn spans(&self) -> Vec<(String, UnitSite, bool)> {
        super::assembly_spans(
            &self.source.value,
            &self.source.site,
            &self.math,
            &self.tags,
        )
    }
    #[must_use]
    pub fn bytes(&self) -> Range<usize> {
        self.bytes.clone()
    }
    #[must_use]
    pub fn site(&self, local: &UnitSite) -> UnitSite {
        let mut site = local.clone();
        site.decoded = Some(local.decoded.as_ref().map_or(self.bytes.clone(), |r| {
            r.start + self.bytes.start..r.end + self.bytes.start
        }));
        site.occurrence = local.occurrence.map(|index| index + self.insertion_start);
        site
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisplayLayout {
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<usize>>,
        rules: Vec<usize>,
    },
    Quote {
        rows: Vec<Option<usize>>,
    },
    Picture {
        width: i32,
        height: i32,
        commands: Vec<PictureCommand>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PictureCommand {
    Label {
        x: i32,
        y: i32,
        region: usize,
    },
    Line {
        x: i32,
        y: i32,
        dx: i32,
        dy: i32,
        length: i32,
        vector: bool,
    },
}
/// Syntax-checked body. Construction is available only through its bounded parser.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedDisplayBody {
    source: LocatedString,
    layout: DisplayLayout,
    regions: Vec<DisplayRegion>,
}
impl ParsedDisplayBody {
    #[must_use]
    pub fn source(&self) -> &LocatedString {
        &self.source
    }
    #[must_use]
    pub fn layout(&self) -> &DisplayLayout {
        &self.layout
    }
    #[must_use]
    pub fn regions(&self) -> &[DisplayRegion] {
        &self.regions
    }
}
type BodyFault = Box<(UnitSite, AssemblyInputFault)>;
struct Parser<'a> {
    source: &'a LocatedString,
    kind: DisplayKind,
    at: usize,
    regions: Vec<DisplayRegion>,
    occurrences: usize,
    insertions: usize,
}
impl Parser<'_> {
    fn fault(&self, bytes: Range<usize>, fault: AssemblyInputFault) -> BodyFault {
        let mut site = self.source.site.clone();
        site.decoded = Some(bytes);
        Box::new((site, fault))
    }
    fn syntax(
        &self,
        bytes: Range<usize>,
        reason: DisplaySyntaxReason,
        expected: &[&str],
    ) -> BodyFault {
        self.fault(
            bytes.clone(),
            AssemblyInputFault::DisplaySyntax {
                kind: self.kind,
                reason,
                token: self.source.value[bytes].into(),
                expected: expected.iter().map(|s| (*s).into()).collect(),
            },
        )
    }
    fn limit(&self, bytes: Range<usize>, resource: AssemblyResource, limit: usize) -> BodyFault {
        self.fault(
            bytes,
            AssemblyInputFault::Limit {
                resource,
                limit,
                actual: limit + 1,
            },
        )
    }
    fn region(&mut self, bytes: Range<usize>, path: &[usize]) -> Result<usize, BodyFault> {
        let mut site = self.source.site.clone();
        site.path.push(PathPart::Key("regions".into()));
        site.path.extend(path.iter().copied().map(PathPart::Index));
        let source = LocatedString {
            value: self.source.value[bytes.clone()].into(),
            site,
        };
        let (math, tags) = crate::units::assembly_inline(&source, 4096 - self.occurrences)
            .map_err(|error| {
                let (mut site, fault) = match error {
                    crate::units::AssemblyScanFault::Content(error) => {
                        let (site, cause) = *error;
                        (site, AssemblyInputFault::DisplayInline { cause })
                    }
                    crate::units::AssemblyScanFault::Limit(site) => (
                        site,
                        AssemblyInputFault::Limit {
                            resource: AssemblyResource::BodyOccurrences,
                            limit: 4096,
                            actual: 4097,
                        },
                    ),
                };
                if let Some(range) = &mut site.decoded {
                    range.start += bytes.start;
                    range.end += bytes.start;
                }
                site.occurrence = site.occurrence.map(|n| n + self.insertions);
                if matches!(fault, AssemblyInputFault::Limit { .. }) {
                    site.path.clone_from(&self.source.site.path);
                    site.occurrence = None;
                }
                Box::new((site, fault))
            })?;
        let spans = super::assembly_spans(&source.value, &source.site, &math, &tags);
        let mut sites: Vec<_> = tags
            .iter()
            .map(crate::TagOccurrence::site)
            .chain(math.iter().map(crate::MathOccurrence::site))
            .chain(spans.iter().map(|(_, site, _)| site))
            .collect();
        sites.sort_by_key(|site| site.decoded.as_ref().map(|r| r.start));
        for site in sites {
            if self.occurrences == 4096 {
                let range = site.decoded.clone().unwrap_or(0..0);
                return Err(self.limit(
                    range.start + bytes.start..range.end + bytes.start,
                    AssemblyResource::BodyOccurrences,
                    4096,
                ));
            }
            self.occurrences += 1;
        }
        let index = self.regions.len();
        let insertion_start = self.insertions;
        self.insertions += tags.len() + math.len();
        self.regions.push(DisplayRegion {
            source,
            math,
            tags,
            bytes,
            insertion_start,
        });
        Ok(index)
    }
    // Layout punctuation inside an insertion or either label-span form is literal.
    fn protected_end(&self, at: usize) -> Option<usize> {
        let tail = &self.source.value[at..];
        if tail.starts_with("@@") {
            return Some(at + 2);
        }
        if tail.starts_with('`') {
            let delimiter = if tail.starts_with("``") { "``" } else { "`" };
            return Some(
                tail[delimiter.len()..]
                    .find(delimiter)
                    .map_or(self.source.value.len(), |n| {
                        at + delimiter.len() + n + delimiter.len()
                    }),
            );
        }
        if tail.starts_with("@tag{") || tail.starts_with("@math{") {
            return Some(
                tail.find('}')
                    .map_or(self.source.value.len(), |n| at + n + 1),
            );
        }
        None
    }
    fn separators(&self, bytes: Range<usize>, delimiter: &str) -> Vec<usize> {
        let mut result = Vec::new();
        let mut at = bytes.start;
        while at < bytes.end {
            if let Some(end) = self.protected_end(at) {
                at = end;
            } else if self.source.value[at..].starts_with(delimiter) {
                result.push(at);
                if result.len() == 65 {
                    break;
                }
                at += delimiter.len();
            } else {
                at += self.source.value[at..]
                    .chars()
                    .next()
                    .expect("nonempty")
                    .len_utf8();
            }
        }
        result
    }
    fn cells(
        &mut self,
        bytes: Range<usize>,
        delimiter: &str,
        row: usize,
        columns: usize,
    ) -> Result<Vec<usize>, BodyFault> {
        let mut ends = self.separators(bytes.clone(), delimiter);
        ends.push(bytes.end);
        if ends.len() > 64 {
            return Err(self.limit(
                ends[63] + delimiter.len()..ends[64],
                AssemblyResource::BodyColumns,
                64,
            ));
        }
        if ends.len() != columns {
            return Err(self.fault(
                bytes,
                AssemblyInputFault::TableColumns {
                    row,
                    expected: columns,
                    actual: ends.len(),
                },
            ));
        }
        let mut at = bytes.start;
        let mut cells = Vec::new();
        for (column, end) in ends.into_iter().enumerate() {
            cells.push(self.region(at..end, &[row, column])?);
            at = end + delimiter.len();
        }
        Ok(cells)
    }
    fn lines(&self) -> Vec<Range<usize>> {
        let mut at = 0;
        self.source
            .value
            .strip_suffix('\n')
            .unwrap_or(&self.source.value)
            .split('\n')
            .take(1025)
            .map(|line| {
                let range = at..at + line.len();
                at += line.len() + 1;
                range
            })
            .collect()
    }
    fn pipe(&self, line: Range<usize>) -> Result<Range<usize>, BodyFault> {
        let value = &self.source.value[line.clone()];
        if value.len() < 2 || !value.starts_with('|') || !value.ends_with('|') {
            return Err(self.syntax(line, DisplaySyntaxReason::BadDelimiter, &["| cells |"]));
        }
        Ok(line.start + 1..line.end - 1)
    }
    fn markdown_table(&mut self) -> Result<DisplayLayout, BodyFault> {
        let lines = self.lines();
        let header = self.pipe(lines[0].clone())?;
        let count = self.separators(header.clone(), "|").len() + 1;
        if count > 64 {
            let separators = self.separators(header.clone(), "|");
            let start = separators[63] + 1;
            let end = separators.get(64).copied().unwrap_or(header.end);
            return Err(self.limit(start..end, AssemblyResource::BodyColumns, 64));
        }
        if lines.len() < 2 {
            return Err(self.syntax(
                self.source.value.len()..self.source.value.len(),
                DisplaySyntaxReason::MissingAlignment,
                &["alignment row"],
            ));
        }
        let alignment = self.pipe(lines[1].clone())?;
        let mut ends = self.separators(alignment.clone(), "|");
        ends.push(alignment.end);
        if ends.len() > 64 {
            return Err(self.limit(ends[63] + 1..ends[64], AssemblyResource::BodyColumns, 64));
        }
        if ends.len() != count {
            return Err(self.fault(
                lines[1].clone(),
                AssemblyInputFault::TableColumns {
                    row: 1,
                    expected: count,
                    actual: ends.len(),
                },
            ));
        }
        let mut columns = Vec::new();
        let mut at = alignment.start;
        for end in ends {
            let cell = self.source.value[at..end].trim_matches(' ');
            if !["---", ":---", "---:", ":---:"].contains(&cell) {
                return Err(self.syntax(
                    at..end,
                    DisplaySyntaxReason::MissingAlignment,
                    &["---", ":---", "---:", ":---:"],
                ));
            }
            columns.push(cell.into());
            at = end + 1;
        }
        if lines.len() < 3 {
            return Err(self.syntax(
                self.source.value.len()..self.source.value.len(),
                DisplaySyntaxReason::MissingRow,
                &["data row"],
            ));
        }
        let mut rows = Vec::new();
        for (row, line) in lines.into_iter().enumerate() {
            if row >= 1024 {
                return Err(self.limit(line, AssemblyResource::BodyRows, 1024));
            }
            if row != 1 {
                let inner = self.pipe(line.clone())?;
                rows.push(self.cells(inner, "|", row, count).map_err(|mut fault| {
                    if matches!(fault.1, AssemblyInputFault::TableColumns { .. }) {
                        fault.0.decoded = Some(line);
                    }
                    fault
                })?);
            }
        }
        Ok(DisplayLayout::Table {
            columns,
            rows,
            rules: Vec::new(),
        })
    }
    fn quote(&mut self) -> Result<DisplayLayout, BodyFault> {
        let mut rows = Vec::new();
        for (row, line) in self.lines().into_iter().enumerate() {
            if row == 1024 {
                return Err(self.limit(line, AssemblyResource::BodyRows, 1024));
            }
            let text = &self.source.value[line.clone()];
            if text == ">" {
                rows.push(None);
            } else if text.starts_with("> ") {
                rows.push(Some(self.region(line.start + 2..line.end, &[row])?));
            } else {
                return Err(self.syntax(line, DisplaySyntaxReason::BadDelimiter, &["> "]));
            }
        }
        Ok(DisplayLayout::Quote { rows })
    }
    fn whitespace(&mut self) {
        while self.at < self.source.value.len()
            && matches!(
                self.source.value.as_bytes()[self.at],
                b' ' | b'\t' | b'\n' | b'\r'
            )
        {
            self.at += 1;
        }
    }
    fn token(&mut self, token: &str) -> Result<(), BodyFault> {
        self.whitespace();
        if self.source.value[self.at..].starts_with(token) {
            self.at += token.len();
            Ok(())
        } else {
            let end = self.source.value[self.at..]
                .chars()
                .next()
                .map_or(self.at, |c| self.at + c.len_utf8());
            Err(self.syntax(self.at..end, DisplaySyntaxReason::BadDelimiter, &[token]))
        }
    }
    fn end(&mut self) -> Result<(), BodyFault> {
        self.whitespace();
        if self.at == self.source.value.len() {
            Ok(())
        } else {
            Err(self.syntax(
                self.at..self.source.value.len(),
                DisplaySyntaxReason::TrailingContent,
                &[],
            ))
        }
    }
    #[allow(clippy::too_many_lines)] // Keep the bounded tabular state machine together.
    fn tabular(&mut self) -> Result<DisplayLayout, BodyFault> {
        self.token(r"\begin{tabular}{")?;
        let start = self.at;
        let Some(end) = self.source.value[start..].find('}').map(|n| n + start) else {
            return Err(self.syntax(
                start..self.source.value.len(),
                DisplaySyntaxReason::UnclosedLayout,
                &["}"],
            ));
        };
        let mut columns = Vec::new();
        for (offset, c) in self.source.value[start..end].char_indices() {
            if c == ' ' {
                continue;
            }
            if !matches!(c, 'l' | 'c' | 'r') {
                return Err(self.syntax(
                    start + offset..start + offset + c.len_utf8(),
                    DisplaySyntaxReason::UnknownLayout,
                    &["l", "c", "r"],
                ));
            }
            if columns.len() == 64 {
                return Err(self.limit(
                    start + offset..start + offset + 1,
                    AssemblyResource::BodyColumns,
                    64,
                ));
            }
            columns.push(c.to_string());
        }
        if columns.is_empty() {
            return Err(self.syntax(
                start..end,
                DisplaySyntaxReason::MissingHeader,
                &["l", "c", "r"],
            ));
        }
        self.at = end + 1;
        let mut rows = Vec::new();
        let mut rules = Vec::new();
        loop {
            self.whitespace();
            if self.source.value[self.at..].starts_with(r"\end{tabular}") {
                self.at += 13;
                break;
            }
            if self.source.value[self.at..].starts_with(r"\hline") {
                if rows.is_empty() {
                    return Err(self.syntax(
                        self.at..self.at + 6,
                        DisplaySyntaxReason::MissingRow,
                        &["row"],
                    ));
                }
                rules.push(rows.len());
                self.at += 6;
                self.whitespace();
                if self.source.value[self.at..].starts_with(r"\end{tabular}") {
                    return Err(self.syntax(
                        self.at..self.at + 13,
                        DisplaySyntaxReason::MissingRow,
                        &["row"],
                    ));
                }
            }
            let start = self.at;
            let mut end = None;
            while self.at < self.source.value.len() {
                if let Some(next) = self.protected_end(self.at) {
                    self.at = next;
                } else if self.source.value[self.at..].starts_with(r"\\") {
                    end = Some(self.at);
                    break;
                } else {
                    self.at += self.source.value[self.at..]
                        .chars()
                        .next()
                        .expect("nonempty")
                        .len_utf8();
                }
            }
            let Some(end) = end else {
                return Err(self.syntax(
                    start..self.at,
                    DisplaySyntaxReason::UnclosedLayout,
                    &[r"\\"],
                ));
            };
            if rows.len() == 1024 {
                return Err(self.limit(start..end, AssemblyResource::BodyRows, 1024));
            }
            rows.push(
                self.cells(start..end, "&", rows.len(), columns.len())
                    .map_err(|mut fault| {
                        if matches!(fault.1, AssemblyInputFault::TableColumns { .. }) {
                            fault.0.decoded = Some(start..end + 2);
                        }
                        fault
                    })?,
            );
            self.at = end + 2;
        }
        if rows.len() < 2 {
            return Err(self.syntax(
                self.at..self.at,
                DisplaySyntaxReason::MissingRow,
                &["two rows"],
            ));
        }
        self.end()?;
        Ok(DisplayLayout::Table {
            columns,
            rows,
            rules,
        })
    }
    fn integer(&mut self, field: &str, minimum: i32, maximum: i32) -> Result<i32, BodyFault> {
        self.whitespace();
        let start = self.at;
        while self.at < self.source.value.len() {
            let c = self.source.value[self.at..]
                .chars()
                .next()
                .expect("nonempty suffix");
            if matches!(c, ',' | ')' | '}' | ' ' | '\t' | '\r' | '\n') {
                break;
            }
            self.at += c.len_utf8();
        }
        let value = &self.source.value[start..self.at];
        let digits = value.strip_prefix('-').unwrap_or(value);
        if digits.is_empty()
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
            || (digits.starts_with('0') && value != "0")
        {
            return Err(self.syntax(
                start..self.at,
                DisplaySyntaxReason::InvalidInteger,
                &["minimal decimal integer"],
            ));
        }
        match value.parse::<i32>() {
            Ok(n) if (minimum..=maximum).contains(&n) => Ok(n),
            _ => Err(self.fault(
                start..self.at,
                AssemblyInputFault::Coordinate {
                    field: field.into(),
                    value: value.into(),
                    minimum,
                    maximum,
                },
            )),
        }
    }
    fn picture(&mut self) -> Result<DisplayLayout, BodyFault> {
        self.token(r"\begin{picture}(")?;
        let width = self.integer("W", 1, 4096)?;
        self.token(",")?;
        let height = self.integer("H", 1, 4096)?;
        self.token(")")?;
        let mut commands = Vec::new();
        loop {
            self.whitespace();
            if self.source.value[self.at..].starts_with(r"\end{picture}") {
                self.at += 13;
                break;
            }
            let start = self.at;
            if commands.len() == 1024 {
                return Err(self.limit(start..start, AssemblyResource::BodyRows, 1024));
            }
            self.token(r"\put(")?;
            let x = self.integer("X", 0, width)?;
            self.token(",")?;
            let y = self.integer("Y", 0, height)?;
            self.token("){")?;
            let vector = self.source.value[self.at..].starts_with(r"\vector(");
            if vector || self.source.value[self.at..].starts_with(r"\line(") {
                self.token(if vector { r"\vector(" } else { r"\line(" })?;
                let direction = self.at;
                let dx = self.integer("DX", -1, 1)?;
                self.token(",")?;
                let dy = self.integer("DY", -1, 1)?;
                if dx == 0 && dy == 0 {
                    return Err(self.syntax(
                        direction..self.at,
                        DisplaySyntaxReason::ZeroDirection,
                        &["nonzero direction"],
                    ));
                }
                self.token("){")?;
                let length = self.integer("L", 1, 4096)?;
                self.token("}}")?;
                commands.push(PictureCommand::Line {
                    x,
                    y,
                    dx,
                    dy,
                    length,
                    vector,
                });
            } else {
                let begin = self.at;
                while self.at < self.source.value.len()
                    && !self.source.value[self.at..].starts_with('}')
                {
                    if let Some(next) = self.protected_end(self.at) {
                        self.at = next;
                    } else {
                        self.at += self.source.value[self.at..]
                            .chars()
                            .next()
                            .expect("nonempty")
                            .len_utf8();
                    }
                }
                if self.at == self.source.value.len() {
                    return Err(self.syntax(
                        begin..self.at,
                        DisplaySyntaxReason::UnclosedLayout,
                        &["}"],
                    ));
                }
                if self.source.value[begin..self.at].trim().is_empty() {
                    return Err(self.syntax(
                        begin..self.at,
                        DisplaySyntaxReason::EmptyBody,
                        &["label content"],
                    ));
                }
                let region = self.region(begin..self.at, &[commands.len()])?;
                self.at += 1;
                commands.push(PictureCommand::Label { x, y, region });
            }
        }
        if commands.is_empty() {
            return Err(self.syntax(
                self.at..self.at,
                DisplaySyntaxReason::MissingRow,
                &[r"\put"],
            ));
        }
        self.end()?;
        Ok(DisplayLayout::Picture {
            width,
            height,
            commands,
        })
    }
}
/// Parse the complete bounded layout and scan every inline region.
/// # Errors
/// Returns the first decisive syntax or resource fault with decoded body coordinates.
pub fn parse_display_body(
    kind: DisplayKind,
    target: DocumentTarget,
    source: &LocatedString,
) -> Result<ParsedDisplayBody, BodyFault> {
    let mut parser = Parser {
        source,
        kind,
        at: 0,
        regions: Vec::new(),
        occurrences: 0,
        insertions: 0,
    };
    if source.value.len() > 262_144 {
        return Err(parser.limit(262_144..262_145, AssemblyResource::BodyBytes, 262_144));
    }
    if source.value.trim().is_empty() {
        return Err(parser.syntax(
            0..source.value.len(),
            DisplaySyntaxReason::EmptyBody,
            &["content"],
        ));
    }
    let layout = match (kind, target) {
        (DisplayKind::Table, DocumentTarget::Markdown) => parser.markdown_table()?,
        (DisplayKind::Table, DocumentTarget::Latex) => parser.tabular()?,
        (DisplayKind::Figure, DocumentTarget::Markdown) => parser.quote()?,
        (DisplayKind::Figure, DocumentTarget::Latex) => parser.picture()?,
    };
    if parser
        .regions
        .iter()
        .all(|r| r.source.value.as_str().trim().is_empty())
    {
        return Err(parser.syntax(
            0..source.value.len(),
            DisplaySyntaxReason::EmptyBody,
            &["content"],
        ));
    }
    Ok(ParsedDisplayBody {
        source: source.clone(),
        layout,
        regions: parser.regions,
    })
}

/// All owners, layouts, occurrences and address sets checked together.
/// ```
/// fn inspect(value: &ftoo_manifest::CheckedDisplays<'_>) -> usize { value.bodies().len() }
/// ```
/// ```compile_fail,E0451
/// let displays = ftoo_manifest::CheckedDisplays { bodies: std::collections::BTreeMap::new(), context: std::marker::PhantomData };
/// ```
/// ```compile_fail,E0277
/// fn convert(draft: ftoo_manifest::DisplayDraft) -> ftoo_manifest::CheckedDisplays<'static> { draft.into() }
/// ```
/// ```compile_fail
/// fn escape<'a>(drafts: &[ftoo_manifest::DisplayDraft], skeletons: &'a ftoo_manifest::CheckedSkeletons<'a>, welded: &'a ftoo_manifest::WeldedUnits<'a,'a,'a,'a,'a,'a>) -> ftoo_manifest::CheckedDisplays<'static> {
///     ftoo_manifest::check_displays(drafts,skeletons,welded).unwrap()
/// }
/// ```
/// ```compile_fail,E0599
/// let displays = ftoo_manifest::CheckedDisplays::default();
/// ```
/// ```compile_fail,E0616
/// fn mutate(displays: &mut ftoo_manifest::CheckedDisplays) { displays.bodies.clear(); }
/// ```
/// ```compile_fail,E0277
/// fn deserialize<T: serde::de::DeserializeOwned>() {}
/// deserialize::<ftoo_manifest::CheckedDisplays>();
/// ```
#[derive(Debug)]
pub struct CheckedDisplays<'a> {
    context: std::marker::PhantomData<&'a ()>,
    bodies: BTreeMap<String, BTreeMap<(DocumentTarget, String), ParsedDisplayBody>>,
}
impl CheckedDisplays<'_> {
    #[must_use]
    pub fn bodies(
        &self,
    ) -> &BTreeMap<String, BTreeMap<(DocumentTarget, String), ParsedDisplayBody>> {
        &self.bodies
    }
}
fn diagnostic(
    unit: Option<&crate::CheckedUnit>,
    draft: &DisplayDraft,
    target: Option<DocumentTarget>,
    language: Option<&str>,
    stage: AssemblyStage,
    site: UnitSite,
    fault: AssemblyInputFault,
) -> AssemblyInputDiagnostic {
    AssemblyInputDiagnostic {
        rule: if stage == AssemblyStage::DisplayEquality {
            crate::RuleId::M059
        } else {
            crate::RuleId::M058
        },
        stage,
        document: unit.map(|u| u.document().into()),
        unit: Some(draft.unit.value.clone()),
        language: language.map(str::to_owned),
        target,
        site: AssemblySite::Source(site),
        fault,
    }
}
fn refuse(mut errors: Vec<AssemblyInputDiagnostic>) -> Result<(), Vec<AssemblyInputDiagnostic>> {
    if errors.is_empty() {
        Ok(())
    } else {
        super::diagnostics::sort(&mut errors);
        Err(errors)
    }
}
/// Admit companions only after statement ownership and weld prerequisites succeed.
/// # Errors
/// Collects all independent errors in the earliest failing semantic phase.
/// # Panics
/// Panics only if private phase invariants disagree, which input cannot arrange.
pub fn check_displays<'a>(
    drafts: &[DisplayDraft],
    skeletons: &'a CheckedSkeletons<'_>,
    welded: &'a WeldedUnits<'_, '_, '_, '_, '_, '_>,
) -> Result<CheckedDisplays<'a>, Vec<AssemblyInputDiagnostic>> {
    let units = skeletons
        .corpus()
        .units()
        .iter()
        .map(|unit| (unit.label().as_str(), unit))
        .collect();
    ownership(drafts, skeletons, &units)?;
    let checked = parse_bodies(drafts, &units)?;
    let sides = resolve_bodies(drafts, &units, &checked, welded)?;
    equality(drafts, &units, &sides)?;
    Ok(CheckedDisplays {
        bodies: checked,
        context: std::marker::PhantomData,
    })
}
type DisplayUnits<'a> = BTreeMap<&'a str, &'a crate::CheckedUnit>;
type DisplayBodies = BTreeMap<String, BTreeMap<(DocumentTarget, String), ParsedDisplayBody>>;
type DisplaySides = BTreeMap<String, Vec<(BodyAddressSide, BodyAddressSide)>>;
fn ownership(
    drafts: &[DisplayDraft],
    skeletons: &CheckedSkeletons<'_>,
    units: &DisplayUnits<'_>,
) -> Result<(), Vec<AssemblyInputDiagnostic>> {
    let corpus = skeletons.corpus();
    let mut errors = Vec::new();
    let mut owners: BTreeMap<&str, Vec<UnitSite>> = BTreeMap::new();
    for draft in drafts {
        owners
            .entry(&draft.unit.value)
            .or_default()
            .push(draft.unit.site.clone());
    }
    let mut repeated = std::collections::BTreeSet::new();
    for draft in drafts {
        let unit = units.get(draft.unit.value.as_str()).copied();
        let sites = &owners[draft.unit.value.as_str()];
        if sites.len() > 1 && repeated.insert(draft.unit.value.clone()) {
            errors.push(diagnostic(
                unit,
                draft,
                None,
                None,
                AssemblyStage::Ownership,
                draft.unit.site.clone(),
                AssemblyInputFault::DisplayMultiplicity {
                    unit: draft.unit.value.clone(),
                    expected: 1,
                    actual: sites.len(),
                    sites: sites.clone(),
                },
            ));
        }
        let Some(unit) = unit else {
            errors.push(diagnostic(
                None,
                draft,
                None,
                None,
                AssemblyStage::Ownership,
                draft.unit.site.clone(),
                AssemblyInputFault::UnknownUnit {
                    label: draft.unit.value.clone(),
                    available: units.keys().map(|s| (*s).into()).collect(),
                },
            ));
            continue;
        };
        for position in skeletons
            .documents()
            .values()
            .flatten()
            .filter(|p| p.unit() == draft.unit.value && p.depth().is_some())
        {
            errors.push(diagnostic(
                Some(unit),
                draft,
                None,
                None,
                AssemblyStage::Ownership,
                position.site().clone(),
                AssemblyInputFault::DisplayHeading {
                    unit: draft.unit.value.clone(),
                    display_at: draft.unit.site.clone(),
                },
            ));
        }
        languages(
            draft,
            unit,
            &corpus.documents()[unit.document()],
            &mut errors,
        );
    }
    refuse(errors)
}
fn languages(
    draft: &DisplayDraft,
    unit: &crate::CheckedUnit,
    required: &[String],
    errors: &mut Vec<AssemblyInputDiagnostic>,
) {
    for target in [DocumentTarget::Markdown, DocumentTarget::Latex] {
        if !draft.bodies.contains_key(&target) {
            let required = required.to_vec();
            errors.push(diagnostic(
                Some(unit),
                draft,
                Some(target),
                None,
                AssemblyStage::Ownership,
                draft.unit.site.parent(),
                AssemblyInputFault::BodyLanguages {
                    target,
                    missing: required.clone(),
                    required,
                    unexpected: Vec::new(),
                },
            ));
        }
    }
    for (target, bodies) in &draft.bodies {
        let missing: Vec<_> = required
            .iter()
            .filter(|l| !bodies.languages.contains_key(*l))
            .cloned()
            .collect();
        let unexpected: Vec<_> = bodies
            .languages
            .keys()
            .filter(|l| !required.contains(l))
            .cloned()
            .collect();
        if !missing.is_empty() || !unexpected.is_empty() {
            errors.push(diagnostic(
                Some(unit),
                draft,
                Some(*target),
                None,
                AssemblyStage::Ownership,
                bodies.site.clone(),
                AssemblyInputFault::BodyLanguages {
                    target: *target,
                    required: required.to_vec(),
                    missing,
                    unexpected,
                },
            ));
        }
    }
}
fn parse_bodies(
    drafts: &[DisplayDraft],
    units: &DisplayUnits<'_>,
) -> Result<DisplayBodies, Vec<AssemblyInputDiagnostic>> {
    let mut checked = BTreeMap::new();
    let mut errors = Vec::new();
    for draft in drafts {
        let unit = units[draft.unit.value.as_str()];
        let mut bodies = BTreeMap::new();
        for (target, source) in &draft.bodies {
            for (language, source) in &source.languages {
                match parse_display_body(draft.kind, *target, source) {
                    Ok(body) => {
                        bodies.insert((*target, language.clone()), body);
                    }
                    Err(error) => {
                        let (site, fault) = *error;
                        errors.push(diagnostic(
                            Some(unit),
                            draft,
                            Some(*target),
                            Some(language),
                            AssemblyStage::DisplaySyntax,
                            site,
                            fault,
                        ));
                    }
                }
            }
        }
        checked.insert(draft.unit.value.clone(), bodies);
    }
    refuse(errors)?;
    Ok(checked)
}
#[allow(clippy::too_many_lines)] // Resolve all body families before globally permitting equality.
fn resolve_bodies(
    drafts: &[DisplayDraft],
    units: &DisplayUnits<'_>,
    checked: &DisplayBodies,
    welded: &WeldedUnits<'_, '_, '_, '_, '_, '_>,
) -> Result<DisplaySides, Vec<AssemblyInputDiagnostic>> {
    let mut errors = Vec::new();
    let mut all_sides = BTreeMap::new();
    for draft in drafts {
        let unit = units[draft.unit.value.as_str()];
        let mut sides = Vec::new();
        for ((target, language), body) in &checked[&draft.unit.value] {
            let mut tags: BTreeMap<String, BodyAddressMember> = BTreeMap::new();
            let mut citations: BTreeMap<String, BodyAddressMember> = BTreeMap::new();
            let mut span_index = 0;
            for region in &body.regions {
                for tag in region.tags() {
                    let site = region.site(tag.site());
                    match welded.integrated().resolved().book().resolve(tag.input()) {
                        Ok(crate::AddressTarget::Theory(id)) => tags
                            .entry(tag.input().into())
                            .or_insert_with(|| BodyAddressMember {
                                value: tag.input().into(),
                                target: Some(id),
                                sites: Vec::new(),
                            })
                            .sites
                            .push(site),
                        Ok(crate::AddressTarget::Documentary(label)) => errors.push(diagnostic(
                            Some(unit),
                            draft,
                            Some(*target),
                            Some(language),
                            AssemblyStage::DisplayResolution,
                            site,
                            AssemblyInputFault::DocumentaryTagTarget {
                                input: tag.input().into(),
                                label,
                            },
                        )),
                        Err(cause) => errors.push(diagnostic(
                            Some(unit),
                            draft,
                            Some(*target),
                            Some(language),
                            AssemblyStage::DisplayResolution,
                            site,
                            AssemblyInputFault::DisplayTag { cause },
                        )),
                    }
                }
                for math in region.math() {
                    if !unit.math().contains_key(math.name()) {
                        errors.push(diagnostic(
                            Some(unit),
                            draft,
                            Some(*target),
                            Some(language),
                            AssemblyStage::DisplayResolution,
                            region.site(math.site()),
                            AssemblyInputFault::DisplayMathReference {
                                name: math.name().into(),
                                available: unit.math().keys().cloned().collect(),
                            },
                        ));
                    }
                }
                for (label, local, citation) in region.spans() {
                    let site = region.site(&local);
                    if citation {
                        if welded.integrated().mints().contains_key(&label) {
                            citations
                                .entry(label.clone())
                                .or_insert_with(|| BodyAddressMember {
                                    value: label,
                                    target: None,
                                    sites: Vec::new(),
                                })
                                .sites
                                .push(site);
                        } else {
                            errors.push(diagnostic(
                                Some(unit),
                                draft,
                                Some(*target),
                                Some(language),
                                AssemblyStage::DisplayResolution,
                                site,
                                AssemblyInputFault::DisplayCitation {
                                    label,
                                    span_index,
                                    expected: 1,
                                    actual: 0,
                                },
                            ));
                        }
                    }
                    span_index += 1;
                }
            }
            sides.push((
                BodyAddressSide {
                    target: *target,
                    language: language.clone(),
                    site: body.source.site.clone(),
                    members: tags.into_values().collect(),
                },
                BodyAddressSide {
                    target: *target,
                    language: language.clone(),
                    site: body.source.site.clone(),
                    members: citations.into_values().collect(),
                },
            ));
        }
        all_sides.insert(draft.unit.value.clone(), sides);
    }
    refuse(errors)?;
    Ok(all_sides)
}
fn equality(
    drafts: &[DisplayDraft],
    units: &DisplayUnits<'_>,
    all_sides: &DisplaySides,
) -> Result<(), Vec<AssemblyInputDiagnostic>> {
    let mut errors = Vec::new();
    for draft in drafts {
        let unit = units[draft.unit.value.as_str()];
        let sides = &all_sides[&draft.unit.value];
        if let Some(reference) = sides.first() {
            for actual in &sides[1..] {
                for (family, reference, actual) in [
                    (AddressFamily::Tags, &reference.0, &actual.0),
                    (AddressFamily::Citations, &reference.1, &actual.1),
                ] {
                    let missing: Vec<_> = reference
                        .members
                        .iter()
                        .filter(|m| !actual.members.iter().any(|a| a.value == m.value))
                        .map(|m| m.value.clone())
                        .collect();
                    let unexpected: Vec<_> = actual
                        .members
                        .iter()
                        .filter(|m| !reference.members.iter().any(|a| a.value == m.value))
                        .map(|m| m.value.clone())
                        .collect();
                    if !missing.is_empty() || !unexpected.is_empty() {
                        errors.push(diagnostic(
                            Some(unit),
                            draft,
                            Some(actual.target),
                            Some(&actual.language),
                            AssemblyStage::DisplayEquality,
                            actual.site.clone(),
                            AssemblyInputFault::DisplaySetMismatch {
                                family,
                                reference: reference.clone(),
                                actual: actual.clone(),
                                missing,
                                unexpected,
                            },
                        ));
                    }
                }
            }
        }
    }
    refuse(errors)
}
