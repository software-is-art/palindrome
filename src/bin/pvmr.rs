//! Palindrome VM Runner - Execute PVM assembly programs

use palindrome_vm::{VM, Parser};
use std::fs;
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: pvmr <file.pvm>");
        std::process::exit(1);
    }
    
    // Read the assembly file
    let code = fs::read_to_string(&args[1])
        .unwrap_or_else(|e| {
            eprintln!("Failed to read file '{}': {}", args[1], e);
            std::process::exit(1);
        });
    
    // Parse the assembly
    let mut parser = Parser::new();
    let instructions = parser.parse(&code)
        .unwrap_or_else(|e| {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        });
    
    if instructions.is_empty() {
        eprintln!("No instructions found in file");
        std::process::exit(1);
    }
    
    // Create VM and load program
    let mut vm = VM::new();
    
    // Copy symbols from parser to VM
    for (label, pos) in parser.labels() {
        vm.symbols.insert(label.clone(), *pos);
    }
    
    vm.load_program(instructions.clone())
        .unwrap_or_else(|e| {
            eprintln!("Failed to load program: {}", e);
            std::process::exit(1);
        });
    
    println!("Palindrome VM Runner");
    println!("===================");
    println!("Loaded {} instructions", instructions.len());
    println!("Starting execution...\n");
    
    // Execute instructions
    let mut instruction_count = 0;
    let mut halted = false;
    
    while (vm.ip as usize) < instructions.len() && !halted {
        let inst = instructions[vm.ip as usize].clone();
        
        match vm.execute(inst.clone()) {
            Ok(()) => {
                instruction_count += 1;
            }
            Err(e) => {
                if e == "HALT" {
                    halted = true;
                    println!("\nProgram halted normally.");
                } else {
                    eprintln!("\nExecution error at IP {}: {}", vm.ip, e);
                    eprintln!("Instruction: {:?}", inst);
                    
                    // Offer to reverse or debug
                    print!("\nOptions: (r)everse last, (d)ebug, (q)uit: ");
                    io::stdout().flush().unwrap();
                    
                    let mut input = String::new();
                    io::stdin().read_line(&mut input).unwrap();
                    
                    match input.trim() {
                        "r" => {
                            match vm.reverse_last() {
                                Ok(()) => {
                                    println!("Reversed last operation. IP now at {}", vm.ip);
                                    continue;
                                }
                                Err(e) => {
                                    eprintln!("Failed to reverse: {}", e);
                                    break;
                                }
                            }
                        }
                        "d" => {
                            debug_vm(&vm);
                            continue;
                        }
                        _ => break,
                    }
                }
            }
        }
    }
    
    if !halted && (vm.ip as usize) >= instructions.len() {
        println!("\nProgram ended (reached end of instructions).");
    }
    
    println!("\nExecution statistics:");
    println!("  Instructions executed: {}", instruction_count);
    println!("  Final IP: {}", vm.ip);
    println!("  Final SP: {}", vm.sp);
    println!("  Tape position: {}", vm.tape.tape.position());
}

fn debug_vm(vm: &VM) {
    println!("\n=== VM State ===");
    println!("IP: {}, SP: {}, FP: {}", vm.ip, vm.sp, vm.fp);
    println!("\nRegisters:");
    for i in 0..8 {
        print!("  R{}: {:8} ", i, vm.registers.general[i]);
        if i % 4 == 3 { println!(); }
    }
    println!("\nFlags:");
    println!("  Zero: {}, Negative: {}, Carry: {}, Overflow: {}",
        vm.registers.flags.zero,
        vm.registers.flags.negative,
        vm.registers.flags.carry,
        vm.registers.flags.overflow
    );
    println!("\nTape position: {}", vm.tape.tape.position());
    println!("History depth: {}", vm.history.stack.len());
    println!("================\n");
}