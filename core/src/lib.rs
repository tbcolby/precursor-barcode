//! Pure logic for Barcode Generator: encoders and the persisted model.
//! No Xous dependencies; everything here is unit-tested on the host.

#![no_std]
extern crate alloc;

pub mod encode;
pub mod model;

pub use encode::{auto_detect, check, encode, fit_module_px, Barcode, EncodeError, Format};
pub use model::{Codes, IndexRecord, LegacyCode, SaveError, SavedCode, Settings, TextField};

#[cfg(test)]
mod tests;
