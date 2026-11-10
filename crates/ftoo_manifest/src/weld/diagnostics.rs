//! Complete occurrence evidence for unit citations and language inequalities.
use crate::{LabelSite, NodeId, ReferenceDomain, RuleId, UnitSite};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagMember {
    pub input: String,
    pub target: NodeId,
    pub sites: Vec<UnitSite>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelMember {
    pub label: String,
    pub sites: Vec<LabelUseSite>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelUseSite {
    pub span_index: usize,
    pub site: UnitSite,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagSide {
    pub language: String,
    pub text_site: UnitSite,
    pub members: Vec<TagMember>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LabelSide {
    pub language: String,
    pub text_site: UnitSite,
    pub members: Vec<LabelMember>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WeldStage {
    UnitCitation,
    Equality,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeldFault {
    UnresolvedUnitCitation {
        label: String,
        span_index: usize,
        expected: usize,
        actual: usize,
        domain: ReferenceDomain,
    },
    TagSetMismatch {
        reference: TagSide,
        actual: TagSide,
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
    LabelSetMismatch {
        reference: LabelSide,
        actual: LabelSide,
        missing: Vec<String>,
        unexpected: Vec<String>,
    },
}
impl WeldFault {
    fn rank(&self) -> usize {
        match self {
            Self::UnresolvedUnitCitation { .. } => 0,
            Self::TagSetMismatch { .. } => 1,
            Self::LabelSetMismatch { .. } => 2,
        }
    }
}
impl fmt::Display for WeldFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeldDiagnostic {
    pub rule: RuleId,
    pub stage: WeldStage,
    pub unit: Option<String>,
    pub language: Option<String>,
    pub site: LabelSite,
    pub fault: WeldFault,
}
impl fmt::Display for WeldDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?} unit={:?} language={:?} at {}: {}",
            self.rule, self.stage, self.unit, self.language, self.site, self.fault
        )
    }
}
/// Nonempty immutable failures from one complete checking phase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeldErrors(Vec<WeldDiagnostic>);
impl WeldErrors {
    pub(super) fn collect(mut diagnostics: Vec<WeldDiagnostic>) -> Option<Self> {
        diagnostics.sort_by(|a, b| {
            let site_key = |d: &WeldDiagnostic| match &d.site {
                LabelSite::Unit { site, .. } => (
                    site.file.clone(),
                    site.path.clone(),
                    site.decoded.as_ref().map(|r| r.start),
                ),
                _ => unreachable!("weld diagnostics only describe unit texts"),
            };
            (&a.rule, &a.unit, &a.language, site_key(a), a.fault.rank()).cmp(&(
                &b.rule,
                &b.unit,
                &b.language,
                site_key(b),
                b.fault.rank(),
            ))
        });
        (!diagnostics.is_empty()).then_some(Self(diagnostics))
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[WeldDiagnostic] {
        &self.0
    }
}
impl fmt::Display for WeldErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.0 {
            writeln!(f, "{diagnostic}")?;
        }
        Ok(())
    }
}
impl std::error::Error for WeldErrors {}
