//! Virtual Machine implementation for Palindrome
//! 
//! The VM executes instructions on the global tape with full reversibility support.

mod executor;
mod registers;

pub use executor::{VM, ExecutionHistory, HistoryFrame, Timeline};
pub use registers::{RegisterFile, Flags};

// Re-export register type
pub type Register = registers::Register;