//! Palindrome VM - A reversible virtual machine with unified tape abstraction
//! 
//! This VM eliminates traditional software complexity by making everything reversible
//! and storing all data on an infinite tape that serves as memory, storage, and history.

pub mod tape;
pub mod vm;
pub mod instruction;
pub mod compiler;

pub use tape::{Tape, Segment, SegmentType};
pub use vm::{VM, Register};
pub use instruction::Instruction;
pub use compiler::Parser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_creation() {
        let vm = VM::new();
        assert_eq!(vm.ip, 0);
    }
}