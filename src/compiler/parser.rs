//! Assembly parser for Palindrome VM

use crate::instruction::Instruction;
use crate::vm::Register;
use std::collections::HashMap;

pub struct Parser {
    labels: HashMap<String, i64>,
    current_position: i64,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            labels: HashMap::new(),
            current_position: 0,
        }
    }
    
    /// Get labels map
    pub fn labels(&self) -> &HashMap<String, i64> {
        &self.labels
    }
    
    pub fn parse(&mut self, source: &str) -> Result<Vec<Instruction>, String> {
        let mut instructions = Vec::new();
        
        // First pass: collect labels
        self.current_position = 0;
        for line in source.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            
            if line.ends_with(':') {
                let label = line.trim_end_matches(':').to_string();
                self.labels.insert(label, self.current_position);
            } else {
                self.current_position += 1;
            }
        }
        
        // Second pass: parse instructions
        self.current_position = 0;
        for (line_num, line) in source.lines().enumerate() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(';') {
                continue;
            }
            
            // Skip labels
            if line.ends_with(':') {
                continue;
            }
            
            match self.parse_instruction(line) {
                Ok(inst) => instructions.push(inst),
                Err(e) => return Err(format!("Line {}: {}", line_num + 1, e)),
            }
        }
        
        Ok(instructions)
    }
    
    fn parse_instruction(&self, line: &str) -> Result<Instruction, String> {
        // Remove comments (everything after ';')
        let line = if let Some(pos) = line.find(';') {
            &line[..pos]
        } else {
            line
        };
        
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty instruction".to_string());
        }
        
        let mnemonic = parts[0].to_uppercase();
        
        match mnemonic.as_str() {
            "IADD" => {
                if parts.len() != 4 {
                    return Err("IADD requires 3 operands".to_string());
                }
                Ok(Instruction::IAdd {
                    dst: self.parse_register(parts[1])?,
                    src1: self.parse_register(parts[2])?,
                    src2: self.parse_register(parts[3])?,
                })
            }
            
            "ISUB" => {
                if parts.len() != 4 {
                    return Err("ISUB requires 3 operands".to_string());
                }
                Ok(Instruction::ISub {
                    dst: self.parse_register(parts[1])?,
                    src1: self.parse_register(parts[2])?,
                    src2: self.parse_register(parts[3])?,
                })
            }
            
            "IMUL" => {
                if parts.len() != 4 {
                    return Err("IMUL requires 3 operands".to_string());
                }
                Ok(Instruction::IMul {
                    dst: self.parse_register(parts[1])?,
                    src1: self.parse_register(parts[2])?,
                    src2: self.parse_register(parts[3])?,
                })
            }
            
            "IXOR" => {
                if parts.len() != 4 {
                    return Err("IXOR requires 3 operands".to_string());
                }
                Ok(Instruction::IXor {
                    dst: self.parse_register(parts[1])?,
                    src1: self.parse_register(parts[2])?,
                    src2: self.parse_register(parts[3])?,
                })
            }
            
            "LOAD" => {
                if parts.len() != 3 {
                    return Err("LOAD requires 2 operands".to_string());
                }
                Ok(Instruction::Load {
                    reg: self.parse_register(parts[1])?,
                    addr: self.parse_register(parts[2])?,
                })
            }
            
            "STORE" => {
                if parts.len() != 3 {
                    return Err("STORE requires 2 operands".to_string());
                }
                Ok(Instruction::Store {
                    addr: self.parse_register(parts[1])?,
                    reg: self.parse_register(parts[2])?,
                })
            }
            
            "PUSH" => {
                if parts.len() != 2 {
                    return Err("PUSH requires 1 operand".to_string());
                }
                Ok(Instruction::Push {
                    reg: self.parse_register(parts[1])?,
                })
            }
            
            "POP" => {
                if parts.len() != 2 {
                    return Err("POP requires 1 operand".to_string());
                }
                Ok(Instruction::Pop {
                    reg: self.parse_register(parts[1])?,
                })
            }
            
            "LOADIMM" | "LI" => {
                if parts.len() != 3 {
                    return Err("LOADIMM requires 2 operands".to_string());
                }
                Ok(Instruction::LoadImm {
                    reg: self.parse_register(parts[1])?,
                    value: self.parse_immediate(parts[2])?,
                })
            }
            
            "TAPEREAD" => {
                if parts.len() != 3 {
                    return Err("TAPEREAD requires 2 operands".to_string());
                }
                Ok(Instruction::TapeRead {
                    reg: self.parse_register(parts[1])?,
                    len: self.parse_byte(parts[2])?,
                })
            }
            
            "TAPEWRITE" => {
                if parts.len() != 3 {
                    return Err("TAPEWRITE requires 2 operands".to_string());
                }
                Ok(Instruction::TapeWrite {
                    reg: self.parse_register(parts[1])?,
                    len: self.parse_byte(parts[2])?,
                })
            }
            
            "TAPESEEK" => {
                if parts.len() != 2 {
                    return Err("TAPESEEK requires 1 operand".to_string());
                }
                Ok(Instruction::TapeSeek {
                    position: self.parse_immediate(parts[1])?,
                })
            }
            
            "TAPEADVANCE" => {
                if parts.len() != 2 {
                    return Err("TAPEADVANCE requires 1 operand".to_string());
                }
                Ok(Instruction::TapeAdvance {
                    delta: self.parse_immediate(parts[1])?,
                })
            }
            
            "TAPEMARK" => {
                if parts.len() != 2 {
                    return Err("TAPEMARK requires 1 operand".to_string());
                }
                Ok(Instruction::TapeMark {
                    label: parts[1].to_string(),
                })
            }
            
            "TAPESEEKMARK" => {
                if parts.len() != 2 {
                    return Err("TAPESEEKMARK requires 1 operand".to_string());
                }
                Ok(Instruction::TapeSeekMark {
                    label: parts[1].to_string(),
                })
            }
            
            "JMP" | "JUMP" => {
                if parts.len() != 2 {
                    return Err("JUMP requires 1 operand".to_string());
                }
                Ok(Instruction::Jump {
                    label: parts[1].to_string(),
                })
            }
            
            "BZ" | "BRANCHZERO" => {
                if parts.len() != 3 {
                    return Err("BRANCHZERO requires 2 operands".to_string());
                }
                Ok(Instruction::BranchZero {
                    reg: self.parse_register(parts[1])?,
                    label: parts[2].to_string(),
                })
            }
            
            "BNZ" | "BRANCHNOTZERO" => {
                if parts.len() != 3 {
                    return Err("BRANCHNOTZERO requires 2 operands".to_string());
                }
                Ok(Instruction::BranchNotZero {
                    reg: self.parse_register(parts[1])?,
                    label: parts[2].to_string(),
                })
            }
            
            "CALL" => {
                if parts.len() != 2 {
                    return Err("CALL requires 1 operand".to_string());
                }
                Ok(Instruction::Call {
                    label: parts[1].to_string(),
                })
            }
            
            "RET" | "RETURN" => {
                Ok(Instruction::Return)
            }
            
            "CHECKPOINT" | "CP" => {
                if parts.len() != 2 {
                    return Err("CHECKPOINT requires 1 operand".to_string());
                }
                Ok(Instruction::Checkpoint {
                    label: parts[1].to_string(),
                })
            }
            
            "REWIND" | "RW" => {
                if parts.len() != 2 {
                    return Err("REWIND requires 1 operand".to_string());
                }
                Ok(Instruction::Rewind {
                    label: parts[1].to_string(),
                })
            }
            
            "CMP" | "COMPARE" => {
                if parts.len() != 4 {
                    return Err("COMPARE requires 3 operands".to_string());
                }
                Ok(Instruction::Compare {
                    dst: self.parse_register(parts[1])?,
                    src1: self.parse_register(parts[2])?,
                    src2: self.parse_register(parts[3])?,
                })
            }
            
            "EQ" | "EQUAL" => {
                if parts.len() != 4 {
                    return Err("EQUAL requires 3 operands".to_string());
                }
                Ok(Instruction::Equal {
                    dst: self.parse_register(parts[1])?,
                    src1: self.parse_register(parts[2])?,
                    src2: self.parse_register(parts[3])?,
                })
            }
            
            "LT" | "LESSTHAN" => {
                if parts.len() != 4 {
                    return Err("LESSTHAN requires 3 operands".to_string());
                }
                Ok(Instruction::LessThan {
                    dst: self.parse_register(parts[1])?,
                    src1: self.parse_register(parts[2])?,
                    src2: self.parse_register(parts[3])?,
                })
            }
            
            "HALT" => Ok(Instruction::Halt),
            "NOP" => Ok(Instruction::Nop),
            
            "DEBUG" => {
                let message = parts[1..].join(" ");
                Ok(Instruction::Debug { message })
            }
            
            _ => Err(format!("Unknown instruction: {}", mnemonic)),
        }
    }
    
    fn parse_register(&self, s: &str) -> Result<Register, String> {
        let s = s.trim_end_matches(',');
        
        if s.starts_with('R') || s.starts_with('r') {
            let num_str = &s[1..];
            let num = num_str.parse::<u8>()
                .map_err(|_| format!("Invalid register: {}", s))?;
            
            if num < 16 {
                Ok(num)
            } else {
                Err(format!("Register out of range: {}", s))
            }
        } else {
            Err(format!("Invalid register format: {}", s))
        }
    }
    
    fn parse_immediate(&self, s: &str) -> Result<i64, String> {
        let s = s.trim_start_matches('#');
        
        if s.starts_with("0x") || s.starts_with("0X") {
            i64::from_str_radix(&s[2..], 16)
                .map_err(|_| format!("Invalid hex immediate: {}", s))
        } else {
            s.parse::<i64>()
                .map_err(|_| format!("Invalid immediate: {}", s))
        }
    }
    
    fn parse_byte(&self, s: &str) -> Result<u8, String> {
        s.parse::<u8>()
            .map_err(|_| format!("Invalid byte value: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_program() {
        let mut parser = Parser::new();
        let program = r#"
            ; Simple test program
            LI R0, 10
            LI R1, 20
            IADD R2, R0, R1
            HALT
        "#;
        
        let instructions = parser.parse(program).unwrap();
        assert_eq!(instructions.len(), 4);
        
        match &instructions[0] {
            Instruction::LoadImm { reg, value } => {
                assert_eq!(*reg, 0);
                assert_eq!(*value, 10);
            }
            _ => panic!("Wrong instruction"),
        }
    }

    #[test]
    fn test_parse_with_labels() {
        let mut parser = Parser::new();
        let program = r#"
        main:
            LI R0, 0
        loop:
            LI R1, 1
            IADD R0, R0, R1
            BNZ R0, loop
            HALT
        "#;
        
        let instructions = parser.parse(program).unwrap();
        assert_eq!(instructions.len(), 5);
        
        // Check that labels were collected
        assert_eq!(parser.labels.get("main"), Some(&0));
        assert_eq!(parser.labels.get("loop"), Some(&1));
    }

    #[test]
    fn test_parse_hex_immediates() {
        let mut parser = Parser::new();
        let program = "LI R0, 0xFF";
        
        let instructions = parser.parse(program).unwrap();
        match &instructions[0] {
            Instruction::LoadImm { reg, value } => {
                assert_eq!(*reg, 0);
                assert_eq!(*value, 255);
            }
            _ => panic!("Wrong instruction"),
        }
    }
}