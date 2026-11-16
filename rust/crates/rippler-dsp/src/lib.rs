//! Pure DSP core for the RipplerX physical modelling synthesiser.
//!
//! This crate has no dependency on any plugin framework. Everything here is
//! portable, testable in isolation, and free of allocation on the audio path.

pub mod envelope;
pub mod filter;
pub mod lut;
pub mod rng;
