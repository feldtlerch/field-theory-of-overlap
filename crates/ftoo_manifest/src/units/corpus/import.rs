use super::{
    AccountOwner, AccountPiece, BlockDraft, BlockFields, BlockKind, BlockStatus, EventDraft,
    ImportFault, MathDraft, Refusal, Run, Site, SourceDraft, TableCell, TargetDraft,
};

fn site(path: &str, text: &str, start: usize, end: usize) -> Site {
    let last_line = if end > 0 && text.as_bytes()[end - 1] == b'\n' {
        &text[..end - 1]
    } else {
        &text[..end]
    };
    Site {
        path: path.into(),
        start_byte: start,
        end_byte: end,
        start_line: 1 + text[..start].matches('\n').count(),
        end_line: 1 + last_line.matches('\n').count(),
    }
}

fn braced(text: &str, start: usize) -> Option<(&str, usize)> {
    if text.as_bytes().get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 0usize;
    let mut escaped = false;
    for (offset, byte) in text.as_bytes()[start..].iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if *byte == b'\\' {
            escaped = true;
            continue;
        }
        if *byte == b'{' {
            depth += 1;
        }
        if *byte == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some((&text[start + 1..start + offset], start + offset + 1));
            }
        }
    }
    None
}

fn command_at(text: &str, start: usize) -> (&str, usize) {
    let bytes = text.as_bytes();
    let mut end = start + 1;
    if bytes.get(end).is_some_and(u8::is_ascii_alphabetic) {
        while bytes.get(end).is_some_and(u8::is_ascii_alphabetic) {
            end += 1;
        }
    } else if end < bytes.len() {
        end += text[end..].chars().next().map_or(0, char::len_utf8);
    }
    (&text[start..end], end)
}

type InlineResult = Result<(Vec<Run>, Vec<TargetDraft>), (usize, usize, String)>;

#[allow(clippy::too_many_lines)] // Closed inline cases retain each source boundary and refusal.
fn inline(path: &str, whole: &str, source: &str, base: usize) -> InlineResult {
    let mut runs = Vec::new();
    let mut targets = Vec::new();
    let mut plain = String::new();
    let mut cursor = 0usize;
    while cursor < source.len() {
        let byte = source.as_bytes()[cursor];
        if byte == b'%' {
            return Err((
                base + cursor,
                base + cursor + 1,
                "comment in content".into(),
            ));
        }
        if byte == b'$' {
            let end = source[cursor + 1..]
                .find('$')
                .map_or(source.len(), |offset| cursor + offset + 2);
            return Err((base + cursor, base + end, "unresolved mathematics".into()));
        }
        if byte == b'{' || byte == b'}' {
            return Err((
                base + cursor,
                base + cursor + 1,
                "unresolved grouping".into(),
            ));
        }
        if byte != b'\\' {
            let ch = source[cursor..]
                .chars()
                .next()
                .expect("cursor inside source");
            plain.push(ch);
            cursor += ch.len_utf8();
            continue;
        }
        let (command, end) = command_at(source, cursor);
        if command == "\\%" || command == "\\#" || command == "\\&" || command == "\\_" {
            plain.push(command.chars().nth(1).expect("escaped command"));
            cursor = end;
            continue;
        }
        if !matches!(
            command,
            "\\emph"
                | "\\textbf"
                | "\\textit"
                | "\\textsc"
                | "\\ftag"
                | "\\ptag"
                | "\\handle"
                | "\\ref"
                | "\\cref"
                | "\\ptg"
                | "\\parencite"
                | "\\textcite"
        ) {
            return Err((
                base + cursor,
                base + end,
                format!("unsupported command {command}"),
            ));
        }
        if !plain.is_empty() {
            let decoded = std::mem::take(&mut plain)
                .replace("---", "—")
                .replace("--", "–")
                .replace("``", "“")
                .replace("''", "”");
            runs.push(Run::Text {
                decoded,
                spacing_intent: "source".into(),
            });
        }
        let star = source.as_bytes().get(end) == Some(&b'*');
        let argument_start = end + usize::from(star);
        if source.as_bytes().get(argument_start) == Some(&b'[') {
            return Err((
                base + cursor,
                base + argument_start + 1,
                format!("unsupported optional argument of {command}"),
            ));
        }
        if source.as_bytes().get(argument_start) != Some(&b'{') {
            return Err((
                base + cursor,
                base + argument_start.min(source.len()),
                format!("missing argument of {command}"),
            ));
        }
        let Some((argument, next)) = braced(source, argument_start) else {
            return Err((
                base + cursor,
                base + source.len(),
                format!("unbalanced argument of {command}"),
            ));
        };
        let run = match command {
            "\\emph" | "\\textbf" | "\\textit" | "\\textsc" => {
                let (children, nested) = inline(path, whole, argument, base + argument_start + 1)?;
                targets.extend(nested);
                Run::Style {
                    style: command[1..].into(),
                    runs: children,
                }
            }
            "\\ftag" | "\\ptag" => {
                targets.push(TargetDraft {
                    selector: argument.into(),
                    exact_key: argument.into(),
                    source_command: format!("{command}{}", if star { "*" } else { "" }),
                    site: site(path, whole, base + cursor, base + next),
                    occurrence: 0,
                });
                Run::Tag {
                    command: command.into(),
                    star,
                    key: argument.into(),
                }
            }
            "\\handle" => {
                targets.push(TargetDraft {
                    selector: argument.into(),
                    exact_key: argument.into(),
                    source_command: format!("{command}{}", if star { "*" } else { "" }),
                    site: site(path, whole, base + cursor, base + next),
                    occurrence: 0,
                });
                Run::Handle {
                    star,
                    selector: argument.into(),
                }
            }
            "\\ref" | "\\cref" | "\\ptg" => Run::Reference {
                command: command.into(),
                star,
                key: argument.into(),
            },
            "\\parencite" | "\\textcite" => Run::Citation {
                command: command.into(),
                ordered_keys: argument
                    .split(',')
                    .map(str::trim)
                    .map(str::to_owned)
                    .collect(),
            },
            _ => unreachable!("closed command set was checked"),
        };
        runs.push(run);
        cursor = next;
    }
    if !plain.is_empty() {
        let decoded = plain
            .replace("---", "—")
            .replace("--", "–")
            .replace("``", "“")
            .replace("''", "”");
        runs.push(Run::Text {
            decoded,
            spacing_intent: "source".into(),
        });
    }
    Ok((runs, targets))
}

fn environment_end(text: &str, start: usize) -> Option<(usize, String)> {
    let mut stack: Vec<String> = Vec::new();
    let mut cursor = start;
    while cursor < text.len() {
        let byte = text.as_bytes()[cursor];
        if byte == b'%' {
            cursor = text[cursor..]
                .find('\n')
                .map_or(text.len(), |index| cursor + index + 1);
            continue;
        }
        if byte != b'\\' {
            cursor += 1;
            continue;
        }
        let (command, end) = command_at(text, cursor);
        if command == "\\begin" || command == "\\end" {
            let (name, next) = braced(text, end)?;
            if command == "\\begin" {
                stack.push(name.into());
            } else if stack.pop().as_deref() != Some(name) {
                return None;
            }
            if stack.is_empty() {
                return Some((next, name.into()));
            }
            cursor = next;
        } else {
            cursor = end;
        }
    }
    None
}

fn environment_kind(name: &str) -> BlockKind {
    match name {
        "equation" | "equation*" | "align" | "align*" | "gather" => BlockKind::Display,
        "itemize" | "enumerate" | "description" => BlockKind::List,
        "tabular" | "tabularx" | "longtable" | "table" => BlockKind::Table,
        "theorem" | "lemma" | "proposition" | "proof" | "remark" | "corollary" | "definition"
        | "postulate" | "insight" => BlockKind::TheoremContainer,
        "econ" => BlockKind::EconomicsBox,
        "epist" => BlockKind::EpistemologyBox,
        "statement" | "statementbox" => BlockKind::StatementBox,
        "quote" | "quotation" => BlockKind::Quotation,
        "abstract" => BlockKind::Abstract,
        _ => BlockKind::Refusal,
    }
}

fn table_fields(text: &str, start: usize, closed: usize) -> Option<BlockFields> {
    let header = "\\begin{tabular}";
    let begin = start + header.len();
    let (columns, body_start) = braced(text, begin)?;
    let ending = "\\end{tabular}";
    let body_end = closed.checked_sub(ending.len())?;
    if body_end < body_start || !text[body_end..closed].starts_with(ending) {
        return None;
    }
    let body = &text[body_start..body_end];
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut depth = 0usize;
    let mut cursor = 0usize;
    while cursor < body.len() {
        let byte = body.as_bytes()[cursor];
        if byte == b'{' {
            depth += 1;
            cursor += 1;
            continue;
        }
        if byte == b'}' {
            depth = depth.checked_sub(1)?;
            cursor += 1;
            continue;
        }
        if byte == b'\\' && body.as_bytes().get(cursor + 1) == Some(&b'\\') && depth == 0 {
            row.push(TableCell {
                text: cell.trim().into(),
                row_span: 1,
                column_span: 1,
                header: false,
            });
            rows.push(std::mem::take(&mut row));
            cell.clear();
            cursor += 2;
            continue;
        }
        if byte == b'\\' || byte == b'%' || byte == b'$' {
            return None;
        }
        if byte == b'&' && depth == 0 {
            row.push(TableCell {
                text: cell.trim().into(),
                row_span: 1,
                column_span: 1,
                header: false,
            });
            cell.clear();
            cursor += 1;
            continue;
        }
        let ch = body[cursor..].chars().next()?;
        cell.push(ch);
        cursor += ch.len_utf8();
    }
    if depth != 0 {
        return None;
    }
    if !cell.trim().is_empty() || !row.is_empty() {
        row.push(TableCell {
            text: cell.trim().into(),
            row_span: 1,
            column_span: 1,
            header: false,
        });
        rows.push(row);
    }
    if rows.is_empty() || rows.iter().any(|row| row.len() != columns.chars().count()) {
        return None;
    }
    Some(BlockFields::Table {
        columns: columns.into(),
        rows,
        caption: None,
    })
}

#[allow(clippy::collapsible_if)] // Each nested boundary has a distinct failure condition.
fn container_children(path: &str, text: &str, start: usize, end: usize) -> Vec<BlockDraft> {
    let Some(open_line) = text[start..end].find('\n').map(|offset| start + offset + 1) else {
        return Vec::new();
    };
    let Some(close_line) = text[open_line..end]
        .rfind("\\end{")
        .map(|offset| open_line + offset)
    else {
        return Vec::new();
    };
    let mut children = Vec::new();
    let mut cursor = open_line;
    let mut after_display = false;
    while cursor < close_line {
        let line_end = text[cursor..close_line]
            .find('\n')
            .map_or(close_line, |offset| cursor + offset + 1);
        let fragment = text[cursor..line_end].trim();
        if fragment.is_empty() {
            after_display = false;
            cursor = line_end;
            continue;
        }
        if fragment.starts_with("\\begin{") {
            if let Some((closed, name)) = environment_end(text, cursor) {
                if closed <= close_line {
                    let child_end = text[closed..close_line]
                        .find('\n')
                        .map_or(close_line, |offset| closed + offset + 1);
                    children.push(BlockDraft {
                        local_index: children.len(),
                        site: site(path, text, cursor, child_end),
                        kind: environment_kind(&name),
                        fields: BlockFields::Unresolved {
                            production: name.clone(),
                        },
                        children: Vec::new(),
                        events: Vec::new(),
                        status: BlockStatus::Unresolved,
                    });
                    after_display = matches!(environment_kind(&name), BlockKind::Display);
                    cursor = child_end;
                    continue;
                }
            }
        }
        let fields = match inline(path, text, fragment, cursor) {
            Ok((runs, _)) => BlockFields::Paragraph {
                continuation: after_display,
                layout: Vec::new(),
                runs,
            },
            Err(_) => BlockFields::Unresolved {
                production: "paragraph".into(),
            },
        };
        let status = if matches!(fields, BlockFields::Unresolved { .. }) {
            BlockStatus::Unresolved
        } else {
            BlockStatus::Decoded
        };
        children.push(BlockDraft {
            local_index: children.len(),
            site: site(path, text, cursor, line_end),
            kind: BlockKind::Paragraph,
            fields,
            children: Vec::new(),
            events: Vec::new(),
            status,
        });
        after_display = false;
        cursor = line_end;
    }
    children
}

fn heading(text: &str) -> Option<(&str, bool, &str, usize)> {
    for command in [
        "\\part",
        "\\chapter",
        "\\section",
        "\\subsection",
        "\\subsubsection",
        "\\appendix",
        "\\coda",
    ] {
        if text.starts_with(command) && command_at(text, 0).0 == command {
            let star = text.as_bytes().get(command.len()) == Some(&b'*');
            let start = command.len() + usize::from(star);
            if let Some((title, end)) = braced(text, start) {
                return Some((command, star, title, end));
            }
        }
    }
    None
}

fn boundary_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let command = trimmed.starts_with('\\').then(|| command_at(trimmed, 0).0);
    trimmed.trim().is_empty()
        || trimmed.starts_with('%')
        || trimmed.starts_with("\\begin{")
        || trimmed.starts_with("\\end{")
        || heading(trimmed).is_some()
        || [
            "\\documentclass",
            "\\end{document}",
            "\\PaperParagraph",
            "\\PaperDisplayParagraph",
            "\\secgrade",
            "\\grade",
            "\\spend",
            "\\grant",
            "\\DeclareQuantity",
            "\\label",
            "\\vspace",
            "\\noindent",
            "\\par",
            "\\tableofcontents",
            "\\printbibliography",
            "\\ftooindex",
            "\\clearpage",
            "\\newpage",
            "\\pagebreak",
            "\\nopagebreak",
            "\\medskip",
            "\\bigskip",
            "\\smallskip",
            "\\begingroup",
            "\\endgroup",
            "\\appendix",
            "\\nobreak",
        ]
        .iter()
        .any(|name| Some(*name) == command)
}

fn opening_hook(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('\\')
        && matches!(
            command_at(trimmed, 0).0,
            "\\PaperParagraph"
                | "\\noindent"
                | "\\label"
                | "\\secgrade"
                | "\\grade"
                | "\\spend"
                | "\\grant"
        )
}

fn admitted_layout(value: &str) -> bool {
    value.split(',').all(|part| {
        matches!(
            part,
            "lastline" | "wholewords" | "stretch=2em" | "stretch=4em" | "rightskip=0pt plus 1em"
        )
    })
}

fn admitted_display_layout(value: &str) -> bool {
    let Some(rest) = value.trim().strip_prefix("\\PaperDisplayParagraph") else {
        return false;
    };
    let Some((kind, after_kind)) = braced(rest, 0) else {
        return false;
    };
    let Some((options, after_options)) = braced(rest, after_kind) else {
        return false;
    };
    matches!(kind, "equation" | "align")
        && admitted_layout(options)
        && rest[after_options..].trim().is_empty()
}

struct Importer<'a> {
    path: &'a str,
    text: &'a str,
    markers: &'a [(String, String)],
    draft: SourceDraft,
    advance: Option<usize>,
    pending_display: bool,
}
impl Importer<'_> {
    fn targets(&mut self, targets: Vec<TargetDraft>) {
        for mut target in targets {
            target.occurrence = self.draft.metadata_targets.len();
            self.draft.metadata_targets.push(target);
        }
    }
    fn event(&mut self, start: usize, end: usize, kind: &str) {
        let index = self.draft.events.len();
        self.draft.events.push(EventDraft {
            kind: kind.into(),
            site: site(self.path, self.text, start, end),
            source: self.text[start..end].into(),
        });
        self.draft.accounting.push(AccountPiece {
            start_byte: start,
            end_byte: end,
            owner: AccountOwner::Event(index),
        });
    }
    fn block(
        &mut self,
        start: usize,
        end: usize,
        kind: BlockKind,
        fields: BlockFields,
        native: bool,
    ) {
        let index = self.draft.blocks.len();
        self.draft.blocks.push(BlockDraft {
            local_index: index,
            site: site(self.path, self.text, start, end),
            kind,
            fields,
            children: Vec::new(),
            events: Vec::new(),
            status: BlockStatus::Decoded,
        });
        let owner = if native {
            AccountOwner::ExistingNative(index)
        } else {
            AccountOwner::Content(index)
        };
        self.draft.accounting.push(AccountPiece {
            start_byte: start,
            end_byte: end,
            owner,
        });
    }
    #[allow(clippy::too_many_arguments)] // Accounted range and fault range are independent.
    fn refuse(
        &mut self,
        start: usize,
        end: usize,
        fault_start: usize,
        fault_end: usize,
        kind: BlockKind,
        command: &str,
        reason: &str,
    ) {
        let mut end = end;
        let mut fault_end = fault_end;
        let mut reason = reason.to_owned();
        if reason.starts_with("unbalanced argument") {
            let open = self.text[fault_start..]
                .find('{')
                .map(|offset| fault_start + offset);
            if let Some((_, closed)) = open.and_then(|open| braced(self.text, open)) {
                fault_end = closed;
                if closed > end {
                    end = self.text[closed..]
                        .find('\n')
                        .map_or(self.text.len(), |offset| closed + offset + 1);
                    reason = "argument crosses block boundary".into();
                }
            } else {
                end = self.text.len();
                fault_end = end;
            }
            self.advance = Some(end);
        }
        if reason == "unresolved mathematics" {
            let index = self.draft.math.len();
            self.draft.math.push(MathDraft {
                local_key: format!("unresolved-{index}"),
                source: self.text[fault_start..fault_end].into(),
                site: site(self.path, self.text, fault_start, fault_end),
                parse_status: "unresolved".into(),
                dependencies: Vec::new(),
                refusals: vec![reason.clone()],
            });
        }
        let index = self.draft.refusals.len();
        self.draft.refusals.push(Refusal {
            code: reason.clone(),
            site: site(self.path, self.text, fault_start, fault_end),
            block_kind: kind.clone(),
            command_or_environment: command.into(),
            reason,
        });
        let block_index = self.draft.blocks.len();
        let children = if matches!(
            kind,
            BlockKind::TheoremContainer
                | BlockKind::StatementBox
                | BlockKind::EconomicsBox
                | BlockKind::EpistemologyBox
        ) {
            container_children(self.path, self.text, start, end)
        } else {
            Vec::new()
        };
        self.draft.blocks.push(BlockDraft {
            local_index: block_index,
            site: site(self.path, self.text, start, end),
            kind,
            fields: BlockFields::Unresolved {
                production: command.into(),
            },
            children,
            events: Vec::new(),
            status: BlockStatus::Unresolved,
        });
        self.draft.accounting.push(AccountPiece {
            start_byte: start,
            end_byte: end,
            owner: AccountOwner::Refusal(index),
        });
    }
    #[allow(clippy::too_many_lines, clippy::collapsible_if)] // One located source line has closed lexical cases.
    fn line(&mut self, start: usize, end: usize) {
        let raw = &self.text[start..end];
        let trimmed = raw.trim_start();
        let prefix = raw.len() - trimmed.len();
        if prefix > 0 {
            self.event(start, start + prefix, "lexical-trivia");
        }
        let body_start = start + prefix;
        if body_start == end {
            return;
        }
        let body = &self.text[body_start..end];
        if body.starts_with('%') {
            let first_end = body
                .find('\n')
                .map_or(end, |offset| body_start + offset + 1);
            if let Some((unit, role)) = self.markers.iter().find(|(unit, _)| {
                prefix == 0
                    && self.text[body_start..first_end]
                        .strip_suffix('\n')
                        .unwrap_or(&self.text[body_start..first_end])
                        == format!("% generated: {unit}")
            }) {
                self.block(
                    body_start,
                    first_end,
                    BlockKind::ExistingNativeInclusion,
                    BlockFields::NativeInclusion {
                        marker: self.text[body_start..first_end]
                            .trim_end_matches('\n')
                            .into(),
                        unit: unit.clone(),
                        role: role.clone(),
                    },
                    true,
                );
            } else if body.starts_with("% generated: ") {
                self.refuse(
                    body_start,
                    first_end,
                    body_start,
                    first_end,
                    BlockKind::ExistingNativeInclusion,
                    "% generated:",
                    "unowned generated marker",
                );
            } else {
                self.event(body_start, first_end, "lexical-trivia");
            }
            if first_end < end {
                self.line(first_end, end);
            }
            return;
        }
        if body.starts_with("\\documentclass") || body.starts_with("\\end{document}") {
            self.event(body_start, end, "document-wrapper");
            return;
        }
        if let Some(rest) = body.strip_prefix("\\PaperParagraph") {
            if let Some((options, end_option)) = braced(rest, 0) {
                if admitted_layout(options) {
                    let consumed = "\\PaperParagraph".len() + end_option;
                    self.event(body_start, body_start + consumed, "layout-instruction");
                    if body_start + consumed < end {
                        self.line(body_start + consumed, end);
                    }
                    return;
                }
            }
            self.refuse(
                body_start,
                end,
                body_start,
                end,
                BlockKind::Paragraph,
                "\\PaperParagraph",
                "unsupported layout option",
            );
            return;
        }
        if let Some((level, star, title, consumed)) = heading(body) {
            if body[consumed..].trim().is_empty() {
                self.pending_display = false;
                let title_start = body_start + consumed - title.len() - 1;
                match inline(self.path, self.text, title, title_start) {
                    Ok((runs, targets)) => {
                        self.targets(targets);
                        self.block(
                            body_start,
                            end,
                            BlockKind::Heading,
                            BlockFields::Heading {
                                level: level.into(),
                                star,
                                title: runs,
                                label_selector: None,
                            },
                            false,
                        );
                    }
                    Err((a, b, reason)) => {
                        self.refuse(body_start, end, a, b, BlockKind::Heading, level, &reason);
                    }
                }
                return;
            }
        }
        if body.starts_with("\\begin{") {
            self.refuse(
                body_start,
                end,
                body_start,
                end,
                BlockKind::Refusal,
                "\\begin",
                "unbalanced environment",
            );
            return;
        }
        if body.starts_with('\\') {
            let (command, command_end) = command_at(body, 0);
            if command == "\\label" {
                if let Some((key, consumed)) = braced(body, command_end) {
                    self.targets(vec![TargetDraft {
                        selector: key.into(),
                        exact_key: key.into(),
                        source_command: command.into(),
                        site: site(self.path, self.text, body_start, body_start + consumed),
                        occurrence: 0,
                    }]);
                    self.event(body_start, body_start + consumed, "label-event");
                    if body_start + consumed < end {
                        self.line(body_start + consumed, end);
                    }
                } else {
                    self.refuse(
                        body_start,
                        end,
                        body_start,
                        end,
                        BlockKind::Refusal,
                        command,
                        "malformed label",
                    );
                }
                return;
            }
            if [
                "\\secgrade",
                "\\grade",
                "\\spend",
                "\\grant",
                "\\DeclareQuantity",
                "\\vspace",
                "\\noindent",
                "\\par",
                "\\tableofcontents",
                "\\printbibliography",
                "\\ftooindex",
                "\\clearpage",
                "\\newpage",
                "\\pagebreak",
                "\\nopagebreak",
                "\\medskip",
                "\\bigskip",
                "\\smallskip",
                "\\begingroup",
                "\\endgroup",
                "\\appendix",
                "\\nobreak",
            ]
            .contains(&command)
            {
                let consumed = if body.as_bytes().get(command_end) == Some(&b'{') {
                    braced(body, command_end).map_or(body.len(), |(_, next)| next)
                } else {
                    command_end
                };
                self.event(
                    body_start,
                    body_start + consumed,
                    if command == "\\appendix" {
                        "appendix-event"
                    } else if command.contains("grade")
                        || command.contains("spend")
                        || command.contains("grant")
                    {
                        "checking-effect"
                    } else {
                        "source-event"
                    },
                );
                if body_start + consumed < end {
                    self.line(body_start + consumed, end);
                }
                return;
            }
        }
        match inline(self.path, self.text, body, body_start) {
            Ok((runs, targets)) => {
                self.targets(targets);
                let continuation = std::mem::take(&mut self.pending_display);
                self.block(
                    body_start,
                    end,
                    BlockKind::Paragraph,
                    BlockFields::Paragraph {
                        continuation,
                        layout: Vec::new(),
                        runs,
                    },
                    false,
                );
            }
            Err((a, b, reason)) => {
                self.pending_display = false;
                self.refuse(
                    body_start,
                    end,
                    a,
                    b,
                    BlockKind::Paragraph,
                    &body[a - body_start..b - body_start],
                    &reason,
                );
            }
        }
    }
}

/// Decode bounded forms and account for all supplied bytes without admitting them.
/// # Errors
/// Refuses invalid UTF-8 or an invalid independent accounting partition.
/// # Panics
/// Panics only if a line found to contain an environment opener loses that same opener before indexing, which supplied bytes cannot arrange.
#[allow(clippy::too_many_lines, clippy::single_match_else)] // The source-order pass closes each block before accounting it.
pub fn import_source(
    path: &str,
    bytes: &[u8],
    digest: String,
    markers: &[(String, String)],
) -> Result<SourceDraft, ImportFault> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| ImportFault::Encoding { path: path.into() })?;
    let mut importer = Importer {
        path,
        text,
        markers,
        draft: SourceDraft {
            path: path.into(),
            digest,
            document: if path.contains("/explainer/") {
                "explainer"
            } else {
                "paper"
            }
            .into(),
            language: "en".into(),
            proposed_unit_id: None,
            metadata_targets: Vec::new(),
            math: Vec::new(),
            blocks: Vec::new(),
            events: Vec::new(),
            accounting: Vec::new(),
            refusals: Vec::new(),
            byte_length: bytes.len(),
        },
        advance: None,
        pending_display: false,
    };
    let mut cursor = 0usize;
    while cursor < text.len() {
        let line_end = text[cursor..]
            .find('\n')
            .map_or(text.len(), |offset| cursor + offset + 1);
        let line = &text[cursor..line_end];
        let trimmed = line.trim_start();
        if trimmed.trim().is_empty() {
            importer.pending_display = false;
        }
        if trimmed.starts_with("\\begin{document}") {
            importer.event(cursor, line_end, "document-wrapper");
            cursor = line_end;
        } else if trimmed.starts_with("\\PaperDisplayParagraph") && trimmed.contains("\\begin{") {
            let begin = cursor + line.find("\\begin{").expect("checked line contains begin");
            if admitted_display_layout(&text[cursor..begin]) {
                importer.event(cursor, begin, "layout-instruction");
            } else {
                importer.refuse(
                    cursor,
                    begin,
                    cursor,
                    begin,
                    BlockKind::Display,
                    "\\PaperDisplayParagraph",
                    "unsupported layout option",
                );
            }
            if let Some((closed, name)) = environment_end(text, begin) {
                let end = text[closed..]
                    .find('\n')
                    .map_or(text.len(), |offset| closed + offset + 1);
                importer.refuse(
                    begin,
                    end,
                    begin,
                    closed,
                    environment_kind(&name),
                    &name,
                    &format!("unsupported environment {name}"),
                );
                importer.pending_display = matches!(environment_kind(&name), BlockKind::Display);
                cursor = end;
            } else {
                importer.refuse(
                    begin,
                    text.len(),
                    begin,
                    text.len(),
                    BlockKind::Refusal,
                    "\\begin",
                    "unbalanced environment",
                );
                cursor = text.len();
            }
        } else if trimmed.starts_with("\\begin{") {
            let begin = cursor + line.len() - trimmed.len();
            if begin > cursor {
                importer.event(cursor, begin, "lexical-trivia");
            }
            match environment_end(text, begin) {
                Some((closed, name)) => {
                    let end = text[closed..]
                        .find('\n')
                        .map_or(text.len(), |offset| closed + offset + 1);
                    if name == "tabular" {
                        if text[closed..end].trim().is_empty()
                            && let Some(fields) = table_fields(text, begin, closed)
                        {
                            importer.block(begin, end, BlockKind::Table, fields, false);
                        } else {
                            importer.refuse(
                                begin,
                                end,
                                begin,
                                closed,
                                BlockKind::Table,
                                &name,
                                &format!("unsupported environment {name}"),
                            );
                        }
                    } else {
                        importer.refuse(
                            begin,
                            end,
                            begin,
                            closed,
                            environment_kind(&name),
                            &name,
                            &format!("unsupported environment {name}"),
                        );
                    }
                    importer.pending_display =
                        matches!(environment_kind(&name), BlockKind::Display);
                    cursor = end;
                }
                None => {
                    importer.refuse(
                        begin,
                        text.len(),
                        begin,
                        text.len(),
                        BlockKind::Refusal,
                        "\\begin",
                        "unbalanced environment",
                    );
                    cursor = text.len();
                }
            }
        } else {
            let mut end = line_end;
            if !boundary_line(line) || opening_hook(line) {
                while end < text.len() {
                    let next_end = text[end..]
                        .find('\n')
                        .map_or(text.len(), |offset| end + offset + 1);
                    if boundary_line(&text[end..next_end]) {
                        break;
                    }
                    end = next_end;
                }
            }
            importer.line(cursor, end);
            cursor = importer.advance.take().unwrap_or(end);
        }
    }
    validate_accounting(&importer.draft, bytes)?;
    Ok(importer.draft)
}

/// Verify the byte partition independently of the importer that produced it.
/// # Errors
/// Refuses holes, overlaps, split UTF-8 boundaries and unknown owners.
pub fn validate_accounting(draft: &SourceDraft, bytes: &[u8]) -> Result<(), ImportFault> {
    let text = std::str::from_utf8(bytes).map_err(|_| ImportFault::Encoding {
        path: draft.path.clone(),
    })?;
    let mut cursor = 0usize;
    for piece in &draft.accounting {
        let reason = if piece.start_byte > cursor {
            Some("hole")
        } else if piece.start_byte < cursor {
            Some("overlap")
        } else if piece.end_byte <= piece.start_byte || piece.end_byte > bytes.len() {
            Some("range")
        } else if !text.is_char_boundary(piece.start_byte) || !text.is_char_boundary(piece.end_byte)
        {
            Some("unicode boundary")
        } else {
            match piece.owner {
                AccountOwner::Content(index) | AccountOwner::ExistingNative(index)
                    if index >= draft.blocks.len() =>
                {
                    Some("missing owner")
                }
                AccountOwner::Event(index) if index >= draft.events.len() => Some("missing owner"),
                AccountOwner::Refusal(index) if index >= draft.refusals.len() => {
                    Some("missing owner")
                }
                _ => None,
            }
        };
        if let Some(reason) = reason {
            return Err(ImportFault::Accounting {
                path: draft.path.clone(),
                at: cursor,
                reason,
            });
        }
        cursor = piece.end_byte;
    }
    if cursor != bytes.len() {
        return Err(ImportFault::Accounting {
            path: draft.path.clone(),
            at: cursor,
            reason: "hole",
        });
    }
    Ok(())
}

/// Confirm that the bytes used for a diagnostic draft are still the bytes at its consumer.
/// # Errors
/// Refuses any changed byte, including a changed or missing suffix.
pub fn ensure_unchanged(path: &str, captured: &[u8], current: &[u8]) -> Result<(), ImportFault> {
    if captured == current {
        Ok(())
    } else {
        Err(ImportFault::InputChanged { path: path.into() })
    }
}
