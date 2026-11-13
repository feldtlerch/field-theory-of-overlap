// Complete native evidence at the locked engine, bundle and fixed preamble.
pub(super) fn seeded_evidence(name: &str) -> (&'static str, &'static str, &'static str) {
    let key = match name {
        "stored" => "negative/stored/refusal",
        "collision" => "negative/collision/refusal",
        "include" => "negative/include/refusal",
        "closure" => "negative/closure/refusal",
        _ => unreachable!("fixed negative case inventory"),
    };
    let (_, stdout, stderr, log) = raw_evidence(key);
    (stdout, stderr, log)
}
pub(super) fn evidence(name: &str) -> (&'static str, &'static str, &'static str) {
    let key = match name {
        "stored" => "cold/negative/stored/refusal",
        "collision" => "cold/negative/collision/refusal",
        "include" => "cold/negative/include/refusal",
        "closure" => "cold/negative/closure/refusal",
        _ => unreachable!("fixed negative case inventory"),
    };
    let (_, stdout, stderr, log) = raw_evidence(key);
    (stdout, stderr, log)
}
// Independent complete format-mode evidence.
pub(in crate::documents) fn raw_evidence(
    name: &str,
) -> (i32, &'static str, &'static str, &'static str) {
    match name {
        "bootstrap" => (1, FORMAT_CHANNEL_0, FORMAT_CHANNEL_1, FORMAT_CHANNEL_2),
        "cold/display/0" => (0, FORMAT_CHANNEL_3, FORMAT_CHANNEL_4, FORMAT_CHANNEL_5),
        "cold/display/1" => (0, FORMAT_CHANNEL_6, FORMAT_CHANNEL_4, FORMAT_CHANNEL_7),
        "cold/display/2" => (0, FORMAT_CHANNEL_3, FORMAT_CHANNEL_4, FORMAT_CHANNEL_8),
        "cold/faces/accepted" => (0, FORMAT_CHANNEL_9, FORMAT_CHANNEL_4, FORMAT_CHANNEL_10),
        "cold/lost-character/refusal" => {
            (1, FORMAT_CHANNEL_11, FORMAT_CHANNEL_12, FORMAT_CHANNEL_13)
        }
        "cold/negative/closure/refusal" => {
            (1, FORMAT_CHANNEL_14, FORMAT_CHANNEL_15, FORMAT_CHANNEL_16)
        }
        "cold/negative/collision/refusal" => {
            (1, FORMAT_CHANNEL_17, FORMAT_CHANNEL_18, FORMAT_CHANNEL_19)
        }
        "cold/negative/include/refusal" => {
            (1, FORMAT_CHANNEL_20, FORMAT_CHANNEL_21, FORMAT_CHANNEL_22)
        }
        "cold/negative/stored/refusal" => {
            (1, FORMAT_CHANNEL_11, FORMAT_CHANNEL_23, FORMAT_CHANNEL_24)
        }
        "cold/unknown/refusal" => (1, FORMAT_CHANNEL_25, FORMAT_CHANNEL_26, FORMAT_CHANNEL_27),
        "display/0" => (0, FORMAT_CHANNEL_28, FORMAT_CHANNEL_29, FORMAT_CHANNEL_5),
        "display/1" => (0, FORMAT_CHANNEL_28, FORMAT_CHANNEL_29, FORMAT_CHANNEL_7),
        "display/2" => (0, FORMAT_CHANNEL_28, FORMAT_CHANNEL_29, FORMAT_CHANNEL_8),
        "faces/accepted" => (0, FORMAT_CHANNEL_30, FORMAT_CHANNEL_29, FORMAT_CHANNEL_10),
        "lost-character/refusal" => (1, FORMAT_CHANNEL_31, FORMAT_CHANNEL_32, FORMAT_CHANNEL_13),
        "negative/closure/refusal" => (1, FORMAT_CHANNEL_33, FORMAT_CHANNEL_34, FORMAT_CHANNEL_16),
        "negative/collision/refusal" => {
            (1, FORMAT_CHANNEL_35, FORMAT_CHANNEL_36, FORMAT_CHANNEL_19)
        }
        "negative/include/refusal" => (1, FORMAT_CHANNEL_37, FORMAT_CHANNEL_38, FORMAT_CHANNEL_22),
        "negative/stored/refusal" => (1, FORMAT_CHANNEL_31, FORMAT_CHANNEL_39, FORMAT_CHANNEL_24),
        "unknown/refusal" => (1, FORMAT_CHANNEL_40, FORMAT_CHANNEL_41, FORMAT_CHANNEL_27),
        "citation/literal" => (
            0,
            include_str!("../../../tests/fixtures/native/citation-literal.stdout"),
            include_str!("../../../tests/fixtures/native/citation-literal.stderr"),
            include_str!("../../../tests/fixtures/native/citation-literal.log"),
        ),
        "citation/composed" => (
            0,
            include_str!("../../../tests/fixtures/native/citation-composed.stdout"),
            include_str!("../../../tests/fixtures/native/citation-composed.stderr"),
            include_str!("../../../tests/fixtures/native/citation-composed.log"),
        ),
        "citation/retired" => (
            1,
            include_str!("../../../tests/fixtures/native/citation-retired.stdout"),
            include_str!("../../../tests/fixtures/native/citation-retired.stderr"),
            include_str!("../../../tests/fixtures/native/citation-retired.log"),
        ),
        "citation/restored" => (
            0,
            include_str!("../../../tests/fixtures/native/citation-restored.stdout"),
            include_str!("../../../tests/fixtures/native/citation-restored.stderr"),
            include_str!("../../../tests/fixtures/native/citation-restored.log"),
        ),
        "citation/encoded" => (
            0,
            include_str!("../../../tests/fixtures/native/citation-encoded.stdout"),
            include_str!("../../../tests/fixtures/native/citation-encoded.stderr"),
            include_str!("../../../tests/fixtures/native/citation-encoded.log"),
        ),
        _ => unreachable!("fixed format-mode inventory"),
    }
}
const FORMAT_CHANNEL_0: &str = include_str!("../../../tests/fixtures/native/bootstrap.stdout");
const FORMAT_CHANNEL_1: &str = include_str!("../../../tests/fixtures/native/bootstrap.stderr");
const FORMAT_CHANNEL_2: &str = include_str!("../../../tests/fixtures/native/bootstrap.log");
const FORMAT_CHANNEL_3: &str = include_str!("../../../tests/fixtures/native/cold-display-0.stdout");
const FORMAT_CHANNEL_4: &str = include_str!("../../../tests/fixtures/native/cold-display-0.stderr");
const FORMAT_CHANNEL_5: &str = include_str!("../../../tests/fixtures/native/cold-display-0.log");
const FORMAT_CHANNEL_6: &str = include_str!("../../../tests/fixtures/native/cold-display-1.stdout");
const FORMAT_CHANNEL_7: &str = include_str!("../../../tests/fixtures/native/cold-display-1.log");
const FORMAT_CHANNEL_8: &str = include_str!("../../../tests/fixtures/native/cold-display-2.log");
const FORMAT_CHANNEL_9: &str =
    include_str!("../../../tests/fixtures/native/cold-faces-accepted.stdout");
const FORMAT_CHANNEL_10: &str =
    include_str!("../../../tests/fixtures/native/cold-faces-accepted.log");
const FORMAT_CHANNEL_11: &str =
    include_str!("../../../tests/fixtures/native/cold-lost-character-refusal.stdout");
const FORMAT_CHANNEL_12: &str =
    include_str!("../../../tests/fixtures/native/cold-lost-character-refusal.stderr");
const FORMAT_CHANNEL_13: &str =
    include_str!("../../../tests/fixtures/native/cold-lost-character-refusal.log");
const FORMAT_CHANNEL_14: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-closure-refusal.stdout");
const FORMAT_CHANNEL_15: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-closure-refusal.stderr");
const FORMAT_CHANNEL_16: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-closure-refusal.log");
const FORMAT_CHANNEL_17: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-collision-refusal.stdout");
const FORMAT_CHANNEL_18: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-collision-refusal.stderr");
const FORMAT_CHANNEL_19: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-collision-refusal.log");
const FORMAT_CHANNEL_20: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-include-refusal.stdout");
const FORMAT_CHANNEL_21: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-include-refusal.stderr");
const FORMAT_CHANNEL_22: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-include-refusal.log");
const FORMAT_CHANNEL_23: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-stored-refusal.stderr");
const FORMAT_CHANNEL_24: &str =
    include_str!("../../../tests/fixtures/native/cold-negative-stored-refusal.log");
const FORMAT_CHANNEL_25: &str =
    include_str!("../../../tests/fixtures/native/cold-unknown-refusal.stdout");
const FORMAT_CHANNEL_26: &str =
    include_str!("../../../tests/fixtures/native/cold-unknown-refusal.stderr");
const FORMAT_CHANNEL_27: &str =
    include_str!("../../../tests/fixtures/native/cold-unknown-refusal.log");
const FORMAT_CHANNEL_28: &str = include_str!("../../../tests/fixtures/native/display-0.stdout");
const FORMAT_CHANNEL_29: &str = include_str!("../../../tests/fixtures/native/display-0.stderr");
const FORMAT_CHANNEL_30: &str =
    include_str!("../../../tests/fixtures/native/faces-accepted.stdout");
const FORMAT_CHANNEL_31: &str =
    include_str!("../../../tests/fixtures/native/lost-character-refusal.stdout");
const FORMAT_CHANNEL_32: &str =
    include_str!("../../../tests/fixtures/native/lost-character-refusal.stderr");
const FORMAT_CHANNEL_33: &str =
    include_str!("../../../tests/fixtures/native/negative-closure-refusal.stdout");
const FORMAT_CHANNEL_34: &str =
    include_str!("../../../tests/fixtures/native/negative-closure-refusal.stderr");
const FORMAT_CHANNEL_35: &str =
    include_str!("../../../tests/fixtures/native/negative-collision-refusal.stdout");
const FORMAT_CHANNEL_36: &str =
    include_str!("../../../tests/fixtures/native/negative-collision-refusal.stderr");
const FORMAT_CHANNEL_37: &str =
    include_str!("../../../tests/fixtures/native/negative-include-refusal.stdout");
const FORMAT_CHANNEL_38: &str =
    include_str!("../../../tests/fixtures/native/negative-include-refusal.stderr");
const FORMAT_CHANNEL_39: &str =
    include_str!("../../../tests/fixtures/native/negative-stored-refusal.stderr");
const FORMAT_CHANNEL_40: &str =
    include_str!("../../../tests/fixtures/native/unknown-refusal.stdout");
const FORMAT_CHANNEL_41: &str =
    include_str!("../../../tests/fixtures/native/unknown-refusal.stderr");
