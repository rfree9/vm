use std::fs;
use std::io::stdin;
use std::str::FromStr;

pub struct VirtualMachine {
    stack: Vec<u8>,
    stack_pointer: i32,
    program_counter: i32,
    exit_code: i32,
    should_exit: bool
}

impl VirtualMachine {
    /* Constructor. */
    pub fn build(args: &[String]) -> Result<VirtualMachine, String> {
        if args.len() != 2 {
            return Err(String::from("usage: vm <file.v>"));
        }

        /* Verifying the file is valid. */

        let file_result = fs::read(&args[1]);
        let mut file_buf = match file_result {
            Ok(file_buf) => file_buf,
            Err(_) => return Err(String::from("Couldn't open file.")),
        };

        if file_buf.len() > (4096 + 4) {
            return Err(String::from("File too big."));
        }

        if file_buf.len() < 4 || file_buf[0..4] != vec![0xde, 0xad, 0xbe, 0xef] {
            return Err(String::from("File format is invalid."));
        }

        /* Creating the stack. */

        let mut stack = file_buf.split_off(4);
        stack.resize(4096, 0);

        /* Creating the struct. */

        Ok(VirtualMachine {
            stack,
            stack_pointer: 4096,
            program_counter: 0,
            exit_code: 0,
            should_exit: false
        })
    }

    /* Parse and execute instructions from the stack. */
    pub fn run(&mut self) -> Result<i32, String> {
        loop {
            let instruction = self.get_next_instruction();
            self.execute_instruction(instruction)?;
            
            self.increment_program_counter();
            if self.should_exit {
                
                break;
            }
        }

        Ok(self.exit_code)
    }

    /* Grab the next 4 bytes from the stack and pack it into one int. */
    fn get_next_instruction(&self) -> u32 {
        let pc = self.program_counter as usize;
        let bound = pc + 4 as usize;

        if pc >= self.stack.len() || bound >= self.stack.len() {
            panic!("VirtualMachine::get_next_instruction() failed: pc or bound out of range");
        }

        let instruction_buf = &self.stack[pc..bound];
        let mut instruction: u32 = 0;

        instruction |= instruction_buf[0] as u32;
        instruction |= (instruction_buf[1] as u32) << 8;
        instruction |= (instruction_buf[2] as u32) << 16;
        instruction |= (instruction_buf[3] as u32) << 24;

        instruction
    }

    /* Increment the program counter by one instruction. */
    fn increment_program_counter(&mut self) {
        self.program_counter += 4;
    }

    /* Pull out an instruction's opcode. */
    fn get_op_code(instruction: u32) -> u32 {
        instruction >> 28
    }

    /* Print out the current state of the stack. */
    fn print_stack(&self) {
        let mut i = 0;

        //print!(" {:04x} ", i);
        for byte in &self.stack {
            if i % 16 == 0 {
                if i != 0 { 
                    print!("\n");
                }
                print!(" {:04x} | ", i);
            }
            
            print!("  {:02x}", byte);

            i += 1;
        }

        print!("\n");
    }

    /* Print the SP and PC. */
    fn print_vm_info(&self) {
        println!(" - stack pointer:   {}", self.stack_pointer);
        println!(" - program counter: {}", self.program_counter);
    }

    /* Executes an instruction. */
    fn execute_instruction(&mut self, instruction: u32) -> Result<(), String> {
        let opcode = VirtualMachine::get_op_code(instruction);
        
        print!("opcode: {:x} -- ", VirtualMachine::get_op_code(instruction));
        match opcode {
            0 => {
                println!("Misc. instruction");
                let misc_instruction = instruction >> 24;

                match misc_instruction {
                    0 => {
                        println!("Exit Instruction");
                        self.exit(instruction)?
                    },
                    0x1 => println!("Swap Instruction"),
                    0x2 => println!("Nop Instruction"),
                    0x4 => {
                        println!("Input Instruction");
                        self.input(instruction)?
                    },
                    0x5 => {
                        println!("stinput Instruction");
                        self.stinput(instruction)?;
                    },
                    0xF => {
                        println!("Debug Instruction");
                        self.print_stack();
                        self.print_vm_info();
                    }
                    _ => return Err(String::from("Bad instruction.")),
                }
            },
            1 => {
                println!("Pop instruction");
                self.pop(instruction)?;
            },
            2 => {
                println!("Binary arithmetic instruction");
                self.binary_arithmetic(instruction)?;
            },
            3 => { 
                println!("Unary arithmetic instruction");
                self.unary_arithmetic(instruction)?;
            },
            4 => println!("String print instruction"),
            5 => {
                println!("Call instruction");
                self.call(instruction)?;
            },
            6 => println!("Return instruction"),
            7 => {
                println!("Unconditional goto instruction");
                self.goto(instruction)?;
            },
            8 => println!("Binary if instruction"),
            9 => println!("Unary if instruction"),
            12 => println!("Dup instruction"),
            13 => println!("Print instruction"),
            14 => println!("Dump instruction"),
            15 => {
                println!("Push instruction");
                self.push(instruction)?;
            },
            _ => return Err(String::from("Bad instruction.")),
        }

        Ok(())
    }

    /* Fetch four bytes from the stack. */ 
    fn pop_int_from_stack(&mut self) -> Result<u32, String> {
        let new_stack_pointer = self.stack_pointer + 4;

        if new_stack_pointer > 4096 {
            return Err(String::from("Failed to pop: stack is empty."));
        }

        if self.stack_pointer < 0 {
            panic!("VirtualMachine::pop_int_from_stack() failed: stack_pointer out of range");
        }

        let start = self.stack_pointer as usize;
        let end = new_stack_pointer as usize;
        let mut popped = 0u32;

        for i in start..end {
            let offset = i - start; 
            let byte = (self.stack[i] as u32) << ((3 - offset) * 8);
            popped |= byte;

            //println!("{:x} {:x} {:x}", self.stack[i], offset, popped);
        }

        self.stack_pointer = new_stack_pointer;

        Ok(popped)
    }

    /* Push a word onto the stack. */
    fn push_int_onto_stack(&mut self, n: i32) -> Result<(), String> {
        let new_stack_pointer = self.stack_pointer - 4;

        if new_stack_pointer < 0 { /* TODO: this should be the end of the instruction space. */
            return Err(String::from("Out of memory."));
        }

        println!("DEBUG: {}", n);

        let bytes = n.to_be_bytes();
        let start = new_stack_pointer as usize;
        let end = start + 4;

        if end > self.stack.len() {
            panic!("VirtualMachine::push_int_onto_stack() failed: end out of range");
        }

        /* Put 'em on there. */
        
        for i in start..end {
            self.stack[i] = bytes[i - start];
        }

        self.stack_pointer = new_stack_pointer;

        Ok(())
    }

    /* Read an int from the stack. */
    fn peak_int_from_stack(&self, stack_offset: i32) -> Result<u32, String> {
        let start = (self.stack_pointer + stack_offset) as usize;
        let end = start + 4; 

        if end > 4096 {
            return Err(String::from("Failed to peak: stack is empty"));
        }
        if start > 4096 {
            return Err(String::from("Failed to peak: offset out of range"));
        }

        let mut peaked = 0u32;

        for i in start..end {
            let offset = i - start; 
            let byte = (self.stack[i] as u32) << ((3 - offset) * 8);
            peaked |= byte;
        }

        println!("DEBUG: peak says {}", peaked);

        Ok(peaked)
    }

    /* INSTRUCTIONS */
    /* TODO: These'll get their own file at some point. */

    fn exit(&mut self, instruction: u32) -> Result<(), String>{
        let code = instruction as i32;
        self.exit_code = code;
        self.should_exit = true;
        println!("DEBUG: exit code: {code}");
        
        Ok(())
    }

    fn input(&mut self, _instruction: u32) -> Result<(), String>{
        let mut ipt = String::new();
        let _ = stdin().read_line(&mut ipt);
        let trimmed = ipt.trim();
        let n: i32;
        
        if trimmed.contains("0x") || trimmed.contains("0X") {
            n = i32::from_str_radix(&trimmed[2..], 16).expect("Bad Hex Input");
        }
        else if trimmed.contains("0b") || trimmed.contains("0B") {
            n = i32::from_str_radix(&trimmed[2..], 2).expect("Bad Binary Input");
        }
        else {
            n = i32::from_str(&trimmed).expect("Bad input.");
        }

        self.push_int_onto_stack(n)?;

        Ok(())
    }

    fn stinput(&mut self, instruction: u32) -> Result<(), String>{
        let shifted = instruction & 0x00FF_FFFF;
        println!("{}", shifted);

        let mut input = String::new();
        let _ = stdin().read_line(&mut input);
        let mut trimmed = input.trim();
        
        if trimmed.len() > shifted as usize {
            trimmed = &trimmed[..shifted as usize];
        }

        // TODO: pass trimmed string to stpush

        Ok(())
    }
   
    fn push(&mut self, instruction: u32) -> Result<(), String> {
        let mut push_value = (instruction & 0x0fffffff) as i32;
        if push_value & (1 << 27) != 0 {
            /* Sign extend. */
            push_value |= 0xf << 28;
        }

        self.push_int_onto_stack(push_value)?;
        
        Ok(())
    }

    fn pop(&mut self, instruction: u32) -> Result<(), String> {
        let offset = instruction & 0x0fffffff;
        let new_stack_pointer = self.stack_pointer + offset as i32;

        println!("DEBUG: sp: {} o:{} nsp:{}", self.stack_pointer, offset, new_stack_pointer);

        if offset % 4 != 0 {
            /* This shouldn't happen, but just in case. */
            return Err(String::from("pop: Offset should be a multiple of four."));
        }

        /* If the stack pointer is already at the bottom of the memory allocated, this instruction
         * has no effect. If the offset is not given, it is by default 4. If the offset places the
         * stack pointer past the end of the memory space, the stack pointer will be reset to the
         * end of the memory space (e.g., length(memory)). */

        /* Stack pointer is at the bottom of the stack. */
        if self.stack_pointer == 4096 {
            return Ok(());
        } 

        /* New SP goes beyond the stack. */
        if new_stack_pointer > 4096 {
            self.stack_pointer = 4096;
            return Ok(());
        }

        self.stack_pointer = new_stack_pointer;
        Ok(())
    }

    fn binary_arithmetic(&mut self, instruction: u32) -> Result<i32, String> {
        let which_seperated = instruction & (0xf << 24);
        let which_operation = which_seperated >> 24;
        let right = self.pop_int_from_stack()? as i32;
        let left = self.pop_int_from_stack()? as i32;
        let result: i32;

        /* Divide by zero check. */
        if (which_operation == 3 || which_operation == 4) && right == 0 {
            return Err(String::from("Attempt to divide by zero."));
        }

        /* TODO: I'm conflicted with how to handle shifting by negative numbers. By default, Rust
         * doesn't allow it, and it's considered undefined behavior in C (from what I can tell, it
         * seems that there's no standard among processors either). The common solution in
         * languages these days is to mod the right operand by the word size. This seems to be the
         * solution Marz's program takes, but I don't like that solution because there really isn't
         * any sense in shifting by a negative number anyway; hence why Rust doesn't allow it. I
         * don't think Marz is going to test freak cases like this, so I'm going to do what I think
         * is right and just kill the program in this case. If you think I should change my mind on
         * this matter then I'll change the code to mirror Marz's, but for now I'm standing my
         * ground. ~row */
    
        /* Negative shift check. */
        if which_operation <= 8 && right < 0 {
            return Err(String::from("Attempt to shift by a negative number."));
        }

        print!("{:x} - ", which_operation);

        print!("l:{} r:{} - ", left, right);

        /* Perform calculation. */
        match which_operation {
            0 => {
                println!("add");
                result = left + right;
            },
            1 => {
                println!("sub");
                result = left - right;
            },
            2 => {
                println!("mul");
                result = left * right;
            },
            3 => {
                println!("div");
                result = left / right;
            },
            4 => {
                println!("rem");
                result = left % right;
            },
            5 => {
                println!("and");
                result = left & right;
            }, 
            6 => {
                println!("or");
                result = left | right;
            },
            7 => {
                println!("xor");
                result = left ^ right;
            },
            8 => {
                println!("lsl");
                result = left << right;
            },
            9 => {
                println!("lsr");
                let unsigned_left = left as u32;
                let unsigned_right = right as u32;
                let lsr = unsigned_left >> unsigned_right;
                result = lsr as i32; 
            },
            11 => {
                println!("asr");
                result = left >> right;
            }, 
            _ => {
                return Err(String::from("Binary arithmetic instruction contained bad identifier."));
            },
        }

        println!("Result: {}", result);
        self.push_int_onto_stack(result)?;

        Ok(result)
    }

    fn unary_arithmetic(&mut self, instruction: u32) -> Result<(), String> {
        let operand = self.pop_int_from_stack()? as i32;
        let which_seperated = instruction & (0xf << 24);
        let which_operation = which_seperated >> 24;
        let result: i32;

        print!("DEBUG -- wo:{} op:{} -- ", which_operation, operand);

        match which_operation {
            0 => {
                println!("neg");
                result = -operand;
            },
            1 => { 
                println!("not");
                result = !operand;
            },
            _ => {
                return Err(String::from("Unary arithmetic instruction contained bad identifier."));
            }
        }

        self.push_int_onto_stack(result)?;

        Ok(())
    }

    /*call instruction*/
    fn call(&mut self, instruction: u32) -> Result<(), String> {
        let og_offset = ((instruction >> 2) & 0x3FFFFFF) as i32;
        let offset = if (og_offset & (1 << 25)) != 0 {
            og_offset | !0x3FFFFFF
        } else {
            og_offset
        };
        
        //final offset in bytes
        let final_offset = offset << 2;

        //push ret addy 
        let red_addy = self.program_counter + 4;
        self.push_int_onto_stack(red_addy)?;

        //jump to new pc
        self.program_counter = self.program_counter + final_offset;

        //prev double increment 
        self.program_counter -= 4;

        println!(
            "DEBUG: call - return_address={}, new_pc={}, raw_offset={:#x}",
            red_addy,
            self.program_counter + 4,
            offset
        );

        Ok(()) 
    }
       
    fn goto(&mut self, instruction: u32) -> Result<(), String>{
        //TODO: make sure offset is signed
        let extracted = (instruction >> 2) & 0x03FF_FFFF; // 26 bits
        let mut offset: i32 = 0;
        // Check if the sign bit (bit 25 after shift) is set
        if extracted & (1 << 25) != 0 {
            // Sign-extend: set upper bits to 1
            offset = (extracted | !0x03FF_FFFF) as i32;
        } else {
            offset = extracted as i32;
        }
        println!("Goto offset: {}", offset);

        //TODO: fix offset calc
        /* 
        offset += self.program_counter;

        self.program_counter = offset;*/
    

        Ok(())
    }
}
