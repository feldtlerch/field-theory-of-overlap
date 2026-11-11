use super::plan::DisplayBody;
use ftoo_manifest::{DisplayLayout, DocumentTarget, PictureCommand};

pub(super) enum Part {
    Structure(String),
    Region(usize),
}
pub(super) fn parts(body: &DisplayBody, target: DocumentTarget) -> Vec<Part> {
    let mut parts = Vec::new();
    match &body.layout {
        DisplayLayout::Table {
            columns,
            rows,
            rules,
        } => {
            if target == DocumentTarget::Latex {
                parts.push(Part::Structure(format!(
                    "\\begin{{center}}\n\\begin{{tabular}}{{{}}}\n",
                    columns.concat()
                )));
            }
            for (row, cells) in rows.iter().enumerate() {
                if target == DocumentTarget::Markdown {
                    parts.push(Part::Structure("| ".into()));
                } else if rules.contains(&row) {
                    parts.push(Part::Structure("\\hline\n".into()));
                }
                for (column, region) in cells.iter().enumerate() {
                    if column > 0 {
                        parts.push(Part::Structure(if target == DocumentTarget::Markdown {
                            " | ".into()
                        } else {
                            " & ".into()
                        }));
                    }
                    parts.push(Part::Region(*region));
                }
                parts.push(Part::Structure(if target == DocumentTarget::Markdown {
                    " |\n".into()
                } else {
                    " \\\\\n".into()
                }));
                if target == DocumentTarget::Markdown && row == 0 {
                    parts.push(Part::Structure(format!("| {} |\n", columns.join(" | "))));
                }
            }
            if target == DocumentTarget::Latex {
                parts.push(Part::Structure("\\end{tabular}\n\\end{center}\n".into()));
            }
        }
        DisplayLayout::Quote { rows } => {
            for region in rows {
                if let Some(region) = region {
                    parts.push(Part::Structure("> ".into()));
                    parts.push(Part::Region(*region));
                } else {
                    parts.push(Part::Structure(">".into()));
                }
                parts.push(Part::Structure("\n".into()));
            }
        }
        DisplayLayout::Picture {
            width,
            height,
            commands,
        } => {
            parts.push(Part::Structure(format!("\\begingroup\\setlength{{\\unitlength}}{{1pt}}\n\\begin{{picture}}({width},{height})\n")));
            for command in commands {
                match command {
                    PictureCommand::Label { x, y, region } => {
                        parts.push(Part::Structure(format!("\\put({x},{y}){{")));
                        parts.push(Part::Region(*region));
                        parts.push(Part::Structure("}\n".into()));
                    }
                    PictureCommand::Line {
                        x,
                        y,
                        dx,
                        dy,
                        length,
                        vector,
                    } => {
                        let command = if *vector { "vector" } else { "line" };
                        parts.push(Part::Structure(format!(
                            "\\put({x},{y}){{\\{command}({dx},{dy}){{{length}}}}}\n"
                        )));
                    }
                }
            }
            parts.push(Part::Structure("\\end{picture}\n\\endgroup\n".into()));
        }
    }
    parts
}
pub(super) fn render(body: &DisplayBody, target: DocumentTarget) -> String {
    let mut output = String::new();
    for part in parts(body, target) {
        match part {
            Part::Structure(value) => output.push_str(&value),
            Part::Region(index) => match target {
                DocumentTarget::Markdown => {
                    super::markdown::inline(&mut output, &body.regions[index].runs);
                }
                DocumentTarget::Latex => {
                    super::latex::inline(&mut output, &body.regions[index].runs);
                }
            },
        }
    }
    output
}
pub(super) fn size(body: &DisplayBody, target: DocumentTarget) -> usize {
    parts(body, target)
        .into_iter()
        .map(|part| match part {
            Part::Structure(value) => value.len(),
            Part::Region(index) => match target {
                DocumentTarget::Markdown => super::markdown::inline_size(&body.regions[index].runs),
                DocumentTarget::Latex => super::latex::inline_size(&body.regions[index].runs),
            },
        })
        .fold(0, usize::saturating_add)
}
