//! Open questions: what the repository publishes as unsettled, as closed, typed records.
//!
//! A record states one question, what it is about and what would settle it. Its
//! vocabulary is closed (M066), its store is strict and ordered (M067), its labels are
//! reserved to the store (M068), every reference resolves in its own domain (M069) and
//! its lifecycle relations are acyclic with typed closures (M070).
mod diagnostics;
#[cfg(test)]
mod tests;
use crate::{LocatedString, NodeId, RULES, RuleId, UnitSite};
pub use diagnostics::*;
use std::collections::{BTreeMap, BTreeSet};

/// What kind of change settles a question.
/// ```compile_fail,E0599
/// let _ = ftoo_manifest::QuestionConcern::Other;
/// ```
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum QuestionConcern {
    Theory,
    Model,
    Documents,
    Machinery,
}
impl QuestionConcern {
    pub const ALL: [Self; 4] = [Self::Theory, Self::Model, Self::Documents, Self::Machinery];
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Theory => "theory",
            Self::Model => "model",
            Self::Documents => "documents",
            Self::Machinery => "machinery",
        }
    }
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|concern| concern.name() == value)
    }
}
/// The domains a typed reference can name; each resolves in its own domain only.
/// ```compile_fail,E0599
/// let _ = ftoo_manifest::ReferenceKind::Url;
/// ```
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReferenceKind {
    Node,
    Rule,
    Unit,
    Record,
    Paper,
    File,
    Open,
}
impl ReferenceKind {
    pub const ALL: [Self; 7] = [
        Self::Node,
        Self::Rule,
        Self::Unit,
        Self::Record,
        Self::Paper,
        Self::File,
        Self::Open,
    ];
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Rule => "rule",
            Self::Unit => "unit",
            Self::Record => "record",
            Self::Paper => "paper",
            Self::File => "file",
            Self::Open => "open",
        }
    }
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == value)
    }
}
/// The lifecycle states; every state other than open is terminal.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum QuestionStateKind {
    Open,
    Settled,
    Succeeded,
    Withdrawn,
}
impl QuestionStateKind {
    pub const ALL: [Self; 4] = [Self::Open, Self::Settled, Self::Succeeded, Self::Withdrawn];
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Settled => "settled",
            Self::Succeeded => "succeeded",
            Self::Withdrawn => "withdrawn",
        }
    }
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|state| state.name() == value)
    }
}
/// One resolved reference; graph nodes and rules are compiled identities.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum QuestionReference {
    Node(NodeId),
    Rule(RuleId),
    Unit(String),
    Record(String),
    Paper(String),
    File(String),
    Open(String),
}
impl QuestionReference {
    #[must_use]
    pub const fn kind(&self) -> ReferenceKind {
        match self {
            Self::Node(_) => ReferenceKind::Node,
            Self::Rule(_) => ReferenceKind::Rule,
            Self::Unit(_) => ReferenceKind::Unit,
            Self::Record(_) => ReferenceKind::Record,
            Self::Paper(_) => ReferenceKind::Paper,
            Self::File(_) => ReferenceKind::File,
            Self::Open(_) => ReferenceKind::Open,
        }
    }
    #[must_use]
    pub fn value(&self) -> String {
        match self {
            Self::Node(id) => id.semantic_name().to_owned(),
            Self::Rule(rule) => format!("{rule:?}"),
            Self::Unit(value)
            | Self::Record(value)
            | Self::Paper(value)
            | Self::File(value)
            | Self::Open(value) => value.clone(),
        }
    }
}
/// An untrusted reference as authored: its kind key and its value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceDraft {
    pub kind: ReferenceKind,
    pub value: LocatedString,
}
/// An untrusted lifecycle state with the fields its kind carries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateDraft {
    Open,
    Settled {
        by: Vec<ReferenceDraft>,
        answer: LocatedString,
    },
    Succeeded {
        by: Vec<ReferenceDraft>,
        answer: Option<LocatedString>,
    },
    Withdrawn {
        because: LocatedString,
    },
}
impl StateDraft {
    #[must_use]
    pub const fn kind(&self) -> QuestionStateKind {
        match self {
            Self::Open => QuestionStateKind::Open,
            Self::Settled { .. } => QuestionStateKind::Settled,
            Self::Succeeded { .. } => QuestionStateKind::Succeeded,
            Self::Withdrawn { .. } => QuestionStateKind::Withdrawn,
        }
    }
}
/// Untrusted record fields; only admission turns them into an [`OpenQuestion`].
/// An empty optional list stands for an absent field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenQuestionDraft {
    pub site: UnitSite,
    pub label: LocatedString,
    pub title: LocatedString,
    pub concern: QuestionConcern,
    pub question: LocatedString,
    pub about: Vec<ReferenceDraft>,
    pub settled_when: LocatedString,
    pub blocks: Vec<ReferenceDraft>,
    pub rests_on: Vec<LocatedString>,
    pub state: StateDraft,
}
/// The resolution domains of the references that are not compiled identities.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QuestionDomains {
    /// Markdown mints: decision records, policy, glossary and confirmation records.
    pub records: BTreeSet<String>,
    /// Labels of the admitted unit corpus.
    pub units: BTreeSet<String>,
    /// The paper's tag targets and label-mint keys.
    pub paper: BTreeSet<String>,
    /// Repository paths checked to be regular files reached without symlinks.
    pub files: BTreeSet<String>,
}
/// A settlement: the repository objects that settle the question and its answer.
/// ```compile_fail,E0451
/// let _ = ftoo_manifest::Settlement { by: vec![], answer: String::new() };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settlement {
    by: Vec<QuestionReference>,
    answer: String,
}
impl Settlement {
    #[must_use]
    pub fn by(&self) -> &[QuestionReference] {
        &self.by
    }
    #[must_use]
    pub fn answer(&self) -> &str {
        &self.answer
    }
}
/// A succession: where the question now lives, with the part answered, if any.
/// ```compile_fail,E0451
/// let _ = ftoo_manifest::Succession { by: vec![], answer: None };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Succession {
    by: Vec<QuestionReference>,
    answer: Option<String>,
}
impl Succession {
    #[must_use]
    pub fn by(&self) -> &[QuestionReference] {
        &self.by
    }
    #[must_use]
    pub fn answer(&self) -> Option<&str> {
        self.answer.as_deref()
    }
}
/// A withdrawal: why the question was ill-posed.
/// ```compile_fail,E0451
/// let _ = ftoo_manifest::Withdrawal { because: String::new() };
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Withdrawal {
    because: String,
}
impl Withdrawal {
    #[must_use]
    pub fn because(&self) -> &str {
        &self.because
    }
}
/// A closed state carries its content; no closed state exists without it.
/// ```compile_fail,E0308
/// let _: ftoo_manifest::QuestionState = ftoo_manifest::QuestionState::Settled;
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuestionState {
    Open,
    Settled(Settlement),
    Succeeded(Succession),
    Withdrawn(Withdrawal),
}
impl QuestionState {
    #[must_use]
    pub const fn kind(&self) -> QuestionStateKind {
        match self {
            Self::Open => QuestionStateKind::Open,
            Self::Settled(_) => QuestionStateKind::Settled,
            Self::Succeeded(_) => QuestionStateKind::Succeeded,
            Self::Withdrawn(_) => QuestionStateKind::Withdrawn,
        }
    }
}
/// One admitted record; immutable, so no operation reopens a closed record.
/// ```compile_fail,E0616
/// fn reopen(q: &mut ftoo_manifest::OpenQuestion) { q.state = ftoo_manifest::QuestionState::Open; }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenQuestion {
    label: String,
    title: String,
    concern: QuestionConcern,
    question: String,
    about: Vec<QuestionReference>,
    settled_when: String,
    blocks: Vec<QuestionReference>,
    rests_on: Vec<String>,
    state: QuestionState,
}
impl OpenQuestion {
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
    #[must_use]
    pub const fn concern(&self) -> QuestionConcern {
        self.concern
    }
    #[must_use]
    pub fn question(&self) -> &str {
        &self.question
    }
    #[must_use]
    pub fn about(&self) -> &[QuestionReference] {
        &self.about
    }
    #[must_use]
    pub fn settled_when(&self) -> &str {
        &self.settled_when
    }
    #[must_use]
    pub fn blocks(&self) -> &[QuestionReference] {
        &self.blocks
    }
    #[must_use]
    pub fn rests_on(&self) -> &[String] {
        &self.rests_on
    }
    #[must_use]
    pub const fn state(&self) -> &QuestionState {
        &self.state
    }
}
/// The admitted store in label order.
/// ```compile_fail,E0599
/// let _ = ftoo_manifest::OpenQuestions::default();
/// ```
/// ```compile_fail,E0277
/// fn deserialize<T: serde::de::DeserializeOwned>() {}
/// deserialize::<ftoo_manifest::OpenQuestions>();
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenQuestions {
    file: String,
    records: Vec<OpenQuestion>,
}
impl OpenQuestions {
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }
    #[must_use]
    pub fn records(&self) -> &[OpenQuestion] {
        &self.records
    }
}

fn diagnostic(
    rule: RuleId,
    stage: QuestionStage,
    site: &UnitSite,
    fault: QuestionFault,
) -> QuestionDiagnostic {
    QuestionDiagnostic {
        rule,
        stage,
        site: site.clone(),
        fault,
    }
}
fn phase(errors: Vec<QuestionDiagnostic>) -> Result<(), QuestionErrors> {
    QuestionErrors::from_diagnostics(errors).map_or(Ok(()), Err)
}
fn boundary(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.windows(3).any(|window| {
        matches!(window[0], b'.' | b'?' | b'!')
            && window[1] == b' '
            && window[2].is_ascii_uppercase()
    })
}
fn text_reason(text: QuestionText, value: &str) -> Option<TextReason> {
    let last = value.chars().last();
    let terminal = match text {
        QuestionText::Title => !matches!(last, Some('.' | '?' | '!' | ':' | ';' | ',')),
        QuestionText::Question => last == Some('?'),
        QuestionText::SettledWhen | QuestionText::Answer | QuestionText::WithdrawnBecause => {
            last == Some('.')
        }
    };
    if value.trim().is_empty() {
        Some(TextReason::Blank)
    } else if value.trim() != value {
        Some(TextReason::Padded)
    } else if value.chars().any(char::is_control) {
        Some(TextReason::ControlCharacter)
    } else if value.contains(['`', '|']) {
        Some(TextReason::ForbiddenCharacter)
    } else if !terminal {
        Some(TextReason::Terminal)
    } else if boundary(value) || (text == QuestionText::Question && value.matches('?').count() != 1)
    {
        Some(TextReason::SeveralSentences)
    } else {
        None
    }
}
fn normalized(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn texts(draft: &OpenQuestionDraft) -> Vec<(QuestionText, &LocatedString)> {
    let mut texts = vec![
        (QuestionText::Title, &draft.title),
        (QuestionText::Question, &draft.question),
        (QuestionText::SettledWhen, &draft.settled_when),
    ];
    match &draft.state {
        StateDraft::Open => {}
        StateDraft::Settled { answer, .. } => texts.push((QuestionText::Answer, answer)),
        StateDraft::Succeeded { answer, .. } => {
            texts.extend(answer.iter().map(|answer| (QuestionText::Answer, answer)));
        }
        StateDraft::Withdrawn { because } => texts.push((QuestionText::WithdrawnBecause, because)),
    }
    texts
}
fn lists(draft: &OpenQuestionDraft) -> Vec<(QuestionList, &[ReferenceDraft])> {
    let mut lists = vec![
        (QuestionList::About, draft.about.as_slice()),
        (QuestionList::Blocks, draft.blocks.as_slice()),
    ];
    match &draft.state {
        StateDraft::Settled { by, .. } => lists.push((QuestionList::SettledBy, by.as_slice())),
        StateDraft::Succeeded { by, .. } => lists.push((QuestionList::SucceededBy, by.as_slice())),
        StateDraft::Open | StateDraft::Withdrawn { .. } => {}
    }
    lists
}
#[allow(clippy::too_many_lines)] // One pass in record order keeps every store fault beside its record.
fn store(drafts: &[OpenQuestionDraft]) -> Result<(), QuestionErrors> {
    let rule = RuleId::M067;
    let step = QuestionStage::Store;
    let mut errors = vec![];
    let mut labels = BTreeMap::<&str, usize>::new();
    let mut titles = BTreeMap::<&str, usize>::new();
    let mut questions = BTreeMap::<String, usize>::new();
    for (index, draft) in drafts.iter().enumerate() {
        let label = &draft.label.value;
        if !crate::units::label_name(label) || !crate::labels::open_kind(label) {
            errors.push(diagnostic(
                rule,
                step,
                &draft.label.site,
                QuestionFault::InvalidLabel {
                    label: label.clone(),
                },
            ));
        }
        if let Some(first) = labels.get(label.as_str()) {
            errors.push(diagnostic(
                rule,
                step,
                &draft.label.site,
                QuestionFault::DuplicateLabel {
                    label: label.clone(),
                    first: *first,
                },
            ));
        } else {
            if let Some(previous) = index
                .checked_sub(1)
                .map(|previous| &drafts[previous].label.value)
                .filter(|previous| label < *previous)
            {
                errors.push(diagnostic(
                    rule,
                    step,
                    &draft.label.site,
                    QuestionFault::LabelOrder {
                        label: label.clone(),
                        previous: previous.clone(),
                    },
                ));
            }
            labels.insert(label, index);
        }
        for (text, value) in texts(draft) {
            if let Some(reason) = text_reason(text, &value.value) {
                errors.push(diagnostic(
                    rule,
                    step,
                    &value.site,
                    QuestionFault::Text {
                        text,
                        value: value.value.clone(),
                        reason,
                    },
                ));
            }
        }
        if let Some(first) = titles.get(draft.title.value.as_str()) {
            errors.push(diagnostic(
                rule,
                step,
                &draft.title.site,
                QuestionFault::DuplicateTitle {
                    title: draft.title.value.clone(),
                    first: *first,
                },
            ));
        } else {
            titles.insert(&draft.title.value, index);
        }
        let question = normalized(&draft.question.value);
        if let Some(first) = questions.get(&question) {
            errors.push(diagnostic(
                rule,
                step,
                &draft.question.site,
                QuestionFault::DuplicateQuestion {
                    question,
                    first: *first,
                },
            ));
        } else {
            questions.insert(question, index);
        }
        for (list, references) in lists(draft) {
            if references.is_empty()
                && matches!(list, QuestionList::SettledBy | QuestionList::SucceededBy)
            {
                errors.push(diagnostic(
                    rule,
                    step,
                    &draft.site.child(list.field()),
                    QuestionFault::EmptyList { list },
                ));
            }
            let mut seen = BTreeSet::new();
            for reference in references {
                if !seen.insert((reference.kind, reference.value.value.as_str())) {
                    errors.push(diagnostic(
                        rule,
                        step,
                        &reference.value.site,
                        QuestionFault::RepeatedReference {
                            list,
                            kind: reference.kind,
                            value: reference.value.value.clone(),
                        },
                    ));
                }
            }
        }
        let mut seen = BTreeSet::new();
        for label in &draft.rests_on {
            if !seen.insert(label.value.as_str()) {
                errors.push(diagnostic(
                    rule,
                    step,
                    &label.site,
                    QuestionFault::RepeatedReference {
                        list: QuestionList::RestsOn,
                        kind: ReferenceKind::Open,
                        value: label.value.clone(),
                    },
                ));
            }
        }
    }
    phase(errors)
}
fn well_formed_path(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains(['\\', '|', '`', ' '])
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".."))
}
struct Resolver<'a> {
    domains: &'a QuestionDomains,
    labels: BTreeSet<&'a str>,
}
impl Resolver<'_> {
    fn resolve(
        &self,
        own: &str,
        list: QuestionList,
        reference: &ReferenceDraft,
        errors: &mut Vec<QuestionDiagnostic>,
    ) -> Option<QuestionReference> {
        let value = &reference.value.value;
        let fault = |fault| {
            diagnostic(
                RuleId::M069,
                QuestionStage::References,
                &reference.value.site,
                fault,
            )
        };
        let unresolved = || {
            fault(QuestionFault::UnresolvedReference {
                list,
                kind: reference.kind,
                value: value.clone(),
            })
        };
        let known = |domain: &BTreeSet<String>| domain.contains(value).then(|| value.clone());
        let resolved = match reference.kind {
            ReferenceKind::Node => NodeId::ALL
                .into_iter()
                .find(|id| id.semantic_name() == value)
                .map(QuestionReference::Node),
            ReferenceKind::Rule => RULES
                .iter()
                .map(|rule| rule.number)
                .find(|number| format!("{number:?}") == *value)
                .map(QuestionReference::Rule),
            ReferenceKind::Unit => known(&self.domains.units).map(QuestionReference::Unit),
            ReferenceKind::Record => known(&self.domains.records).map(QuestionReference::Record),
            ReferenceKind::Paper => known(&self.domains.paper).map(QuestionReference::Paper),
            ReferenceKind::File => {
                if !well_formed_path(value) {
                    errors.push(fault(QuestionFault::MalformedPath {
                        list,
                        value: value.clone(),
                    }));
                    return None;
                }
                known(&self.domains.files).map(QuestionReference::File)
            }
            ReferenceKind::Open => {
                if list == QuestionList::Blocks {
                    errors.push(fault(QuestionFault::OpenInBlocks {
                        value: value.clone(),
                    }));
                    return None;
                }
                if value == own {
                    errors.push(fault(QuestionFault::SelfReference { list }));
                    return None;
                }
                self.labels
                    .contains(value.as_str())
                    .then(|| QuestionReference::Open(value.clone()))
            }
        };
        if resolved.is_none() {
            errors.push(unresolved());
        }
        resolved
    }
}
struct Resolved {
    about: Vec<QuestionReference>,
    blocks: Vec<QuestionReference>,
    closure: Vec<QuestionReference>,
}
fn references(
    drafts: &[OpenQuestionDraft],
    domains: &QuestionDomains,
) -> Result<Vec<Resolved>, QuestionErrors> {
    let resolver = Resolver {
        domains,
        labels: drafts.iter().map(|d| d.label.value.as_str()).collect(),
    };
    let mut errors = vec![];
    let mut records = vec![];
    for draft in drafts {
        let own = draft.label.value.as_str();
        if draft.about.is_empty() {
            errors.push(diagnostic(
                RuleId::M069,
                QuestionStage::References,
                &draft.site.child(QuestionList::About.field()),
                QuestionFault::EmptyList {
                    list: QuestionList::About,
                },
            ));
        }
        let mut record = Resolved {
            about: vec![],
            blocks: vec![],
            closure: vec![],
        };
        for (list, references) in lists(draft) {
            for reference in references {
                if let Some(value) = resolver.resolve(own, list, reference, &mut errors) {
                    match list {
                        QuestionList::About => record.about.push(value),
                        QuestionList::Blocks => record.blocks.push(value),
                        QuestionList::SettledBy | QuestionList::SucceededBy => {
                            record.closure.push(value);
                        }
                        QuestionList::RestsOn => unreachable!("rests-on holds labels"),
                    }
                }
            }
        }
        for label in &draft.rests_on {
            let reference = ReferenceDraft {
                kind: ReferenceKind::Open,
                value: label.clone(),
            };
            resolver.resolve(own, QuestionList::RestsOn, &reference, &mut errors);
        }
        records.push(record);
    }
    phase(errors)?;
    Ok(records)
}
/// Members of every cycle of a successor relation, grouped and sorted, in label order.
fn cycles(edges: &BTreeMap<&str, Vec<&str>>) -> Vec<Vec<String>> {
    let reach = |start: &str| {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&str> = edges.get(start).cloned().unwrap_or_default();
        while let Some(next) = stack.pop() {
            if seen.insert(next) {
                stack.extend(edges.get(next).into_iter().flatten().copied());
            }
        }
        seen
    };
    let reaches: BTreeMap<&str, BTreeSet<&str>> =
        edges.keys().map(|node| (*node, reach(node))).collect();
    let mut grouped = BTreeSet::new();
    let mut groups = vec![];
    for (node, reached) in &reaches {
        if !reached.contains(node) || grouped.contains(node) {
            continue;
        }
        let group: Vec<&str> = reached
            .iter()
            .copied()
            .filter(|other| reaches.get(other).is_some_and(|back| back.contains(node)))
            .collect();
        grouped.extend(group.iter().copied());
        groups.push(group.into_iter().map(str::to_owned).collect());
    }
    groups
}
fn lifecycle(drafts: &[OpenQuestionDraft], resolved: &[Resolved]) -> Result<(), QuestionErrors> {
    let rule = RuleId::M070;
    let step = QuestionStage::Lifecycle;
    let mut errors = vec![];
    let states: BTreeMap<&str, QuestionStateKind> = drafts
        .iter()
        .map(|d| (d.label.value.as_str(), d.state.kind()))
        .collect();
    let mut rests = BTreeMap::<&str, Vec<&str>>::new();
    let mut successors = BTreeMap::<&str, Vec<&str>>::new();
    let sites: BTreeMap<&str, &UnitSite> = drafts
        .iter()
        .map(|d| (d.label.value.as_str(), &d.site))
        .collect();
    for (draft, record) in drafts.iter().zip(resolved) {
        let own = draft.label.value.as_str();
        rests.insert(
            own,
            draft.rests_on.iter().map(|l| l.value.as_str()).collect(),
        );
        successors.insert(own, vec![]);
        if let StateDraft::Settled { by, .. } | StateDraft::Succeeded { by, .. } = &draft.state {
            let settled = matches!(draft.state, StateDraft::Settled { .. });
            for reference in by {
                let admitted = if settled {
                    !matches!(reference.kind, ReferenceKind::File | ReferenceKind::Open)
                } else {
                    matches!(reference.kind, ReferenceKind::Open | ReferenceKind::Node)
                };
                if !admitted {
                    errors.push(diagnostic(
                        rule,
                        step,
                        &reference.value.site,
                        if settled {
                            QuestionFault::SettlementKind {
                                kind: reference.kind,
                            }
                        } else {
                            QuestionFault::SuccessionKind {
                                kind: reference.kind,
                            }
                        },
                    ));
                }
            }
            if !settled {
                successors.insert(
                    own,
                    record
                        .closure
                        .iter()
                        .filter_map(|reference| match reference {
                            QuestionReference::Open(label) => Some(label.as_str()),
                            _ => None,
                        })
                        .collect(),
                );
            }
        }
        if draft.state.kind() == QuestionStateKind::Open {
            for label in &draft.rests_on {
                let Some(&state) = states.get(label.value.as_str()) else {
                    continue;
                };
                if state != QuestionStateKind::Open {
                    errors.push(diagnostic(
                        rule,
                        step,
                        &label.site,
                        QuestionFault::RestsOnClosed {
                            label: label.value.clone(),
                            state,
                        },
                    ));
                }
            }
        }
    }
    for (list, edges) in [
        (QuestionList::RestsOn, &rests),
        (QuestionList::SucceededBy, &successors),
    ] {
        for labels in cycles(edges) {
            errors.push(diagnostic(
                rule,
                step,
                &sites[labels[0].as_str()].child(list.field()),
                QuestionFault::Cycle { list, labels },
            ));
        }
    }
    phase(errors)
}
/// Admit the complete store: shape and texts, then references, then lifecycle.
/// # Errors
/// Returns every fault of the earliest failing phase, without a partial store.
pub fn admit_open_questions(
    file: &str,
    drafts: Vec<OpenQuestionDraft>,
    domains: &QuestionDomains,
) -> Result<OpenQuestions, QuestionErrors> {
    store(&drafts)?;
    let resolved = references(&drafts, domains)?;
    lifecycle(&drafts, &resolved)?;
    let records = drafts
        .into_iter()
        .zip(resolved)
        .map(|(draft, resolved)| {
            let state = match draft.state {
                StateDraft::Open => QuestionState::Open,
                StateDraft::Settled { answer, .. } => QuestionState::Settled(Settlement {
                    by: resolved.closure,
                    answer: answer.value,
                }),
                StateDraft::Succeeded { answer, .. } => QuestionState::Succeeded(Succession {
                    by: resolved.closure,
                    answer: answer.map(|answer| answer.value),
                }),
                StateDraft::Withdrawn { because } => QuestionState::Withdrawn(Withdrawal {
                    because: because.value,
                }),
            };
            OpenQuestion {
                label: draft.label.value,
                title: draft.title.value,
                concern: draft.concern,
                question: draft.question.value,
                about: resolved.about,
                settled_when: draft.settled_when.value,
                blocks: resolved.blocks,
                rests_on: draft.rests_on.into_iter().map(|l| l.value).collect(),
                state,
            }
        })
        .collect();
    Ok(OpenQuestions {
        file: file.to_owned(),
        records,
    })
}
