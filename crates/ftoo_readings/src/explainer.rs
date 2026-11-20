use crate::{
    construction::ReadingBuilder,
    identity::{ReadingId, ReadingPayload, ReadingSpec},
    paper::PaperReadingId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum ExplainerReadingId {
    Direction,
    EnemyToNone,
    RecordLimit,
}

impl ExplainerReadingId {
    pub(crate) fn spec(self) -> ReadingSpec {
        match self {
            Self::Direction => ReadingSpec::new(
                "read:explainer:direction",
                ReadingPayload::Quotation {
                    owner: ReadingId::Paper(PaperReadingId::Direction),
                },
            ),
            Self::EnemyToNone => ReadingSpec::new(
                "read:explainer:enemy-to-none",
                ReadingPayload::Quotation {
                    owner: ReadingId::Paper(PaperReadingId::EnemyToNone),
                },
            ),
            Self::RecordLimit => ReadingSpec::new(
                "read:explainer:record-limit",
                ReadingPayload::Quotation {
                    owner: ReadingId::Paper(PaperReadingId::RecordLimit),
                },
            ),
        }
    }
}

pub(crate) fn populate(builder: &mut ReadingBuilder) {
    builder.allocate(ReadingId::Explainer(ExplainerReadingId::Direction));
    builder.allocate(ReadingId::Explainer(ExplainerReadingId::EnemyToNone));
    builder.allocate(ReadingId::Explainer(ExplainerReadingId::RecordLimit));
}
