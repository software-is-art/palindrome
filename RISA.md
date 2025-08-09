# Reversible Instruction Set Architecture (RISA)

## Overview

This document describes Palindrome VM's theoretically reversible instruction set - a set of operations that are mathematically reversible without requiring a trail or history. When combined with Software Defined Memory (SDM), these instructions enable zero-overhead time travel and provably reversible computation.

## Theoretical Foundation

### Bennett's Reversibility Principle

Every reversible operation must preserve information. For a function to be reversible, it must be bijective (one-to-one and onto). This means:

1. **No information loss**: The output must contain enough information to reconstruct the input
2. **No information creation**: Can't generate new entropy
3. **Deterministic**: Same input always produces same output

### Conservative Logic

Based on Fredkin and Toffoli's work, our operations preserve the number of 1s and 0s (or more generally, preserve some quantity). This ensures physical reversibility.

## Reversible Instructions

### Arithmetic Operations

#### RADD - Reversible Addition
```assembly
RADD Ra, Rb, Rc
// Before: Ra = a, Rb = b, Rc = c
// After:  Ra = a, Rb = b, Rc = (c + a + b) mod 2^64
// Inverse: RSUB Ra, Rb, Rc
```

#### RSUB - Reversible Subtraction
```assembly
RSUB Ra, Rb, Rc  
// Before: Ra = a, Rb = b, Rc = c
// After:  Ra = a, Rb = b, Rc = (c - a - b) mod 2^64
// Inverse: RADD Ra, Rb, Rc
```

#### RXOR - Reversible XOR
```assembly
RXOR Ra, Rb
// Before: Ra = a, Rb = b
// After:  Ra = a ⊕ b, Rb = b
// Inverse: RXOR Ra, Rb (self-inverse)
```

#### SWAP - Register Swap
```assembly
SWAP Ra, Rb
// Before: Ra = a, Rb = b  
// After:  Ra = b, Rb = a
// Inverse: SWAP Ra, Rb (self-inverse)
```

### Conditional Operations

#### CSWAP - Conditional Swap (Fredkin Gate)
```assembly
CSWAP Rc, Ra, Rb
// If Rc ≠ 0: swap Ra and Rb
// If Rc = 0: no change
// Inverse: CSWAP Rc, Ra, Rb (self-inverse)
```

#### CNOT - Controlled NOT
```assembly
CNOT Rc, Ra
// If Rc ≠ 0: Ra = ¬Ra
// If Rc = 0: no change  
// Inverse: CNOT Rc, Ra (self-inverse)
```

### Memory Operations

#### RLOAD - Reversible Load
```assembly
RLOAD Rdst, Raddr, Rold
// Before: Rdst = d, Raddr = addr, Rold = o, Mem[addr] = m
// After:  Rdst = m, Raddr = addr, Rold = d, Mem[addr] = m
// Inverse: RLOAD Rold, Raddr, Rdst
```

#### RSTORE - Reversible Store
```assembly
RSTORE Raddr, Rsrc, Rold
// Before: Raddr = addr, Rsrc = s, Rold = o, Mem[addr] = m
// After:  Raddr = addr, Rsrc = s, Rold = m, Mem[addr] = s
// Inverse: RSTORE Raddr, Rold, Rsrc
```

#### MSWAP - Memory-Register Swap
```assembly
MSWAP Raddr, Rval
// Before: Raddr = addr, Rval = v, Mem[addr] = m
// After:  Raddr = addr, Rval = m, Mem[addr] = v
// Inverse: MSWAP Raddr, Rval (self-inverse)
```

### Stack Operations

#### RPUSH - Reversible Push
```assembly
RPUSH Rsrc, Rold
// Before: Rsrc = s, Rold = o, SP = sp, Mem[sp] = m
// After:  Rsrc = s, Rold = m, SP = sp-8, Mem[sp-8] = s
// Inverse: RPOP Rsrc, Rold
```

#### RPOP - Reversible Pop  
```assembly
RPOP Rdst, Rold
// Before: Rdst = d, Rold = o, SP = sp, Mem[sp] = m
// After:  Rdst = m, Rold = d, SP = sp+8, Mem[sp+8] = o
// Inverse: RPUSH Rdst, Rold
```

### Control Flow

#### RJMP - Reversible Jump
```assembly
RJMP Label, Rret
// Before: IP = ip, Rret = r
// After:  IP = Label, Rret = ip + 1
// Inverse: RJMP Rret, Label (swap operands)
```

#### RCALL - Reversible Call
```assembly
RCALL Func, Rret, Rframe
// Saves return address and frame pointer reversibly
// Complex but preserves all state information
```

### Tape Operations

#### TSWAP - Tape-Register Swap
```assembly
TSWAP Rval, Length
// Swaps register with tape at current position
// Naturally reversible with SDM versioning
```

#### TMOVE - Reversible Tape Movement
```assembly
TMOVE Rdelta, Rold
// Before: TapePos = t, Rdelta = d, Rold = o
// After:  TapePos = t + d, Rdelta = d, Rold = t
```

## Integration with SDM

### Zero-Copy Reversibility

When combined with SDM's versioning:

```assembly
RLOAD R0, R1, R2    // Load with old value preserved
// SDM automatically versions the page
// No trail needed - the instruction is its own inverse!
```

### Temporal Operations

```assembly
// Checkpoint without trail
CHECKPOINT "save"   // SDM marks current versions

// Rewind by reverse execution
REVERSE_MODE        // CPU executes instructions backwards
// ... instructions execute in reverse ...
FORWARD_MODE        // Resume forward execution
```

## Programming Patterns

### Pattern 1: Reversible Arithmetic
```assembly
// Compute (a + b) * c reversibly
RADD R0, R1, R2    // R2 = a + b
RMUL R2, R3, R4    // R4 = (a + b) * c
// Original values in R0, R1, R3 preserved!
```

### Pattern 2: Reversible Memory Update
```assembly
// Increment memory location
RLOAD R0, R1, R2   // R0 = Mem[R1], R2 = old R0
RADD R0, One, R0   // R0 = R0 + 1  
RSTORE R1, R0, R3  // Mem[R1] = R0, R3 = old Mem[R1]
```

### Pattern 3: Conditional Execution
```assembly
// Reversible if-then-else
CMP R0, R1, R2     // R2 = (R0 == R1)
CSWAP R2, R3, R4   // Swap R3/R4 if equal
// ... operations on R3 ...
CSWAP R2, R3, R4   // Restore if needed
```

## Advantages

1. **Provable Reversibility**: Can mathematically prove any program is reversible
2. **Zero Trail Overhead**: No history recording needed
3. **Natural Parallelism**: No trail synchronization required
4. **Energy Efficiency**: Theoretical minimum energy dissipation
5. **Formal Verification**: Easier to prove program properties

## Disadvantages

1. **Register Pressure**: Need registers for temporary/old values
2. **Programming Complexity**: Requires different thinking
3. **Code Size**: More instructions for same operation
4. **Limited Optimization**: Can't eliminate "dead" values

## Hybrid Mode

Palindrome VM supports both modes:

```assembly
.mode trail         // Traditional mode with trail
IADD R2, R0, R1    // Simple but uses trail

.mode reversible   // Provably reversible mode  
RADD R0, R1, R2    // No trail needed

.mode auto         // Compiler chooses based on context
```

## Future Extensions

### Quantum-Inspired Operations
```assembly
PHASE Ra, Angle    // Quantum phase shift
HADAMARD Ra        // Superposition operation
```

### Reversible Cryptography
```assembly
RENCRYPT Rkey, Rplain, Rcipher
RDECRYPT Rkey, Rcipher, Rplain  
```

### Reversible Neural Operations
```assembly
RNEURON Weights, Input, Output, Gradient
```

## Conclusion

The Reversible ISA, combined with SDM, creates a unique computing environment where:
- Every operation can be undone
- Time travel has zero overhead
- Programs can be formally verified
- Distribution and parallelism are natural

This positions Palindrome VM as not just a debugging tool, but a platform for exploring new computational paradigms where reversibility is fundamental, not an afterthought.