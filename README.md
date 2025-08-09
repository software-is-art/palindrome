# Palindrome VM

A reversible virtual machine with unified tape abstraction that eliminates traditional software complexity through time-travel debugging and inherent reversibility.

## Overview

Palindrome VM is an experimental virtual machine where:
- Everything is stored on an infinite bidirectional tape
- Every operation is reversible by design
- Memory, storage, and history are unified
- No need for traditional version control, migrations, or defensive programming

## Quick Start

### Build the VM

```bash
cargo build --release
```

### Run a Program

```bash
./target/release/pvmr examples/simple_test.pvm
```

### Example: Fibonacci Sequence

```bash
./target/release/pvmr examples/fibonacci.pvm
```

This calculates the first 10 Fibonacci numbers, writes them to tape, then reads them back.

## Assembly Language

### Registers
- 16 general-purpose registers: R0 through R15
- Flags: Zero, Negative, Carry, Overflow

### Basic Instructions

```asm
; Arithmetic (all reversible)
IADD R2, R0, R1    ; R2 = R0 + R1
ISUB R2, R0, R1    ; R2 = R0 - R1
IMUL R2, R0, R1    ; R2 = R0 * R1
IXOR R2, R0, R1    ; R2 = R0 ^ R1 (self-inverse)

; Load immediate
LI R0, 42          ; R0 = 42

; Memory operations
LOAD R0, R1        ; R0 = memory[R1]
STORE R1, R0       ; memory[R1] = R0
PUSH R0            ; Push R0 to stack
POP R0             ; Pop from stack to R0

; Tape operations
TAPEREAD R0, 8     ; Read 8 bytes from tape to R0
TAPEWRITE R0, 8    ; Write 8 bytes from R0 to tape
TAPEADVANCE 8      ; Move tape head forward 8 positions
TAPEMARK label     ; Mark current tape position
TAPESEEKMARK label ; Seek to marked position

; Control flow
JMP label          ; Unconditional jump
BZ R0, label       ; Branch if R0 is zero
BNZ R0, label      ; Branch if R0 is not zero
CALL function      ; Call function
RET                ; Return from function

; Time operations (unique to Palindrome!)
CHECKPOINT save    ; Create a checkpoint
REWIND save        ; Rewind VM state to checkpoint

; System
HALT               ; Stop execution
NOP                ; No operation
DEBUG message      ; Print debug info
```

### Example Program

```asm
main:
    ; Create checkpoint for reversal
    CHECKPOINT start
    
    ; Calculate 5 + 3
    LI R0, 5
    LI R1, 3
    IADD R2, R0, R1
    
    ; Write result to tape
    TAPEWRITE R2, 8
    
    ; Can rewind to undo everything
    ; REWIND start
    
    HALT
```

## Architecture

### Tape System
- Infinite bidirectional tape divided into 4KB pages
- Copy-on-write for efficiency
- All operations recorded in history trail

### Software Defined Memory (SDM)
- **Policy-driven memory management** - no manual memory allocation
- **Hierarchical storage** - transparently spans DRAM, SSD, network, and cloud storage
- **Temporal awareness** - optimized for time-travel access patterns
- **Zero-copy versioning** - historical data preserved efficiently
- **Predictive prefetching** - learns from access patterns to optimize performance

### Segments
- Code: Program instructions
- Stack: Function calls and local variables
- Heap: Dynamic memory
- Data: User-defined segments

### Reversibility
- Every instruction has an inverse
- Full execution history maintained
- Checkpoint/rewind for time-travel debugging
- SDM automatically optimizes storage for rewind operations

## Interactive Debugging

When an error occurs, the VM offers interactive options:
- `r`: Reverse last operation
- `d`: Show debug information
- `q`: Quit

## Future Features

- [ ] Binary instruction encoding
- [ ] JIT compilation
- [ ] Distributed tape protocol
- [ ] Compression for history
- [ ] High-level Palindrome language

## Design Philosophy

Traditional software complexity comes from irreversible operations:
- Once data is deleted, it's gone
- Once a migration runs, it can't be undone
- Once a bug corrupts state, recovery is difficult

Palindrome VM eliminates these problems by making everything reversible at the VM level. The 2x memory overhead is insignificant compared to eliminating entire categories of bugs and complexity.

## License

MIT