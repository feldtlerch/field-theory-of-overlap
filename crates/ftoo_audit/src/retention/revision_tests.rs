use super::{revision, *};

#[test]
fn engine_source_refusals_preserve_complete_resolved_identity() {
    let pin = ftoo_manifest::ENGINE_PIN.0;
    let entry = format!(
        "[[package]]\nname=\"nrz-cas\"\nversion=\"0.1.0\"\nsource=\"git+https://example.invalid/engine?rev=other#{pin}\"\n"
    );
    for value in [
        "registry+https://example.invalid",
        "git+https://example.invalid#short",
        "path+local",
        "git+https://example.invalid",
    ] {
        let changed =
            format!("[[package]]\nname=\"nrz-cas\"\nversion=\"0.1.0\"\nsource=\"{value}\"\n");
        let actual = revision::resolved(&changed, EngineRevisionSource::CurrentLock).unwrap_err();
        assert_eq!(
            actual,
            EngineRevisionUnavailable {
                source: EngineRevisionSource::CurrentLock,
                path: "Cargo.lock",
                package: "nrz-cas",
                reason: EngineRevisionReason::Unsupported {
                    value: value.into()
                }
            }
        );
        assert_eq!(
            actual.to_string(),
            format!(
                "retention provenance M061 at file \"Cargo.lock\": engine-revision-unavailable source=current-lock package=\"nrz-cas\" reason=unsupported value=\"{value}\""
            )
        );
        assert_eq!(
            revision::resolved(&entry, EngineRevisionSource::CurrentLock),
            Ok(pin.into())
        );
    }
    let source = format!("git+https://example.invalid/engine?rev=other#{pin}");
    let actual = revision::resolved(
        &format!("{entry}{entry}"),
        EngineRevisionSource::CurrentLock,
    )
    .unwrap_err();
    assert_eq!(
        actual,
        EngineRevisionUnavailable {
            source: EngineRevisionSource::CurrentLock,
            path: "Cargo.lock",
            package: "nrz-cas",
            reason: EngineRevisionReason::Ambiguous {
                entries: vec![
                    ("0.1.0".into(), source.clone()),
                    ("0.1.0".into(), source.clone())
                ]
            }
        }
    );
    assert_eq!(
        actual.to_string(),
        format!(
            "retention provenance M061 at file \"Cargo.lock\": engine-revision-unavailable source=current-lock package=\"nrz-cas\" reason=ambiguous entries=[{{\"version\":\"0.1.0\",\"source\":\"{source}\"}},{{\"version\":\"0.1.0\",\"source\":\"{source}\"}}]"
        )
    );
    assert_eq!(
        revision::resolved(&entry, EngineRevisionSource::CurrentLock),
        Ok(pin.into())
    );
}

#[test]
fn current_lock_and_requested_revision_are_independent_cross_checks() {
    let lock = include_str!("../../../../Cargo.lock");
    let manifest = include_str!("../../Cargo.toml");
    let pin = ftoo_manifest::ENGINE_PIN.0;
    assert_eq!(check_current_engine(lock, manifest), Ok(()));
    for (path, changed_lock, changed_manifest) in [
        (
            "Cargo.lock",
            lock.replace(&format!("#{pin}"), &format!("#{}", "0".repeat(40))),
            manifest.to_string(),
        ),
        (
            "crates/ftoo_audit/Cargo.toml",
            lock.to_string(),
            manifest.replace(pin, &"0".repeat(40)),
        ),
    ] {
        let actual = check_current_engine(&changed_lock, &changed_manifest).unwrap_err();
        assert_eq!(
            actual,
            RetentionError::ProvenanceMismatch {
                component: Component::EngineRevision,
                path: path.into(),
                difference: ProvenanceDifference::Values {
                    expected: ProvenanceValue::Text(pin.into()),
                    actual: ProvenanceValue::Text("0".repeat(40))
                }
            }
        );
        assert_eq!(
            actual.to_string(),
            format!(
                "retention provenance M061 at file \"{path}\": provenance-mismatch component=engine-revision expected=\"{pin}\" actual=\"0000000000000000000000000000000000000000\""
            )
        );
        assert_eq!(check_current_engine(lock, manifest), Ok(()));
    }
}

#[test]
fn malformed_lock_keeps_the_complete_parser_message_and_span() {
    let lock = include_str!("../../../../Cargo.lock");
    assert_eq!(
        revision::resolved(lock, EngineRevisionSource::CurrentLock),
        Ok(ftoo_manifest::ENGINE_PIN.0.into())
    );
    let message = "TOML parse error at line 1, column 1\n  |\n1 | [\n  | ^\ninvalid table header\n";
    let actual = revision::resolved("[", EngineRevisionSource::CurrentLock).unwrap_err();
    assert_eq!(
        actual,
        EngineRevisionUnavailable {
            source: EngineRevisionSource::CurrentLock,
            path: "Cargo.lock",
            package: "nrz-cas",
            reason: EngineRevisionReason::Malformed {
                message: message.into(),
                span: Some(0..1)
            }
        }
    );
    assert_eq!(
        actual.to_string(),
        format!(
            "retention provenance M061 at file \"Cargo.lock\": engine-revision-unavailable source=current-lock package=\"nrz-cas\" reason=malformed message={} span=0..1",
            serde_json::to_string(message).unwrap()
        )
    );
    assert_eq!(
        revision::resolved(lock, EngineRevisionSource::CurrentLock),
        Ok(ftoo_manifest::ENGINE_PIN.0.into())
    );
}
