//! Closed implementations: callers cannot replace descriptors or functions.
use crate::{Measurement, Receipt, RunError, theory};
use ftoo_manifest::{ProducerDescriptor, ProducerId};
#[derive(Clone, Copy)]
pub(crate) struct Entry {
    pub descriptor: ProducerDescriptor,
    pub produce: fn() -> Result<(Receipt, Measurement), RunError>,
}
pub(crate) const REGISTRY: [Entry; 2] = [
    Entry {
        descriptor: ProducerId::LangevinCoefficientOneValue.descriptor(),
        produce: theory::coefficient_one,
    },
    Entry {
        descriptor: ProducerId::LangevinCoefficientThreeValue.descriptor(),
        produce: theory::coefficient_three,
    },
];
