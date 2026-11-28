//! M065: one conjunction over the existing paper release conditions.
use super::{PaperFault, Result, bytes, check_reading_acceptance, compare, text};
use serde_json::Value;
use std::path::Path;

pub(super) fn write_registers(root: &Path, prepared: &crate::PreparedDocuments) -> Result<()> {
    for (name, expected) in [
        (
            "registry/screen-destination-gaps.json",
            canonical_destinations(&prepared.screen_gap_inventory())?,
        ),
        (
            "registry/html-block-gaps.json",
            super::html_gaps::canonical(prepared.html_gap_rows())?,
        ),
    ] {
        let path = root.join(name);
        if path.is_symlink() {
            return Err(PaperFault::Input(format!(
                "{name}: derived register is a symlink"
            )));
        }
        if std::fs::read(&path).ok().as_deref() != Some(expected.as_slice()) {
            std::fs::write(&path, expected)
                .map_err(|error| PaperFault::Input(format!("{name}: {error}")))?;
        }
    }
    Ok(())
}

pub(super) fn part<T>(part: &'static str, result: Result<T>) -> Result<T> {
    result.map_err(|fault| PaperFault::Release {
        part,
        reason: fault.to_string(),
    })
}

pub(super) fn destinations(prepared: &crate::PreparedDocuments) -> Result<()> {
    part(
        "screen destinations",
        prepared
            .check_release_destinations()
            .map_err(PaperFault::Input),
    )
}

pub(super) fn registered_destinations(
    root: &Path,
    prepared: &crate::PreparedDocuments,
) -> Result<()> {
    let path = "registry/screen-destination-gaps.json";
    part(
        "screen destinations",
        bytes(&root.join(path))
            .and_then(|stored| compare_destinations(&stored, &prepared.screen_gap_inventory())),
    )
}

pub(super) fn canonical_destinations(inventory: &Value) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec_pretty(inventory).map_err(|error| {
        PaperFault::Input(format!("registry/screen-destination-gaps.json: {error}"))
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn compare_destinations(stored: &[u8], inventory: &Value) -> Result<()> {
    if stored == canonical_destinations(inventory)? {
        Ok(())
    } else {
        Err(PaperFault::Input("registry/screen-destination-gaps.json differs from the canonical checked screen gap inventory".into()))
    }
}

pub(super) fn screen_parts(
    root: &Path,
    prepared: &crate::PreparedDocuments,
    registered: bool,
) -> Result<()> {
    registered_destinations(root, prepared)?;
    if !registered {
        destinations(prepared)?;
    }
    part(
        "HTML block gaps",
        super::html_gaps::check(root, prepared.html_gap_rows()),
    )?;
    if !registered {
        super::html_gaps::refusal(prepared.html_gap_rows())?;
    }
    Ok(())
}

pub(super) fn inputs(root: &Path) -> Result<()> {
    part(
        "self-description acceptance",
        check_reading_acceptance(root),
    )?;
    Ok(())
}

pub(super) fn check(root: &Path, work: &Path, receipt: &Value) -> Result<()> {
    inputs(root)?;
    check_current(root, work, receipt)
}

pub(super) fn check_current(root: &Path, work: &Path, receipt: &Value) -> Result<()> {
    part(
        "printed identities",
        (|| {
            let rows: Value = serde_json::from_slice(&bytes(&work.join("identity-ledger.json"))?)
                .map_err(|error| error.to_string())?;
            for row in rows.as_array().ok_or("Missing shipped identity ledger")? {
                if !text(row, "spelling")?.is_empty()
                    && !matches!(text(row, "status")?, "admitted" | "paper")
                {
                    return Err(
                        format!("Provisional printed identity: {}", text(row, "key")?).into(),
                    );
                }
            }
            Ok(())
        })(),
    )?;
    part("paper byte gate", compare(root, work, receipt))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn screen_register_refuses_missing_extra_changed_and_duplicate_rows() {
        let inventory = json!({"schema": 1, "entries": [{"document": "paper-pilot", "language": "en", "fragment": "target"}], "content-gaps": [{"document": "paper-pilot", "language": "en", "position": 1, "before-unit": "first", "after-unit": "second", "message": "omitted"}]});
        let expected = canonical_destinations(&inventory).unwrap();
        compare_destinations(&expected, &inventory).unwrap();
        for changed in [
            json!({"schema": 1, "entries": [], "content-gaps": inventory["content-gaps"]}),
            json!({"schema": 1, "entries": [inventory["entries"][0], inventory["entries"][0]], "content-gaps": inventory["content-gaps"]}),
            json!({"schema": 1, "entries": [{"document": "paper-pilot", "language": "de", "fragment": "target"}], "content-gaps": inventory["content-gaps"]}),
            json!({"schema": 1, "entries": inventory["entries"], "content-gaps": [{"document": "paper-pilot", "language": "en", "position": 1, "before-unit": "other", "after-unit": "second", "message": "omitted"}]}),
        ] {
            assert!(
                compare_destinations(&canonical_destinations(&changed).unwrap(), &inventory)
                    .is_err()
            );
        }
        let duplicate = String::from_utf8(expected).unwrap().replacen(
            "\"entries\":",
            "\"entries\": [], \"entries\":",
            1,
        );
        assert!(compare_destinations(duplicate.as_bytes(), &inventory).is_err());
        let empty = json!({"schema": 1, "entries": [], "content-gaps": []});
        assert!(
            compare_destinations(&canonical_destinations(&inventory).unwrap(), &empty).is_err()
        );
    }
}
