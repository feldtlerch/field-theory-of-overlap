//! Generate and check the complete manifest artifact against Rust authority.
//! Semantic comparison precedes registry admission and exact byte comparison.
mod artifact;
mod projection;
#[cfg(test)]
mod tests;
pub use artifact::{
    ByteGateFault, IoOperation, ProjectionCheckError, check_projection, generate_projection,
};
