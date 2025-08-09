//! Instruction set for the Palindrome VM
//! 
//! All instructions are designed to be reversible, with each having
//! a clear inverse operation for supporting time-travel debugging.

use crate::vm::Register;

#[derive(Debug, Clone)]
pub enum Instruction {
    // Arithmetic operations (all preserve inputs)
    IAdd { dst: Register, src1: Register, src2: Register },
    ISub { dst: Register, src1: Register, src2: Register },
    IMul { dst: Register, src1: Register, src2: Register },
    IXor { dst: Register, src1: Register, src2: Register },
    
    // Memory operations
    Load { reg: Register, addr: Register },
    Store { addr: Register, reg: Register },
    Swap { addr: Register, reg: Register },
    Push { reg: Register },
    Pop { reg: Register },
    
    // Tape primitive operations
    TapeRead { reg: Register, len: u8 },
    TapeWrite { reg: Register, len: u8 },
    TapeSeek { position: i64 },
    TapeSeekReg { reg: Register },
    TapeAdvance { delta: i64 },
    TapeMark { label: String },
    TapeSeekMark { label: String },
    
    // Segment operations
    SegmentCreate { name: String, size: Register },
    SegmentSeek { name: String, offset: Register },
    SegmentRead { name: String, offset: Register, len: Register, dst: Register },
    SegmentWrite { name: String, offset: Register, len: Register, src: Register },
    
    // Advanced tape operations
    Splice { dst: i64, src: i64, len: Register },
    Compact { start: i64, end: i64 },
    Fork { label: String },
    Merge { strategy: MergeStrategy },
    
    // Control flow
    Call { label: String },
    Return,
    Jump { label: String },
    Branch { condition: Register, label: String },
    BranchZero { reg: Register, label: String },
    BranchNotZero { reg: Register, label: String },
    
    // Time operations
    Checkpoint { label: String },
    Rewind { label: String },
    RewindN { steps: Register },
    
    // Comparison
    Compare { dst: Register, src1: Register, src2: Register },
    Equal { dst: Register, src1: Register, src2: Register },
    LessThan { dst: Register, src1: Register, src2: Register },
    
    // Constants
    LoadImm { reg: Register, value: i64 },
    
    // System
    Halt,
    Nop,
    Debug { message: String },
}

#[derive(Debug, Clone)]
pub enum MergeStrategy {
    Latest,
    Earliest,
    Combine,
    Manual,
}

impl Instruction {
    /// Get the inverse of this instruction
    pub fn inverse(&self) -> Option<Instruction> {
        match self {
            Instruction::IAdd { dst, src1: _, src2 } => 
                Some(Instruction::ISub { dst: *dst, src1: *dst, src2: *src2 }),
            Instruction::ISub { dst, src1: _, src2 } => 
                Some(Instruction::IAdd { dst: *dst, src1: *dst, src2: *src2 }),
            Instruction::IXor { .. } => Some(self.clone()), // Self-inverse
            Instruction::Swap { .. } => Some(self.clone()),  // Self-inverse
            Instruction::Push { reg } => Some(Instruction::Pop { reg: *reg }),
            Instruction::Pop { reg } => Some(Instruction::Push { reg: *reg }),
            Instruction::TapeAdvance { delta } => 
                Some(Instruction::TapeAdvance { delta: -delta }),
            _ => None, // Some instructions need context to reverse
        }
    }
    
    /// Check if instruction modifies state
    pub fn is_stateful(&self) -> bool {
        match self {
            Instruction::Nop | 
            Instruction::Debug { .. } |
            Instruction::Compare { .. } |
            Instruction::Equal { .. } |
            Instruction::LessThan { .. } => false,
            _ => true,
        }
    }
    
    /// Check if instruction is a branch
    pub fn is_branch(&self) -> bool {
        matches!(self,
            Instruction::Jump { .. } |
            Instruction::Branch { .. } |
            Instruction::BranchZero { .. } |
            Instruction::BranchNotZero { .. } |
            Instruction::Call { .. } |
            Instruction::Return
        )
    }
    
    /// Get the size of this instruction in bytes (for future binary encoding)
    pub fn size(&self) -> usize {
        match self {
            Instruction::Nop => 1,
            Instruction::Halt => 1,
            Instruction::Return => 1,
            Instruction::LoadImm { .. } => 10, // 1 byte opcode + 1 byte reg + 8 bytes value
            Instruction::IAdd { .. } | 
            Instruction::ISub { .. } | 
            Instruction::IMul { .. } | 
            Instruction::IXor { .. } => 4, // 1 byte opcode + 3 bytes for registers
            _ => 8, // Default size for now
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inverse_operations() {
        let add = Instruction::IAdd { dst: 0, src1: 1, src2: 2 };
        let inv = add.inverse().unwrap();
        
        match inv {
            Instruction::ISub { dst, src1, src2 } => {
                assert_eq!(dst, 0);
                assert_eq!(src1, 0);
                assert_eq!(src2, 2);
            }
            _ => panic!("Wrong inverse operation"),
        }
    }

    #[test]
    fn test_self_inverse() {
        let xor = Instruction::IXor { dst: 0, src1: 1, src2: 2 };
        let inv = xor.inverse().unwrap();
        
        match (&xor, &inv) {
            (Instruction::IXor { .. }, Instruction::IXor { .. }) => {
                // Should be identical
                assert!(matches!(inv, Instruction::IXor { dst: 0, src1: 1, src2: 2 }));
            }
            _ => panic!("XOR should be self-inverse"),
        }
    }

    #[test]
    fn test_stateful_check() {
        assert!(Instruction::IAdd { dst: 0, src1: 1, src2: 2 }.is_stateful());
        assert!(!Instruction::Nop.is_stateful());
        assert!(!Instruction::Compare { dst: 0, src1: 1, src2: 2 }.is_stateful());
    }

    #[test]
    fn test_branch_check() {
        assert!(Instruction::Jump { label: "test".to_string() }.is_branch());
        assert!(Instruction::Call { label: "func".to_string() }.is_branch());
        assert!(!Instruction::IAdd { dst: 0, src1: 1, src2: 2 }.is_branch());
    }
}