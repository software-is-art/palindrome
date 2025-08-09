//! VM executor - the heart of the Palindrome VM

use crate::tape::{SegmentedTape, SegmentType};
use crate::instruction::Instruction;
use crate::vm::registers::RegisterFile;
use std::collections::HashMap;

/// The main VM structure
pub struct VM {
    /// The global tape (with segments)
    pub tape: SegmentedTape,
    /// Register file
    pub registers: RegisterFile,
    /// Instruction pointer (position on tape)
    pub ip: i64,
    /// Stack pointer (position on tape)
    pub sp: i64,
    /// Frame pointer
    pub fp: i64,
    /// Execution history
    pub history: ExecutionHistory,
    /// Parallel timelines (for fork/merge)
    pub timelines: HashMap<String, Timeline>,
    /// Current timeline
    pub current_timeline: String,
    /// Symbol table for labels
    pub symbols: HashMap<String, i64>,
}

/// Execution history for reversibility
pub struct ExecutionHistory {
    /// Stack of executed instructions with saved state
    pub stack: Vec<HistoryFrame>,
    /// Named checkpoints
    pub checkpoints: HashMap<String, usize>,
}

/// A single frame in the execution history
#[derive(Clone)]
pub struct HistoryFrame {
    pub instruction: Instruction,
    pub registers_before: RegisterFile,
    pub ip_before: i64,
    pub sp_before: i64,
    pub fp_before: i64,
    pub tape_trail_len: usize,
}

/// A parallel timeline (for fork operations)
#[derive(Clone)]
pub struct Timeline {
    pub tape: SegmentedTape,
    pub registers: RegisterFile,
    pub ip: i64,
    pub sp: i64,
    pub fp: i64,
}

impl VM {
    pub fn new() -> Self {
        let mut tape = SegmentedTape::new();
        
        // Initialize standard segments
        tape.create_segment("code".to_string(), 1024 * 1024, SegmentType::Code)
            .expect("Failed to create code segment");
        tape.create_segment("stack".to_string(), 1024 * 1024, SegmentType::Stack)
            .expect("Failed to create stack segment");
        tape.create_segment("heap".to_string(), 1024 * 1024, SegmentType::Heap)
            .expect("Failed to create heap segment");
        
        VM {
            tape,
            registers: RegisterFile::new(),
            ip: 0,
            sp: 1024 * 1024, // Stack starts at 1MB
            fp: 1024 * 1024,
            history: ExecutionHistory::new(),
            timelines: HashMap::new(),
            current_timeline: "main".to_string(),
            symbols: HashMap::new(),
        }
    }
    
    /// Execute a single instruction
    pub fn execute(&mut self, inst: Instruction) -> Result<(), String> {
        // Save state for reversibility
        self.save_history_frame(inst.clone());
        
        match inst {
            // Arithmetic
            Instruction::IAdd { dst, src1, src2 } => {
                let val1 = self.registers.read(src1)?;
                let val2 = self.registers.read(src2)?;
                self.registers.write(dst, val1.wrapping_add(val2))?;
                self.registers.update_flags(self.registers.read(dst)?);
            }
            
            Instruction::ISub { dst, src1, src2 } => {
                let val1 = self.registers.read(src1)?;
                let val2 = self.registers.read(src2)?;
                self.registers.write(dst, val1.wrapping_sub(val2))?;
                self.registers.update_flags(self.registers.read(dst)?);
            }
            
            Instruction::IMul { dst, src1, src2 } => {
                let val1 = self.registers.read(src1)?;
                let val2 = self.registers.read(src2)?;
                self.registers.write(dst, val1.wrapping_mul(val2))?;
                self.registers.update_flags(self.registers.read(dst)?);
            }
            
            Instruction::IXor { dst, src1, src2 } => {
                let val1 = self.registers.read(src1)?;
                let val2 = self.registers.read(src2)?;
                self.registers.write(dst, val1 ^ val2)?;
                self.registers.update_flags(self.registers.read(dst)?);
            }
            
            // Memory
            Instruction::Load { reg, addr } => {
                let address = self.registers.read(addr)?;
                self.tape.tape.seek(address);
                let value = i64::from_le_bytes(
                    self.tape.tape.read(8).try_into()
                        .map_err(|_| "Failed to read 8 bytes")?
                );
                self.registers.write(reg, value)?;
            }
            
            Instruction::Store { addr, reg } => {
                let address = self.registers.read(addr)?;
                let value = self.registers.read(reg)?;
                self.tape.tape.seek(address);
                self.tape.tape.write(&value.to_le_bytes());
            }
            
            Instruction::Push { reg } => {
                self.sp -= 8;
                self.tape.tape.seek(self.sp);
                let value = self.registers.read(reg)?;
                self.tape.tape.write(&value.to_le_bytes());
            }
            
            Instruction::Pop { reg } => {
                self.tape.tape.seek(self.sp);
                let value = i64::from_le_bytes(
                    self.tape.tape.read(8).try_into()
                        .map_err(|_| "Failed to read 8 bytes")?
                );
                self.registers.write(reg, value)?;
                self.sp += 8;
            }
            
            // Tape operations
            Instruction::TapeRead { reg, len } => {
                let data = self.tape.tape.read(len as usize);
                // Store first 8 bytes in register (or less)
                let mut bytes = [0u8; 8];
                let copy_len = len.min(8) as usize;
                bytes[..copy_len].copy_from_slice(&data[..copy_len]);
                self.registers.write(reg, i64::from_le_bytes(bytes))?;
            }
            
            Instruction::TapeWrite { reg, len } => {
                let value = self.registers.read(reg)?;
                let bytes = value.to_le_bytes();
                self.tape.tape.write(&bytes[..len.min(8) as usize]);
            }
            
            Instruction::TapeSeek { position } => {
                self.tape.tape.seek(position);
            }
            
            Instruction::TapeSeekReg { reg } => {
                let position = self.registers.read(reg)?;
                self.tape.tape.seek(position);
            }
            
            Instruction::TapeAdvance { delta } => {
                self.tape.tape.advance(delta);
            }
            
            Instruction::TapeMark { label } => {
                self.tape.tape.mark(label);
            }
            
            Instruction::TapeSeekMark { label } => {
                self.tape.tape.seek_mark(&label)?;
            }
            
            // Control flow
            Instruction::Jump { label } => {
                self.ip = self.resolve_label(&label)?;
                return Ok(()); // Don't increment IP
            }
            
            Instruction::BranchZero { reg, label } => {
                if self.registers.read(reg)? == 0 {
                    self.ip = self.resolve_label(&label)?;
                    return Ok(()); // Don't increment IP
                }
            }
            
            Instruction::BranchNotZero { reg, label } => {
                if self.registers.read(reg)? != 0 {
                    self.ip = self.resolve_label(&label)?;
                    return Ok(()); // Don't increment IP
                }
            }
            
            Instruction::Call { label } => {
                // Push return address
                self.sp -= 8;
                self.tape.tape.seek(self.sp);
                self.tape.tape.write(&(self.ip + 1).to_le_bytes());
                
                // Push frame pointer
                self.sp -= 8;
                self.tape.tape.seek(self.sp);
                self.tape.tape.write(&self.fp.to_le_bytes());
                
                // Set new frame pointer
                self.fp = self.sp;
                
                // Jump to function
                self.ip = self.resolve_label(&label)?;
                return Ok(()); // Don't increment IP
            }
            
            Instruction::Return => {
                // Restore frame pointer
                self.tape.tape.seek(self.fp);
                self.fp = i64::from_le_bytes(
                    self.tape.tape.read(8).try_into()
                        .map_err(|_| "Failed to read frame pointer")?
                );
                self.sp += 8;
                
                // Pop return address
                self.tape.tape.seek(self.sp);
                self.ip = i64::from_le_bytes(
                    self.tape.tape.read(8).try_into()
                        .map_err(|_| "Failed to read return address")?
                );
                self.sp += 8;
                return Ok(()); // IP already set
            }
            
            // Time operations
            Instruction::Checkpoint { label } => {
                self.tape.tape.checkpoint(label.clone());
                self.history.checkpoints.insert(label, self.history.stack.len());
            }
            
            Instruction::Rewind { label } => {
                self.tape.tape.rewind(&label)?;
                
                // Restore VM state
                if let Some(&checkpoint_pos) = self.history.checkpoints.get(&label) {
                    while self.history.stack.len() > checkpoint_pos {
                        self.history.stack.pop();
                    }
                    
                    if let Some(frame) = self.history.stack.last() {
                        self.registers = frame.registers_before.clone();
                        self.ip = frame.ip_before;
                        self.sp = frame.sp_before;
                        self.fp = frame.fp_before;
                    }
                }
                return Ok(()); // IP already restored
            }
            
            Instruction::RewindN { steps } => {
                let n = self.registers.read(steps)? as usize;
                for _ in 0..n {
                    self.reverse_last()?;
                }
                return Ok(()); // IP handled by reverse_last
            }
            
            // Constants
            Instruction::LoadImm { reg, value } => {
                self.registers.write(reg, value)?;
            }
            
            // Comparison
            Instruction::Compare { dst, src1, src2 } => {
                let val1 = self.registers.read(src1)?;
                let val2 = self.registers.read(src2)?;
                let result = if val1 < val2 { -1 } else if val1 > val2 { 1 } else { 0 };
                self.registers.write(dst, result)?;
                self.registers.update_flags(result);
            }
            
            Instruction::Equal { dst, src1, src2 } => {
                let val1 = self.registers.read(src1)?;
                let val2 = self.registers.read(src2)?;
                let result = if val1 == val2 { 1 } else { 0 };
                self.registers.write(dst, result)?;
                self.registers.update_flags(result);
            }
            
            Instruction::LessThan { dst, src1, src2 } => {
                let val1 = self.registers.read(src1)?;
                let val2 = self.registers.read(src2)?;
                let result = if val1 < val2 { 1 } else { 0 };
                self.registers.write(dst, result)?;
                self.registers.update_flags(result);
            }
            
            // System
            Instruction::Halt => {
                return Err("HALT".to_string());
            }
            
            Instruction::Nop => {
                // Do nothing
            }
            
            Instruction::Debug { message } => {
                println!("DEBUG: {}", message);
                println!("  IP: {}, SP: {}, FP: {}", self.ip, self.sp, self.fp);
                println!("  Registers: {:?}", &self.registers.general[0..8]);
            }
            
            _ => return Err(format!("Unimplemented instruction: {:?}", inst)),
        }
        
        self.ip += 1;
        Ok(())
    }
    
    fn save_history_frame(&mut self, instruction: Instruction) {
        let frame = HistoryFrame {
            instruction,
            registers_before: self.registers.clone(),
            ip_before: self.ip,
            sp_before: self.sp,
            fp_before: self.fp,
            tape_trail_len: self.tape.tape.trail_len(),
        };
        self.history.stack.push(frame);
    }
    
    fn resolve_label(&self, label: &str) -> Result<i64, String> {
        self.symbols.get(label)
            .copied()
            .or_else(|| self.tape.tape.get_mark(label))
            .ok_or_else(|| format!("Unknown label: {}", label))
    }
    
    /// Reverse the last executed instruction
    pub fn reverse_last(&mut self) -> Result<(), String> {
        if let Some(frame) = self.history.stack.pop() {
            // Restore registers
            self.registers = frame.registers_before;
            self.ip = frame.ip_before;
            self.sp = frame.sp_before;
            self.fp = frame.fp_before;
            
            // Rewind tape operations
            let rewind_count = self.tape.tape.trail_len() - frame.tape_trail_len;
            self.tape.tape.rewind_n(rewind_count);
            
            Ok(())
        } else {
            Err("No operations to reverse".to_string())
        }
    }
    
    /// Load a program into the code segment
    pub fn load_program(&mut self, instructions: Vec<Instruction>) -> Result<(), String> {
        // For now, just store instruction count
        // In a real implementation, we'd encode instructions to bytes
        self.symbols.insert("__program_size__".to_string(), instructions.len() as i64);
        Ok(())
    }
}

impl ExecutionHistory {
    pub fn new() -> Self {
        ExecutionHistory {
            stack: Vec::new(),
            checkpoints: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_creation() {
        let vm = VM::new();
        assert_eq!(vm.ip, 0);
        assert_eq!(vm.sp, 1024 * 1024);
        assert_eq!(vm.current_timeline, "main");
    }

    #[test]
    fn test_arithmetic_operations() {
        let mut vm = VM::new();
        
        // Load some values
        vm.execute(Instruction::LoadImm { reg: 0, value: 10 }).unwrap();
        vm.execute(Instruction::LoadImm { reg: 1, value: 20 }).unwrap();
        
        // Add them
        vm.execute(Instruction::IAdd { dst: 2, src1: 0, src2: 1 }).unwrap();
        
        assert_eq!(vm.registers.read(2).unwrap(), 30);
    }

    #[test]
    fn test_stack_operations() {
        let mut vm = VM::new();
        
        // Push some values
        vm.execute(Instruction::LoadImm { reg: 0, value: 42 }).unwrap();
        vm.execute(Instruction::Push { reg: 0 }).unwrap();
        
        vm.execute(Instruction::LoadImm { reg: 1, value: 100 }).unwrap();
        vm.execute(Instruction::Push { reg: 1 }).unwrap();
        
        // Pop them back
        vm.execute(Instruction::Pop { reg: 2 }).unwrap();
        assert_eq!(vm.registers.read(2).unwrap(), 100);
        
        vm.execute(Instruction::Pop { reg: 3 }).unwrap();
        assert_eq!(vm.registers.read(3).unwrap(), 42);
    }

    #[test]
    fn test_reversibility() {
        let mut vm = VM::new();
        
        // Execute some operations
        vm.execute(Instruction::LoadImm { reg: 0, value: 10 }).unwrap();
        vm.execute(Instruction::LoadImm { reg: 1, value: 20 }).unwrap();
        vm.execute(Instruction::IAdd { dst: 2, src1: 0, src2: 1 }).unwrap();
        
        assert_eq!(vm.registers.read(2).unwrap(), 30);
        
        // Reverse the addition
        vm.reverse_last().unwrap();
        assert_eq!(vm.registers.read(2).unwrap(), 0);
        
        // Reverse loading R1
        vm.reverse_last().unwrap();
        assert_eq!(vm.registers.read(1).unwrap(), 0);
    }
}