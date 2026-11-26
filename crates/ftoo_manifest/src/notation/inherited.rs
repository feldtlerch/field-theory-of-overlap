//! Decoded single-symbol inherited declarations.
use super::{DefinitionId, DefinitionKind, DefinitionSpec, ExprDraft, PrimitiveSpec, Signature};
use crate::assembly::parse_catalogue_body;

#[derive(Clone, Copy)]
pub(super) enum SourceFile {
    Symbols,
    Historical,
}
pub(super) const SYMBOLS: [(DefinitionId, &str, SourceFile); 33] = [
    (DefinitionId::Decayrate, "Decayrate", SourceFile::Symbols),
    (DefinitionId::Moralvel, "Moralvel", SourceFile::Symbols),
    (DefinitionId::Fielddrift, "Fielddrift", SourceFile::Symbols),
    (DefinitionId::Leanbare, "Leanbare", SourceFile::Historical),
    (DefinitionId::Will, "Will", SourceFile::Historical),
    (DefinitionId::Bendfn, "Bendfn", SourceFile::Historical),
    (
        DefinitionId::Frameloading,
        "Frameloading",
        SourceFile::Historical,
    ),
    (
        DefinitionId::HingeAlpha,
        "HingeAlpha",
        SourceFile::Historical,
    ),
    (
        DefinitionId::Accountability,
        "Accountability",
        SourceFile::Historical,
    ),
    (DefinitionId::Idcov, "Idcov", SourceFile::Historical),
    (
        DefinitionId::Rewardfield,
        "Rewardfield",
        SourceFile::Historical,
    ),
    (
        DefinitionId::Heldweight,
        "Heldweight",
        SourceFile::Historical,
    ),
    (
        DefinitionId::Rewardpotential,
        "Rewardpotential",
        SourceFile::Historical,
    ),
    (
        DefinitionId::Attenuation,
        "Attenuation",
        SourceFile::Historical,
    ),
    (DefinitionId::Biforce, "Biforce", SourceFile::Historical),
    (DefinitionId::HingeBeta, "HingeBeta", SourceFile::Historical),
    (DefinitionId::Horizon, "Horizon", SourceFile::Historical),
    (
        DefinitionId::Nonreprfrac,
        "Nonreprfrac",
        SourceFile::Historical,
    ),
    (DefinitionId::Edgeload, "Edgeload", SourceFile::Historical),
    (
        DefinitionId::Cyclicshift,
        "Cyclicshift",
        SourceFile::Historical,
    ),
    (
        DefinitionId::Rootofunity,
        "Rootofunity",
        SourceFile::Historical,
    ),
    (DefinitionId::Jacgrowth, "Jacgrowth", SourceFile::Historical),
    (DefinitionId::Jaccouple, "Jaccouple", SourceFile::Historical),
    (
        DefinitionId::Recoveryfact,
        "Recoveryfact",
        SourceFile::Historical,
    ),
    (DefinitionId::Reports, "Reports", SourceFile::Historical),
    (
        DefinitionId::Frameoffsets,
        "Frameoffsets",
        SourceFile::Historical,
    ),
    (DefinitionId::Residuals, "Residuals", SourceFile::Historical),
    (
        DefinitionId::Anchorprior,
        "Anchorprior",
        SourceFile::Historical,
    ),
    (
        DefinitionId::WoodburyJay,
        "WoodburyJay",
        SourceFile::Historical,
    ),
    (DefinitionId::Woodburyd, "Woodburyd", SourceFile::Historical),
    (
        DefinitionId::Colourgainfamily,
        "Colourgainfamily",
        SourceFile::Historical,
    ),
    (
        DefinitionId::Colourspan,
        "Colourspan",
        SourceFile::Historical,
    ),
    (
        DefinitionId::Colourcoordinate,
        "Colourcoordinate",
        SourceFile::Historical,
    ),
];

#[derive(Debug, Eq, PartialEq)]
pub(super) enum DecodeFault {
    Missing,
    Duplicate,
    Wrapper,
    Signature,
    Body,
}

fn mask_comments(source: &str) -> String {
    let mut bytes = source.as_bytes().to_vec();
    let mut comment = false;
    let mut preceding_slashes = 0;
    for byte in &mut bytes {
        if comment {
            if *byte == b'\n' {
                comment = false;
            } else {
                *byte = b' ';
            }
        } else if *byte == b'%' && preceding_slashes % 2 == 0 {
            *byte = b' ';
            comment = true;
        }
        if *byte == b'\\' {
            preceding_slashes += 1;
        } else {
            preceding_slashes = 0;
        }
    }
    String::from_utf8(bytes).expect("comment masking preserves UTF-8")
}

pub(super) fn group(source: &str, at: usize) -> Option<(std::ops::Range<usize>, usize)> {
    let bytes = source.as_bytes();
    let mut start = at;
    while bytes.get(start).is_some_and(u8::is_ascii_whitespace) {
        start += 1;
    }
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut depth = 1usize;
    let mut slash = false;
    for (index, byte) in bytes.iter().enumerate().skip(start + 1) {
        if slash {
            slash = false;
            continue;
        }
        match byte {
            b'\\' => slash = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start + 1..index, index + 1));
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn declaration_range(source: &str, name: &str) -> Option<std::ops::Range<usize>> {
    let masked = mask_comments(source);
    for (at, _) in masked.match_indices("\\NewDocumentCommand") {
        let after_command = at + "\\NewDocumentCommand".len();
        let Some((target, after_target)) = group(&masked, after_command) else {
            continue;
        };
        if masked[target].trim() != format!("\\{name}") {
            continue;
        }
        let end = group(&masked, after_target)
            .and_then(|(_, after_signature)| group(&masked, after_signature))
            .map_or(after_command, |(_, after_body)| after_body);
        return Some(at..end);
    }
    None
}

/// Return the retained declaration and a body parsed by the document grammar.
pub(super) fn decode(
    source: &str,
    name: &str,
    primitives: &[PrimitiveSpec],
    definitions: &[DefinitionSpec],
) -> Result<(String, ExprDraft), DecodeFault> {
    let masked = mask_comments(source);
    let mut found = None;
    for (at, _) in masked.match_indices("\\NewDocumentCommand") {
        let mut cursor = at + "\\NewDocumentCommand".len();
        let Some((target, next)) = group(&masked, cursor) else {
            continue;
        };
        if masked[target].trim() != format!("\\{name}") {
            continue;
        }
        if found.is_some() {
            return Err(DecodeFault::Duplicate);
        }
        cursor = next;
        let (signature, next) = group(&masked, cursor).ok_or(DecodeFault::Wrapper)?;
        if !masked[signature].trim().is_empty() {
            return Err(DecodeFault::Signature);
        }
        cursor = next;
        let (wrapper, end) = group(&masked, cursor).ok_or(DecodeFault::Wrapper)?;
        let content = masked[wrapper.clone()].trim();
        let prefix = "\\ensuremath";
        if !content.starts_with(prefix) {
            return Err(DecodeFault::Wrapper);
        }
        let body_at = wrapper.start
            + masked[wrapper.clone()]
                .find(prefix)
                .ok_or(DecodeFault::Wrapper)?
            + prefix.len();
        let (body, body_end) = group(&masked, body_at).ok_or(DecodeFault::Wrapper)?;
        if !masked[body_end..wrapper.end].trim().is_empty() {
            return Err(DecodeFault::Wrapper);
        }
        let parsed = parse_catalogue_body(primitives, definitions, masked[body].trim())
            .map_err(|_| DecodeFault::Body)?;
        let admitted = match parsed.draft() {
            ExprDraft::Literal { value } => {
                value.len() == 1 && value.as_bytes()[0].is_ascii_alphabetic()
            }
            ExprDraft::Call {
                callee: super::Callee::Primitive(id),
                arguments,
            } => arguments.is_empty() && id.operation().is_symbol(),
            _ => false,
        };
        if !admitted {
            return Err(DecodeFault::Body);
        }
        let declaration_end = if masked.as_bytes().get(end) == Some(&b'\n') {
            end + 1
        } else {
            end
        };
        found = Some((source[at..declaration_end].to_string(), parsed.into_draft()));
    }
    found.ok_or(DecodeFault::Missing)
}

type DecodedSpecs = (Vec<DefinitionSpec>, Vec<(DefinitionId, String)>);
pub(super) fn specs(
    source_symbols: &str,
    source_historical: &str,
    primitives: &[PrimitiveSpec],
    existing: &[DefinitionSpec],
) -> Result<DecodedSpecs, (DefinitionId, DecodeFault)> {
    let mut definitions = existing.to_vec();
    let mut declarations = Vec::new();
    for &(id, stem, file) in &SYMBOLS {
        let source = match file {
            SourceFile::Symbols => source_symbols,
            SourceFile::Historical => source_historical,
        };
        let (declaration, body) =
            decode(source, id.token(), primitives, &definitions).map_err(|fault| (id, fault))?;
        definitions.push(DefinitionSpec {
            id,
            token: id.token(),
            stem,
            signature: Signature { arity: 0 },
            kind: DefinitionKind::Inherited { body },
        });
        declarations.push((id, declaration));
    }
    Ok((definitions, declarations))
}
