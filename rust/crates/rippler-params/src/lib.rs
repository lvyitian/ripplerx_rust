//! Parameter definitions and legacy state compatibility for RipplerX.
//!
//! Everything that has to stay compatible with the JUCE implementation lives
//! here: parameter identifiers, value ranges, and the legacy XML reader.
//!
//! The crate denies missing documentation, but it cannot forbid `unsafe`:
//! nice-plug's `Params` trait is an unsafe trait, so every implementation of it
//! needs an `unsafe impl`. The only `unsafe` in the crate comes from that
//! derive macro.

#![deny(missing_docs)]

pub mod enums;
pub mod params;
pub mod ranges;
