//! Pure DSP core for the RipplerX physical modelling synthesiser.
//!
//! This crate has no dependency on any plugin framework. Everything here is
//! portable, testable in isolation, and free of allocation on the audio path.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod comb;
pub mod envelope;
pub mod filter;
pub mod limiter;
pub mod lut;
pub mod luts;
pub mod models;
pub mod partial;
pub mod rng;
pub mod utils;
pub mod waveguide;
