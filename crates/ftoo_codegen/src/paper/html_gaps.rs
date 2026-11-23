//! The development inventory of HTML passages withheld by the checked document plan.
use super::{PaperFault, Result, bytes};
use crate::documents::HtmlGapRow;
use serde_json::json;
use std::path::Path;

const REGISTER: &str = "registry/html-block-gaps.json";

fn rows(gaps: &[HtmlGapRow]) -> Vec<serde_json::Value> {
    gaps.iter()
        .map(|row| {
            json!({
                "document": row.document,
                "unit": row.unit,
                "block": row.block,
                "inline-kind": row.inline_kind,
            })
        })
        .collect()
}

fn describe(row: &HtmlGapRow) -> String {
    format!(
        "{} / {} block {} {} at {}",
        row.document, row.unit, row.block, row.inline_kind, row.site
    )
}

pub(crate) fn refusal(gaps: &[HtmlGapRow]) -> Result<()> {
    if gaps.is_empty() {
        return Ok(());
    }
    Err(PaperFault::Release {
        part: "HTML block gaps",
        reason: gaps.iter().map(describe).collect::<Vec<_>>().join("; "),
    })
}

fn canonical(gaps: &[HtmlGapRow]) -> Result<Vec<u8>> {
    let expected = json!({"schema": 1, "gaps": rows(gaps)});
    let mut bytes =
        serde_json::to_vec_pretty(&expected).map_err(|error| format!("{REGISTER}: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn compare(stored: &[u8], gaps: &[HtmlGapRow]) -> Result<()> {
    let expected = canonical(gaps)?;
    if stored != expected {
        return Err(
            format!("{REGISTER} differs from the canonical checked HTML block gaps").into(),
        );
    }
    Ok(())
}

pub(crate) fn check(root: &Path, gaps: &[HtmlGapRow]) -> Result<()> {
    compare(&bytes(&root.join(REGISTER))?, gaps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ftoo_manifest::{SourceAnchor, UnitSite};

    #[test]
    fn registered_gaps_refuse_missing_extra_and_changed_rows() {
        let row = HtmlGapRow {
            document: "example".into(),
            unit: "example-unit".into(),
            block: 7,
            inline_kind: "mathematics".into(),
            site: UnitSite {
                file: "fixture.toml".into(),
                path: vec![],
                anchor: SourceAnchor::WholeFile,
                decoded: None,
                occurrence: None,
            },
        };
        let row_value = rows(std::slice::from_ref(&row))[0].clone();
        let expected = canonical(std::slice::from_ref(&row)).unwrap();
        compare(&expected, std::slice::from_ref(&row)).unwrap();
        for changed in [
            json!({"schema": 1, "gaps": []}),
            json!({"schema": 1, "gaps": [row_value.clone(), row_value.clone()]}),
            json!({"schema": 1, "gaps": [{"document": "example", "unit": "example-unit", "block": 8, "inline-kind": "mathematics"}]}),
            json!({"schema": 1, "gaps": [{"document": "example", "unit": "example-unit", "block": 7, "inline-kind": "unsupported-style"}]}),
        ] {
            let mut bytes = serde_json::to_vec_pretty(&changed).unwrap();
            bytes.push(b'\n');
            assert!(compare(&bytes, std::slice::from_ref(&row)).is_err());
        }
        assert!(compare(&expected, &[]).is_err());
        compare(&canonical(&[]).unwrap(), &[]).unwrap();
        let duplicate = String::from_utf8(expected.clone()).unwrap().replacen(
            "\"gaps\":",
            "\"gaps\": [], \"gaps\":",
            1,
        );
        assert!(compare(duplicate.as_bytes(), std::slice::from_ref(&row)).is_err());
        assert!(
            compare(
                &serde_json::to_vec(&json!({"schema": 1, "gaps": [row_value]})).unwrap(),
                std::slice::from_ref(&row)
            )
            .is_err()
        );
        assert!(refusal(&[row]).unwrap_err().to_string().contains("Paper release M065 refused at HTML block gaps: example / example-unit block 7 mathematics"));
    }
}
