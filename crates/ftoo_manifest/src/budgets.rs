//! Pin-specific immutable profiles, resolved and exhaustively checked by the adapter.
use crate::{ENGINE_PIN, EnginePin, ProducerId};
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BudgetId {
    CoefficientOneV1,
    CoefficientThreeV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetProfile {
    PinnedTrialLimitsV1,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetSpec {
    pub id: BudgetId,
    pub producer: ProducerId,
    pub profile: BudgetProfile,
    pub pin: EnginePin,
}
impl BudgetSpec {
    #[must_use]
    pub const fn reviewed(producer: ProducerId) -> Self {
        Self {
            id: producer.descriptor().budget,
            producer,
            profile: BudgetProfile::PinnedTrialLimitsV1,
            pin: ENGINE_PIN,
        }
    }
}

impl BudgetProfile {
    /// Complete-route measurement at `ENGINE_PIN`: 6,073,177 for both targets.
    /// Round up to 500,000 (6,500,000), then step by 500,000 until headroom is at least 25%.
    #[must_use]
    pub const fn work(self) -> u64 {
        match self {
            Self::PinnedTrialLimitsV1 => 8_000_000,
        }
    }
    /// Combined replay measurement: 3,465,362 for both targets.
    /// Round up to 500,000 (3,500,000), then step by 500,000 until headroom is at least 25%.
    #[must_use]
    pub const fn replay_work(self) -> u64 {
        match self {
            Self::PinnedTrialLimitsV1 => 4_500_000,
        }
    }
}
