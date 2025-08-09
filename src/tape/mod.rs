//! Tape module - The fundamental data structure of Palindrome VM
//! 
//! Everything in the VM is stored on an infinite bidirectional tape.
//! The tape supports reversible operations through a history trail.

mod core;
mod segment;

pub use core::{Tape, Page, Trail, TrailOp};
pub use segment::{Segment, SegmentedTape, SegmentType, Schema, Field, DataType, Index, IndexType};