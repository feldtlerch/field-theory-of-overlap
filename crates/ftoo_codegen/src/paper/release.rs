//! M065: one conjunction over the existing paper release conditions.
use super::{PaperFault, Result, bytes, check_reading_acceptance, compare, text};
use serde_json::Value;
use std::path::Path;

pub(super) fn part<T>(part: &'static str, result: Result<T>) -> Result<T> {
    result.map_err(|fault| PaperFault::Release {
        part,
        reason: fault.to_string(),
    })
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
