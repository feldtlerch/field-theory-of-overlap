use super::{
    diagnostics::{AssemblyFault, DocumentError},
    failure,
};
use ftoo_manifest::{AssemblySite, AssemblyStage, RuleId, UnitSite};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Entry {
    pub key: String,
    pub author: String,
    pub year: String,
    pub title: String,
    pub journal: String,
    pub volume: String,
    pub number: String,
    pub pages: String,
    pub doi: String,
    pub raw: String,
    pub label: String,
    pub anchor: String,
}

pub(super) const DERIVED_KEYS: [&str; 7] = [
    "a:sencapability",
    "a:senidentity",
    "a:senrational",
    "a:sensocialchoice",
    "a:fisher",
    "a:vmf",
    "a:multidim",
];

fn refuse(site: &UnitSite, key: &str, reason: &str) -> DocumentError {
    failure(
        RuleId::M055,
        AssemblyStage::Ownership,
        AssemblySite::Source(site.clone()),
        AssemblyFault::UnknownToken {
            value: key.into(),
            expected: vec![reason.into()],
        },
    )
}

fn braced(source: &str) -> Option<(&str, &str)> {
    if !source.starts_with('{') {
        return None;
    }
    let mut depth = 0usize;
    for (at, scalar) in source.char_indices() {
        if scalar == '{' {
            depth += 1;
        }
        if scalar == '}' {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some((&source[1..at], &source[at + 1..]));
            }
        }
    }
    None
}

fn fields(mut source: &str) -> Option<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    while !source.trim().is_empty() {
        source = source.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        if source.is_empty() {
            break;
        }
        let equal = source.find('=')?;
        let name = source[..equal].trim();
        if !matches!(
            name.to_ascii_lowercase().as_str(),
            "author" | "title" | "journal" | "volume" | "number" | "pages" | "year" | "doi"
        ) {
            return None;
        }
        source = source[equal + 1..].trim_start();
        let (value, tail) = braced(source)?;
        if value.contains('{') || value.contains('}') {
            return None;
        }
        if result
            .insert(
                name.to_ascii_lowercase(),
                value.split_whitespace().collect::<Vec<_>>().join(" "),
            )
            .is_some()
        {
            return None;
        }
        source = tail;
    }
    Some(result)
}

fn headers(source: &str) -> Option<Vec<(&str, &str, &str)>> {
    if source.contains('%') {
        return None;
    }
    let mut result = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find('@') {
        rest = &rest[at + 1..];
        let end = rest.find(|c: char| !c.is_ascii_alphabetic())?;
        let kind = &rest[..end];
        if kind.eq_ignore_ascii_case("comment") {
            return None;
        }
        let value = rest[end..].trim_start();
        if !value.starts_with('{') {
            return None;
        }
        let (body, tail) = braced(value)?;
        if !kind.eq_ignore_ascii_case("string") {
            let (key, _) = body.split_once(',')?;
            result.push((kind, key.trim(), value));
        }
        rest = tail;
    }
    Some(result)
}

fn field_from_body<'a>(body: &'a str, requested: &str) -> Option<&'a str> {
    let (content, _) = braced(body)?;
    let (_, mut rest) = content.split_once(',')?;
    while !rest.trim().is_empty() {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        let end = rest.find(|c: char| !c.is_ascii_alphabetic())?;
        let name = &rest[..end];
        rest = rest[end..].trim_start().strip_prefix('=')?.trim_start();
        let (value, tail) = braced(rest)?;
        if name.eq_ignore_ascii_case(requested) {
            return Some(value);
        }
        rest = tail;
    }
    None
}

fn author_year(body: &str) -> Option<(&str, &str)> {
    let author = field_from_body(body, "author")?;
    let year = field_from_body(body, "year")?;
    let surname = first_surname(author);
    if surname.contains('{')
        || surname.contains('}')
        || year.len() != 4
        || !year.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    Some((author, year))
}

fn first_surname(author: &str) -> &str {
    author.split_once(',').map_or_else(
        || {
            author
                .split_whitespace()
                .take_while(|word| !word.eq_ignore_ascii_case("and"))
                .last()
                .unwrap_or("")
        },
        |(last, _)| last.trim(),
    )
}

fn literal_fields(values: &[&str], pages: &str) -> bool {
    let markup = |value: &str| {
        value.chars().any(|c| {
            matches!(
                c,
                '\\' | '~' | '`' | '$' | '#' | '%' | '&' | '^' | '_' | '\'' | '"'
            )
        })
    };
    !values.iter().any(|value| {
        markup(value)
            || value.contains("--")
            || value.contains("<<")
            || value.contains(">>")
            || value.contains(",,")
    }) && !markup(pages)
        && !pages.contains("<<")
        && !pages.contains(">>")
        && !pages.contains(",,")
        && !pages.contains("---")
}

pub(super) fn resolve(source: &str, key: &str, site: &UnitSite) -> Result<Entry, DocumentError> {
    // bibroute.py changes these entries before the paper typesets them.
    if DERIVED_KEYS
        .iter()
        .any(|changed| changed.eq_ignore_ascii_case(key))
    {
        return Err(refuse(
            site,
            key,
            "entry changed by bibliography derivation",
        ));
    }
    let headers =
        headers(source).ok_or_else(|| refuse(site, key, "closed bibliography entries"))?;
    let selected: Vec<_> = headers
        .iter()
        .filter(|(_, candidate, _)| candidate.eq_ignore_ascii_case(key))
        .collect();
    if selected.len() != 1 || !selected[0].0.eq_ignore_ascii_case("article") {
        return Err(refuse(site, key, "one bibliography article entry"));
    }
    let entry = selected[0].2;
    let (body, _) = braced(entry).ok_or_else(|| refuse(site, key, "closed bibliography entry"))?;
    let (entry_key, content) = body
        .split_once(',')
        .ok_or_else(|| refuse(site, key, "matching bibliography entry key"))?;
    if !entry_key.eq_ignore_ascii_case(key) {
        return Err(refuse(site, key, "matching bibliography entry key"));
    }
    let values =
        fields(content).ok_or_else(|| refuse(site, key, "simple braced article fields"))?;
    let get = |field: &str| {
        values
            .get(field)
            .filter(|value| !value.is_empty())
            .cloned()
            .ok_or_else(|| refuse(site, key, field))
    };
    let author = get("author")?;
    if author
        .split_whitespace()
        .any(|word| word.eq_ignore_ascii_case("and"))
        || author.contains('\\')
    {
        return Err(refuse(site, key, "one literal author"));
    }
    let surname = author
        .split_once(',')
        .map_or(author.as_str(), |(last, _)| last)
        .trim();
    if surname.is_empty() || !surname.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(refuse(site, key, "one literal author surname"));
    }
    let year = get("year")?;
    if year.len() != 4 || !year.chars().all(|c| c.is_ascii_digit()) {
        return Err(refuse(site, key, "four-digit publication year"));
    }
    let title = get("title")?;
    let journal = get("journal")?;
    let volume = get("volume")?;
    let number = get("number")?;
    let pages = get("pages")?;
    let doi = get("doi")?;
    if !literal_fields(&[&author, &title, &journal, &volume, &number, &doi], &pages) {
        return Err(refuse(site, key, "literal bibliography fields"));
    }
    for (_, other_key, other_body) in &headers {
        if other_key.eq_ignore_ascii_case(key) {
            continue;
        }
        let (other_author, other_year) = author_year(other_body).ok_or_else(|| {
            refuse(
                site,
                key,
                &format!("parseable bibliography author and year for {other_key}"),
            )
        })?;
        if other_year == year && first_surname(other_author).eq_ignore_ascii_case(surname) {
            return Err(refuse(site, key, "unique bibliography surname and year"));
        }
    }
    let raw = format!("@article{{{body}}}");
    let label = format!("{surname} {year}");
    let anchor = format!("bibliography-{}-{year}", surname.to_ascii_lowercase());
    Ok(Entry {
        key: key.into(),
        author,
        year,
        title,
        journal,
        volume,
        number,
        pages: pages.replace("--", "–"),
        doi,
        raw,
        label,
        anchor,
    })
}

pub(super) fn screen_text(entry: &Entry) -> String {
    format!(
        "{}, {}. {}. {} {}({}), {}. DOI: {}.",
        entry.author,
        entry.year,
        entry.title,
        entry.journal,
        entry.volume,
        entry.number,
        entry.pages,
        entry.doi
    )
}

pub(super) fn citation_label(
    kind: ftoo_manifest::BibliographyCitationKind,
    entries: &[Entry],
) -> String {
    let joined = entries
        .iter()
        .map(|entry| entry.label.as_str())
        .collect::<Vec<_>>()
        .join("; ");
    match kind {
        ftoo_manifest::BibliographyCitationKind::Parenthetical => format!("({joined})"),
        ftoo_manifest::BibliographyCitationKind::Textual => entries
            .iter()
            .map(|entry| entry.label.replace(' ', " (") + ")")
            .collect::<Vec<_>>()
            .join("; "),
    }
}

pub(super) fn entries(plan: &super::plan::DocumentPlan) -> Vec<Entry> {
    fn walk(
        runs: &[super::plan::Inline],
        seen: &mut std::collections::BTreeSet<String>,
        output: &mut Vec<Entry>,
    ) {
        for run in runs {
            match run {
                super::plan::Inline::BibliographyCitation { entries, .. } => {
                    for entry in entries {
                        if seen.insert(entry.key.clone()) {
                            output.push(entry.clone());
                        }
                    }
                }
                super::plan::Inline::Styled { children, .. } => walk(children, seen, output),
                _ => {}
            }
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut output = Vec::new();
    for block in &plan.blocks {
        walk(&block.runs, &mut seen, &mut output);
    }
    output
}

pub(super) fn recover_screen(source: &str, entries: &[Entry]) -> bool {
    let Some(body) = source.strip_suffix('\n') else {
        return false;
    };
    let rows: Vec<_> = body.split("\n\n").collect();
    if rows.len() != entries.len() {
        return false;
    }
    rows.into_iter().zip(entries).all(|(row, entry)| {
        let Some(rest) = row.strip_prefix("<a id=\"") else {
            return false;
        };
        let Some((anchor, rest)) = rest.split_once("\"></a>") else {
            return false;
        };
        let Some((text, key)) = rest.split_once("<!-- bibentry ") else {
            return false;
        };
        let Some(key) = key.strip_suffix(" -->") else {
            return false;
        };
        anchor == entry.anchor
            && key == entry.key
            && text == super::markdown::text(&screen_text(entry))
    })
}
