use super::{
    latex::{Piece, RawTex, append},
    plan::{DocumentPlan, Inline},
};
use ftoo_manifest::AddressBook;
use std::fmt::Write;
const HELPERS: &str = r"\ExplSyntaxOn
\prop_new:N \g_ftdoc_live_prop
\prop_new:N \g_ftdoc_retired_prop
\prop_new:N \g_ftdoc_outside_prop
\prop_new:N \g_ftdoc_sites_prop
\tl_new:N \l_ftdoc_input_tl
\tl_new:N \l_ftdoc_key_tl
\tl_new:N \l_ftdoc_value_tl
\tl_new:N \l_ftdoc_site_tl
\tl_new:N \l_ftdoc_site_key_tl
\tl_new:N \l_ftdoc_unit_tl
\tl_new:N \g_ftdoc_document_tl
\tl_new:N \g_ftdoc_language_tl
\str_new:N \l_ftdoc_hex_str
\str_new:N \l_ftdoc_message_str
\int_new:N \l_ftdoc_chunks_int
\cs_generate_variant:Nn \prop_get:NnNTF { NVNTF }
\cs_generate_variant:Nn \prop_gput:Nnn { Nxx }
\cs_generate_variant:Nn \str_set_convert:Nnnn { Nxnn }
\msg_new:nnn { ftdoc } { tag } { Addressing~runtime~refusal.~Document~\g_ftdoc_document_tl,~unit~\l_ftdoc_unit_tl,~language~\g_ftdoc_language_tl,~site~\l_ftdoc_site_tl. }
\cs_new_protected:Npn \FtdocContext #1#2
 { \tl_gset:Nn \g_ftdoc_document_tl {#1} \tl_gset:Nn \g_ftdoc_language_tl {#2} }
\cs_new_protected:Npn \ftdoc_record:nn #1#2
 {
  \str_set_convert:Nxnn \l_ftdoc_hex_str {#2} {} {utf8/hex}
  \str_set:Nx \l_ftdoc_hex_str { \str_lowercase:f { \l_ftdoc_hex_str } }
  \int_set:Nn \l_ftdoc_chunks_int { \int_max:nn {1} { \int_div_truncate:nn { \str_count:N \l_ftdoc_hex_str + 31 } {32} } }
  \int_step_inline:nn { \l_ftdoc_chunks_int }
   { \iow_term:x { FTDOC-R:#1:\int_eval:n {##1-1}:\int_use:N \l_ftdoc_chunks_int :\str_range:Nnn \l_ftdoc_hex_str {32*(##1-1)+1} {32*##1} } }
 }
\cs_new_protected:Npn \ftdoc_reject:
 {
  \ftdoc_record:nn {i} {\l_ftdoc_key_tl}
  \ftdoc_record:nn {s} {\l_ftdoc_site_tl}
  \ftdoc_record:nn {m} {\l_ftdoc_value_tl}
  \iow_term:n {FTDOC-END}
  \msg_fatal:nn {ftdoc} {tag}
 }
\cs_new_protected:Npn \ftdoc_unknown:
 {
  \str_clear:N \l_ftdoc_message_str
  \str_map_inline:Nn \l_ftdoc_key_tl
   {
    \int_case:nnF {`##1}
     {
      {34}{\str_put_right:Nx \l_ftdoc_message_str {\char_generate:nn {92}{12}\char_generate:nn {34}{12}}}
      {92}{\str_put_right:Nx \l_ftdoc_message_str {\char_generate:nn {92}{12}\char_generate:nn {92}{12}}}
      {10}{\str_put_right:Nx \l_ftdoc_message_str {\char_generate:nn {92}{12}n}}
      {13}{\str_put_right:Nx \l_ftdoc_message_str {\char_generate:nn {92}{12}r}}
      {9}{\str_put_right:Nx \l_ftdoc_message_str {\char_generate:nn {92}{12}t}}
     }
     {
      \bool_lazy_or:nnTF
       {\int_compare_p:n {`##1 < 32}}
       {\bool_lazy_and_p:nn {\int_compare_p:n {`##1 > 126}} {\int_compare_p:n {`##1 < 160}}}
       {\str_put_right:Nx \l_ftdoc_message_str {\char_generate:nn {92}{12}u\char_generate:nn {123}{12}\int_to_hex:n {`##1}\char_generate:nn {125}{12}}}
       {\str_put_right:Nn \l_ftdoc_message_str {##1}}
     }
   }
  \tl_set:Nx \l_ftdoc_value_tl {tag~\char_generate:nn {34}{12}\l_ftdoc_message_str\char_generate:nn {34}{12}:~unknown}
  \ftdoc_reject:
 }
\cs_new_protected:Npn \FtdocTag #1#2
 {
  \tl_set:Nx \l_ftdoc_input_tl {#1}
  \tl_set:Nx \l_ftdoc_key_tl {\tl_to_str:V \l_ftdoc_input_tl}
  \tl_set:Nn \l_ftdoc_site_tl {#2}
  \tl_set:Nx \l_ftdoc_site_key_tl {\tl_to_str:V \g_ftdoc_document_tl/\tl_to_str:V \g_ftdoc_language_tl/\tl_to_str:V \l_ftdoc_site_tl}
  \prop_get:NVNTF \g_ftdoc_sites_prop \l_ftdoc_site_key_tl \l_ftdoc_unit_tl
   {} {\tl_clear:N \l_ftdoc_unit_tl}
  \prop_get:NVNTF \g_ftdoc_live_prop \l_ftdoc_key_tl \l_ftdoc_value_tl
   {\exp_after:wN \use_i:nn \l_ftdoc_value_tl}
   {\prop_get:NVNTF \g_ftdoc_retired_prop \l_ftdoc_key_tl \l_ftdoc_value_tl
     {\ftdoc_reject:}
     {\prop_get:NVNTF \g_ftdoc_outside_prop \l_ftdoc_key_tl \l_ftdoc_value_tl
       {\ftdoc_reject:}{\ftdoc_unknown:}}}
 }
";
fn inert(value: &str) -> String {
    let mut output = String::new();
    for c in value.chars() {
        write!(output, "\\char_generate:nn {{{}}}{{12}}", u32::from(c)).expect("string write");
    }
    output
}
pub(super) fn render(
    book: &AddressBook<'_>,
    plans: &[DocumentPlan],
) -> Result<String, super::DocumentError> {
    let mut output = String::new();
    append(&mut output, Piece::Raw(RawTex(HELPERS)));
    let mut live: Vec<_> = book.live().map(ftoo_manifest::TagId::descriptor).collect();
    live.sort_by_key(|t| t.spelling());
    for tag in live {
        let record = format!(
            "\\prop_gput:Nxx \\g_ftdoc_live_prop {{{}}} {{{{{}}}{{{}}}}}\n",
            inert(tag.spelling()),
            inert(tag.spelling()),
            inert(
                book.resolve(tag.spelling())
                    .expect("live store")
                    .semantic_name()
            )
        );
        push(&mut output, &record, plans)?;
    }
    let mut retired: Vec<_> = book.retired().map(|e| e.tag().descriptor()).collect();
    retired.sort_by_key(|t| t.spelling());
    let mut outside: Vec<_> = book.out_of_slice().map(|e| e.tag().descriptor()).collect();
    outside.sort_by_key(|t| t.spelling());
    for (store, tags) in [("retired", retired), ("outside", outside)] {
        for tag in tags {
            let message = book
                .resolve(tag.spelling())
                .expect_err("unavailable store")
                .to_string();
            let record = format!(
                "\\prop_gput:Nxx \\g_ftdoc_{store}_prop {{{}}} {{{}}}\n",
                inert(tag.spelling()),
                inert(&message)
            );
            push(&mut output, &record, plans)?;
        }
    }
    let mut sites = Vec::new();
    for plan in plans {
        for block in &plan.blocks {
            for run in block.target_runs(ftoo_manifest::DocumentTarget::Latex) {
                if let Inline::Tag { token, .. } = run {
                    sites.push((
                        plan.document.as_str(),
                        plan.language.as_str(),
                        block.unit.as_str(),
                        token.as_str(),
                    ));
                }
            }
        }
    }
    sites.sort_unstable();
    for (document, language, unit, token) in sites {
        let record = format!(
            "\\prop_gput:Nxx \\g_ftdoc_sites_prop {{{}}} {{{}}}\n",
            inert(&format!("{document}/{language}/{token}")),
            inert(unit)
        );
        push(&mut output, &record, plans)?;
    }
    push(&mut output, "\\ExplSyntaxOff\n", plans)?;
    check(book, plans, &output)?;
    Ok(output)
}
fn record(store: &str, key: &str, value: String) -> super::diagnostics::EmissionRecord {
    let mut site = super::input::site("generated/addressing.tex");
    site.path = vec![
        ftoo_manifest::PathPart::Key(store.into()),
        ftoo_manifest::PathPart::Key(key.into()),
    ];
    super::diagnostics::EmissionRecord {
        kind: super::diagnostics::EmissionKind::Tag,
        unit: key.into(),
        language: store.into(),
        target: ftoo_manifest::DocumentTarget::Latex,
        occurrence: None,
        value,
        site,
    }
}
fn group(input: &mut &str) -> Option<String> {
    *input = input.strip_prefix('{')?;
    let mut output = String::new();
    while !input.starts_with('}') {
        *input = input.strip_prefix("\\char_generate:nn {")?;
        let end = input.find('}')?;
        let scalar = input[..end].parse::<u32>().ok()?;
        if scalar.to_string() != input[..end] {
            return None;
        }
        output.push(char::from_u32(scalar)?);
        *input = input[end..].strip_prefix("}{12}")?;
    }
    *input = &input[1..];
    Some(output)
}
fn decode_record(line: &str) -> Option<super::diagnostics::EmissionRecord> {
    let tail = line.strip_prefix("\\prop_gput:Nxx \\g_ftdoc_")?;
    let (store, mut tail) = tail.split_once("_prop ")?;
    if !["live", "retired", "outside", "sites"].contains(&store) {
        return None;
    }
    let key = group(&mut tail)?;
    tail = tail.strip_prefix(' ')?;
    let value = if store == "live" {
        tail = tail.strip_prefix('{')?;
        let handle = group(&mut tail)?;
        let label = group(&mut tail)?;
        tail = tail.strip_prefix('}')?;
        format!("{handle}\n{label}")
    } else {
        group(&mut tail)?
    };
    tail.is_empty().then(|| record(store, &key, value))
}
pub(super) fn check(
    book: &AddressBook<'_>,
    plans: &[DocumentPlan],
    source: &str,
) -> Result<(), super::DocumentError> {
    let mut expected = Vec::new();
    for tag in book.live() {
        let handle = tag.descriptor().spelling();
        let target = book.resolve(handle).expect("checked live target");
        expected.push(record(
            "live",
            handle,
            format!("{handle}\n{}", target.semantic_name()),
        ));
    }
    for (store, tags) in [
        (
            "retired",
            book.retired()
                .map(ftoo_manifest::RetiredEntry::tag)
                .collect::<Vec<_>>(),
        ),
        (
            "outside",
            book.out_of_slice()
                .map(|entry| entry.tag())
                .collect::<Vec<_>>(),
        ),
    ] {
        for tag in tags {
            let handle = tag.descriptor().spelling();
            expected.push(record(
                store,
                handle,
                book.resolve(handle)
                    .expect_err("checked unavailable target")
                    .to_string(),
            ));
        }
    }
    expected.sort_by(|a, b| {
        let rank = |store: &str| match store {
            "live" => 0,
            "retired" => 1,
            _ => 2,
        };
        (rank(&a.language), &a.unit).cmp(&(rank(&b.language), &b.unit))
    });
    let mut sites = Vec::new();
    for plan in plans {
        for block in &plan.blocks {
            for run in block.target_runs(ftoo_manifest::DocumentTarget::Latex) {
                if let Inline::Tag { token, .. } = run {
                    sites.push((&plan.document, &plan.language, &block.unit, token));
                }
            }
        }
    }
    sites.sort_unstable();
    for (document, language, unit, token) in sites {
        expected.push(record(
            "sites",
            &format!("{document}/{language}/{token}"),
            unit.clone(),
        ));
    }
    compare(&expected, plans, source)
}
pub(super) fn compare(
    expected: &[super::diagnostics::EmissionRecord],
    plans: &[DocumentPlan],
    source: &str,
) -> Result<(), super::DocumentError> {
    let body = source
        .strip_prefix(HELPERS)
        .and_then(|s| s.strip_suffix("\\ExplSyntaxOff\n"));
    let actual = body.map_or_else(
        || vec![record("runtime", "framing", source.into())],
        |body| {
            body.lines()
                .map(|line| {
                    decode_record(line).unwrap_or_else(|| record("runtime", "record", line.into()))
                })
                .collect()
        },
    );
    let index = expected
        .iter()
        .zip(&actual)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.len().min(actual.len()));
    let bytes = body.map_or(0..source.len(), |body| {
        let mut start = HELPERS.len();
        for (ordinal, line) in body.split_inclusive('\n').enumerate() {
            if ordinal == index {
                return start..start + line.len();
            }
            start += line.len();
        }
        start..start
    });
    let (mut missing, mut unexpected, mut changed) = (Vec::new(), Vec::new(), Vec::new());
    super::plan::difference(
        expected,
        &actual,
        &mut missing,
        &mut unexpected,
        &mut changed,
    );
    if missing.is_empty() && unexpected.is_empty() && changed.is_empty() {
        return Ok(());
    }
    Err(super::failure(
        ftoo_manifest::RuleId::M055,
        ftoo_manifest::AssemblyStage::Emission,
        ftoo_manifest::AssemblySite::Generated {
            path: "generated/addressing.tex".into(),
            bytes,
            origins: plans
                .iter()
                .flat_map(|plan| plan.origins(ftoo_manifest::DocumentTarget::Latex))
                .collect(),
        },
        super::AssemblyFault::EmissionMismatch {
            missing,
            unexpected,
            changed,
        },
    ))
}
fn push(
    output: &mut String,
    line: &str,
    plans: &[DocumentPlan],
) -> Result<(), super::DocumentError> {
    if output.len().saturating_add(line.len()) > 16_777_216 {
        return Err(super::failure(
            ftoo_manifest::RuleId::M055,
            ftoo_manifest::AssemblyStage::Emission,
            ftoo_manifest::AssemblySite::Generated {
                path: "generated/addressing.tex".into(),
                bytes: 16_777_216..16_777_217,
                origins: plans
                    .iter()
                    .flat_map(|p| p.origins(ftoo_manifest::DocumentTarget::Latex))
                    .collect(),
            },
            super::AssemblyFault::Limit {
                resource: ftoo_manifest::AssemblyResource::DocumentBytes,
                limit: 16_777_216,
                actual: 16_777_217,
            },
        ));
    }
    output.push_str(line);
    Ok(())
}
