//! Constants of Chimithèque, a management application for chemical products,
//! biological reagents and laboratory consumables.
//!
//! It holds the validation regular expressions, the GHS file parsing regular
//! expressions and the reference data loaded into a new database: units,
//! CMR substances, signal words, symbols, suppliers, producers...
#![warn(missing_docs)]

mod defines;

// Re-export all public items from the defines module
pub use crate::defines::*;
