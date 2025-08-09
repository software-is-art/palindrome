//! Register file and flags for the VM

/// Type alias for register indices
pub type Register = u8;

/// Register file containing all CPU registers
#[derive(Clone, Debug)]
pub struct RegisterFile {
    /// 16 general purpose registers
    pub general: [i64; 16],
    /// Flags register
    pub flags: Flags,
}

/// CPU flags
#[derive(Default, Clone, Debug)]
pub struct Flags {
    pub zero: bool,
    pub carry: bool,
    pub overflow: bool,
    pub negative: bool,
}

impl RegisterFile {
    pub fn new() -> Self {
        RegisterFile {
            general: [0; 16],
            flags: Flags::default(),
        }
    }
    
    /// Read a register value
    pub fn read(&self, reg: Register) -> Result<i64, String> {
        if reg < 16 {
            Ok(self.general[reg as usize])
        } else {
            Err(format!("Invalid register: R{}", reg))
        }
    }
    
    /// Write a register value
    pub fn write(&mut self, reg: Register, value: i64) -> Result<(), String> {
        if reg < 16 {
            self.general[reg as usize] = value;
            Ok(())
        } else {
            Err(format!("Invalid register: R{}", reg))
        }
    }
    
    /// Update flags based on a value
    pub fn update_flags(&mut self, value: i64) {
        self.flags.zero = value == 0;
        self.flags.negative = value < 0;
        // Carry and overflow would be set by specific operations
    }
    
    /// Reset all registers to zero
    pub fn reset(&mut self) {
        self.general = [0; 16];
        self.flags = Flags::default();
    }
}

impl Default for RegisterFile {
    fn default() -> Self {
        Self::new()
    }
}

impl Flags {
    /// Get the condition code as a value
    pub fn condition_code(&self) -> u8 {
        let mut code = 0u8;
        if self.zero { code |= 1; }
        if self.carry { code |= 2; }
        if self.overflow { code |= 4; }
        if self.negative { code |= 8; }
        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_read_write() {
        let mut regs = RegisterFile::new();
        
        regs.write(0, 42).unwrap();
        assert_eq!(regs.read(0).unwrap(), 42);
        
        regs.write(15, -100).unwrap();
        assert_eq!(regs.read(15).unwrap(), -100);
    }

    #[test]
    fn test_invalid_register() {
        let mut regs = RegisterFile::new();
        
        assert!(regs.write(16, 0).is_err());
        assert!(regs.read(16).is_err());
    }

    #[test]
    fn test_flags() {
        let mut regs = RegisterFile::new();
        
        regs.update_flags(0);
        assert!(regs.flags.zero);
        assert!(!regs.flags.negative);
        
        regs.update_flags(-42);
        assert!(!regs.flags.zero);
        assert!(regs.flags.negative);
    }
}