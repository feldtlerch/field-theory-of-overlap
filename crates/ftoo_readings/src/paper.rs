use crate::{
    construction::ReadingBuilder,
    identity::{ReadingId, ReadingPayload, ReadingSpec},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) enum PaperReadingId {
    Direction,
    HarmEnlargement,
    WonderBeauty,
    EnemyToNone,
    RecordLimit,
}

impl PaperReadingId {
    pub(crate) fn spec(self) -> ReadingSpec {
        match self {
            Self::Direction => ReadingSpec::new(
                "read:paper:direction",
                ReadingPayload::Refrain {
                    unit: "sec:say:direction",
                },
            ),
            Self::HarmEnlargement => ReadingSpec::new(
                "read:paper:harm-enlargement",
                ReadingPayload::Refrain {
                    unit: "sec:say:harm-enlargement",
                },
            ),
            Self::WonderBeauty => ReadingSpec::new(
                "read:paper:wonder-beauty",
                ReadingPayload::Refrain {
                    unit: "sec:say:wonder-beauty",
                },
            ),
            Self::EnemyToNone => ReadingSpec::new(
                "read:paper:enemy-to-none",
                ReadingPayload::Refrain {
                    unit: "sec:say:enemy-to-none",
                },
            ),
            Self::RecordLimit => ReadingSpec::new(
                "read:paper:record-limit",
                ReadingPayload::Refrain {
                    unit: "sec:coda:record-limit",
                },
            ),
        }
    }
}

pub(crate) fn populate(builder: &mut ReadingBuilder) {
    builder.allocate(ReadingId::Paper(PaperReadingId::Direction));
    builder.allocate(ReadingId::Paper(PaperReadingId::HarmEnlargement));
    builder.allocate(ReadingId::Paper(PaperReadingId::WonderBeauty));
    builder.allocate(ReadingId::Paper(PaperReadingId::EnemyToNone));
    builder.allocate(ReadingId::Paper(PaperReadingId::RecordLimit));
}
