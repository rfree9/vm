use std::fs;
use std::io::{stdin, stdout, Write};
use std::str::FromStr;
use std::collections::VecDeque;

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
        
        stdout().flush().expect("Failed to flush buffer");
    }

    /* Print the SP and PC. */
    fn print_vm_info(&self) {
        println!(" - stack pointer:   {}", self.stack_pointer);
        println!(" - program counter: {}", self.program_counter);
        
        stdout().flush().expect("Failed to flush buffer");
    }

    /* Executes an instruction. */
    fn execute_instruction(&mut self, instruction: u32) -> Result<(), String> {
        let opcode = VirtualMachine::get_op_code(instruction);
        
        match opcode {
            0 => {
                let misc_instruction = instruction >> 24;

                match misc_instruction {
                    0 => {
                        self.exit(instruction)?
                    },
                    0x1 => {
                        self.swap(instruction)?
                    },
                    0x2 => (),
                    0x4 => {
                        self.input()?
                    },
                    0x5 => {
                        self.stinput(instruction)?;
                    },
                    0xF => {
                        self.print_stack();
                        self.print_vm_info();

                        // ---------------------------------------------
                        // I used this for debugging swap might be usefull for something else later:
                        
                        // println!("Debug Instruction (top of stack):");
                        // Print the next four 4-byte words from SP
                        // for i in 0..4 {
                        //     let offset = (i * 4) as i32;
                        //     match self.peek_int_from_stack(offset) {
                        //         Ok(val) => println!(" SP+{}: {:#010x}", offset, val),
                        //         Err(e)  => println!(" SP+{}: <error: {}>", offset, e),
                        //     }
                        // }
                        // println!(" - stack pointer:   {}", self.stack_pointer);
                        // println!(" - program counter: {}", self.program_counter);
                    }
                    _ => return Err(String::from("Bad instruction.")),
                }
            },
            1 => {
                self.pop(instruction)?;
            },
            2 => {
                self.binary_arithmetic(instruction)?;
            },
            3 => { 
                self.unary_arithmetic(instruction)?;
            },
            4 => {
                self.stprint(instruction)?;
            },
            5 => {
                self.call(instruction)?;
            },
            6 => {
                self.ret(instruction)?;
            },
            7 => {
                self.goto(instruction)?;
            },
            8 => {
                self.binary_if(instruction)?;
            },
            9 => {
                self.unary_if(instruction)?;
            },
            12 => {
                self.dup(instruction)?;
            },
            13 => {
                self.print(instruction)?;
            },
            14 => {
                self.dump()?;
            },
            15 => {
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
    fn peek_int_from_stack(&self, stack_offset: i32) -> Result<u32, String> {
        let start = (self.stack_pointer + stack_offset) as usize;
        let end = start + 4; 

        if end > 4096 {
            return Err(String::from("Failed to peek: stack is empty"));
        }
        if start > 4096 {
            return Err(String::from("Failed to peek: offset out of range"));
        }

        let mut peeked = 0u32;

        for i in start..end {
            let offset = i - start; 
            let byte = (self.stack[i] as u32) << ((3 - offset) * 8);
            peeked |= byte;
        }

        Ok(peeked)
    }

    /* Sign extend partial numbers. 
    fn sign_extend_partial_word(word: i32, msb: i32) -> i32 {
        if msb > 31 || msb < 0 {
            panic!("sign_extend_partial_word() failed: invalid msb.");
        }

        if word & (1 << msb) == 0 {
            return word;
        }

        let sign_mask = !((1 << (msb + 1)) - 1);
        
        let signed_word = word | sign_mask;

        eprintln!("DEBUG: w:{:x} msb:{} sm:{:x} sw:{:x}", word, msb, sign_mask, signed_word);

        signed_word
    } */

    /* INSTRUCTIONS */
    /* TODO: These'll get their own file at some point. */

    fn exit(&mut self, instruction: u32) -> Result<(), String>{
        let code = instruction as i32;
        self.exit_code = code;
        self.should_exit = true;
        
        Ok(())
    }

    fn swap(&mut self, instruction: u32) -> Result<(), String> {
        // from and to are bits 23-12 and 11-0)
        let raw_from = ((instruction >> 12) & 0xFFF) as i32;
        let raw_to   = (instruction & 0xFFF) as i32;

        // Sign-extend 12-bit values to 32-bit
        let signed_from = (raw_from << 20) >> 20;
        let signed_to   = (raw_to << 20) >> 20;

        // Scale by 4 (shift left two bits)
        let offset_from = signed_from << 2;
        let offset_to   = signed_to << 2;

        // The rest of the swap logic goes here (unchanged)
        // Example:
        let addr_from = self.stack_pointer + offset_from;
        let addr_to = self.stack_pointer + offset_to;
        // Bounds check
        if addr_from < 0 || addr_from + 4 > 4096 || addr_to < 0 || addr_to + 4 > 4096 {
            return Err(String::from("swap: address out of bounds"));
        }
        for i in 0..4 {
            self.stack.swap((addr_from + i) as usize, (addr_to + i) as usize);
        }

        // println!("----------- SWAP DEBUG -----------");
        // self.print_vm_info();
        // self.print_stack();
                
        Ok(())
    }

    fn input(&mut self) -> Result<(), String>{
        let mut ipt = String::new();
        let read_response = stdin().read_line(&mut ipt);

        match read_response {
            Err(_) => return Err(String::from("Couldn't read input.")),
            _ => (),
        }

        let trimmed = ipt.trim();
        let n: i32;
        let convert_response;
        
        if trimmed.contains("0x") || trimmed.contains("0X") {
            convert_response = i32::from_str_radix(&trimmed[2..], 16);
        }
        else if trimmed.contains("0b") || trimmed.contains("0B") {
            convert_response = i32::from_str_radix(&trimmed[2..], 2);
        }
        else {
            convert_response = i32::from_str(&trimmed);
        }

        n = match convert_response {
            Ok(n) => n,
            Err(_) => return Err(String::from("Bad input.")),
        };

        self.push_int_onto_stack(n)?;

        Ok(())
    }

    fn stinput(&mut self, instruction: u32) -> Result<(), String>{
        let shifted_mask = (1 << 24) - 1;
        let shifted = instruction & shifted_mask;

        let mut input = String::new();
        let response = stdin().read_line(&mut input);

        match response {
            Err(e) => return Err(format!("Couldn't read input: {}", e)),
            _ => (),
        }

        let mut trimmed = input.trim();
        
        if trimmed.len() > shifted as usize {
            trimmed = &trimmed[..shifted as usize];
        }

        /* Convert our string into words we can push onto the stack. */
        let bytes: Vec<u8> = trimmed.bytes().collect();
        let len = bytes.len();
        let mut cur = 0i32;
        let mut d: VecDeque<i32> = VecDeque::new();
        let mut byte_index = 0i32;

        for i in 0..len {
             let byte = bytes[i];
             cur |= (byte as i32) << 8 * byte_index;

             byte_index += 1;
             if byte_index == 3 {
                 if (i + 1) < len {
                     cur |= 1 << 24;
                 }

                 d.push_front(cur);
                 byte_index = 0;
                 cur = 0;
             }
        }
        
        if byte_index != 3 {
             d.push_front(cur);
        }

        for no in d {
            self.push_int_onto_stack(no)?;
        }

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
        let mut right = self.pop_int_from_stack()? as i32;
        let left = self.pop_int_from_stack()? as i32;
        let result: i32;

        /* Divide by zero check. */
        if (which_operation == 3 || which_operation == 4) && right == 0 {
            return Err(String::from("Attempt to divide by zero."));
        }

        /* Negative shift check. */
        if which_operation >= 8 && right < 0 {
            //return Err(String::from("Attempt to shift by a negative number."));
            let true_right = right as u32 % 32;
            right = true_right as i32;
        }

        /* Perform calculation. */
        match which_operation {
            0 => {
                result = left + right;
            },
            1 => {
                result = left - right;
            },
            2 => {
                result = left * right;
            },
            3 => {
                result = left / right;
            },
            4 => {
                result = left % right;
            },
            5 => {
                result = left & right;
            }, 
            6 => {
                result = left | right;
            },
            7 => {
                result = left ^ right;
            },
            8 => {
                result = left << right;
            },
            9 => {
                let unsigned_left = left as u32;
                let unsigned_right = right as u32;
                let lsr = unsigned_left >> unsigned_right;
                result = lsr as i32; 
            },
            11 => {
                result = left >> right;
            }, 
            _ => {
                return Err(String::from("Binary arithmetic instruction contained bad identifier."));
            },
        }

        self.push_int_onto_stack(result)?;

        Ok(result)
    }

    fn unary_arithmetic(&mut self, instruction: u32) -> Result<(), String> {
        let operand = self.pop_int_from_stack()? as i32;
        let which_seperated = instruction & (0xf << 24);
        let which_operation = which_seperated >> 24;
        let result: i32;

        match which_operation {
            0 => {
                result = -operand;
            },
            1 => { 
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

        Ok(()) 
    }
       
    fn ret(&mut self, instruction: u32) -> Result<(), String> {
        // Extract stack offset from bits 27:2 (always a multiple of 4)
        let offset_raw = instruction & 0x0FFF_FFFC;
        let offset = offset_raw as i32;

        // Then pop the return address
        //self.print_stack();
        //self.print_vm_info();

        // Free the stack frame first
        self.stack_pointer += offset;

        let return_address = self.pop_int_from_stack()? as i32;

        // Adjust program counter
        self.program_counter = return_address;
        self.program_counter -= 4;
        
        // println!(
        //     "DEBUG: ret – return_address={}, freed_offset={}, new_sp={}",
        //     return_address,
        //     offset,
        //     self.stack_pointer
        // );

        Ok(())
    }

    fn goto(&mut self, instruction: u32) -> Result<(), String>{
        //TODO: make sure offset is signed
        let extracted = (instruction >> 2) & 0x03FF_FFFF; // 26 bits
        let offset: i32;
        // Check if the sign bit (bit 25 after shift) is set
        if extracted & (1 << 25) != 0 {
            // Sign-extend: set upper bits to 1
            offset = (extracted | !0x03FF_FFFF) as i32;
        } else {
            offset = extracted as i32;
        }

        //TODO: fix offset calc
        /* offset += self.program_counter;
        self.program_counter = offset;*/

        /*TO TEST PLEASE*/
        self.program_counter += offset << 2;
        // shift left 2 to convert to bytes
        self.program_counter -= 4;
        //bc run() increments pc by 4
        Ok(())
    }

    fn print(&mut self, instruction: u32) -> Result<(), String>{
        let offset_mask = (1 << 26) - 1;
        let mut offset: i32 = (instruction as i32 >> 2) & offset_mask;
        offset <<= 2;
        if instruction & (1 << 25) != 0 {
            offset |= !offset_mask;
        }

        let fmt: i8 = instruction as i8 & 3;
        let val = self.peek_int_from_stack(offset)? as i32;

        // println!("o:{} om:{:x} i:{:x}", offset, offset_mask, instruction);
        //println!("o:{} om:{:x} i:{:x}", offset, offset_mask, instruction);

        match fmt {
            0 => println!("{}", val),
            1 => println!("0x{:x}", val),
            2 => println!("0b{:b}", val),
            3 => println!("0o{:o}", val),
            _ => {
                return Err(String::from("print: faulty format code."));
            }
        };

        Ok(())
    }

    fn binary_if(&mut self, instruction: u32) -> Result<(), String>{
        /*let offset: i32 = (instruction as i32 >> 2) & 0x3FFFFF;
        let cond: u32 = (instruction >> 25) & 0x7;
        let lhs = self.peek_int_from_stack(4).unwrap_or(0);
        let rhs = self.peek_int_from_stack(0).unwrap_or(0);*/
        //println!("cond = {}", cond);
        let offset_mask = (1 << 25) - 1;
        let mut offset: i32 = instruction as i32 & offset_mask;
        if instruction & (1 << 24) != 0 {
            offset |= !offset_mask;
        }

        let cond_mask = (1 << 4) - 1;
        let cond = (instruction >> 25) & cond_mask;
        let lhs = self.peek_int_from_stack(4).unwrap_or(0);
        let rhs = self.peek_int_from_stack(0).unwrap_or(0);
        let result: bool;

        match cond{
            0 => {
                result = lhs == rhs;
            },
            1 => {
                result = lhs != rhs;
            },
            2 => {
                result = lhs < rhs;
            },
            3 => {
                result = lhs > rhs; 
            },
            4 => {
                result = lhs <= rhs;
            },
            5 => {
                result = lhs >= rhs;
            },
            _ => {
                return Err(String::from("Binary if: faulty instruction."));
            }
        };

        if result {
            self.program_counter += offset;
            /* Band-aid fix. :) */
            self.program_counter -= 4;
        }
        
        Ok(())
    }

    fn unary_if(&mut self, instruction: u32) -> Result<(), String>{
        let offset_mask = (1 << 25) - 1;
        let condition_mask = (1 << 2) - 1;

        let mut offset = (instruction as i32) & offset_mask;
        if instruction & (1 << 24) != 0 {
            offset |= !offset_mask;
        }
        let condition = (instruction >> 25) & condition_mask;
        let peek = self.peek_int_from_stack(0)? as i32;
        let result: bool;

        match condition {
            0 => {
                result = peek == 0;
            },
            1 => {
                result = peek != 0;
            },
            2 => {
                result = peek < 0;
            },
            3 => {
                result = peek > 0;
            },
            _ => {
                return Err(String::from("Unary if: faulty instruction."));
            },
        }

        if result {
            self.program_counter += offset;
            /* Band-aid fix. :) */
            self.program_counter -= 4;
        }

        Ok(())
    }

    fn dump(&self) -> Result<(), String>{
        let start = self.stack_pointer as usize;
        //if stack empty gtfo
        if start == 4096 {
            return Ok(());
        }
        //read through stack 4 bytes at a time
        // let mut offset = 0;
        for i in (start..4096).step_by(4) {
            if i + 4 > self.stack.len() {
                break;
            }
            //start converting bytes from i
            let word_bytes = &self.stack[i..i+4];
            let word = u32::from_be_bytes(word_bytes.try_into().unwrap());
            println!("{:04x}: {:08x}", i, word);
            // offset += 1;
        }
        Ok(())
    }

    fn stprint(&self, instruction: u32) -> Result<(), String> {
        let mut stack_offset = (instruction as i32) & !(0xf << 28);
        if stack_offset & (1 << 27) != 0 {
            /* Sign extend. */
            stack_offset |= 0xf << 28;
        }
    
        let start_address = self.stack_pointer + stack_offset;
        if start_address >= 4096 || start_address < 0 {
            return Err(String::from("stprint: Offset out of range."));
        }

        /* The actual print loop. */
        let start_index = start_address as usize;
        let stack_size = self.stack.len();
        let mut last_char_set = -1;
        let mut d = VecDeque::new();
        for i in start_index..stack_size {
            let cur = self.stack[i];

            /* Since strings are stored in three-byte chunklets prefaced by numbers, when we hit a
             * null terminator, we don't die right away, instead we print the last three bytes. */
            if cur == 0 || last_char_set != -1 {
                last_char_set += 1;
            }
            if last_char_set > 3 {
                break;
            }
            /* Don't add numbers to the deque. */
            if cur == 0 || cur == 1 {
                continue;
            }

            //print!("{}", cur as char);
            d.push_front(cur as char);
            if d.len() == 3 {
                for c in &d {
                    print!("{}", c);
                }

                d.clear();
            }
        }

        if !d.is_empty() {
            for c in &d {
                print!("{}", c);
            }
        }

        stdout().flush().expect("Failed to flush buffer");

        Ok(())
    }

    fn dup(&mut self, instruction: u32) -> Result<(), String> {
        let offset_mask = (1 << 28) - 1;

        /* Marz's handles negative offsets. Sounds horrible to me, but who cares anymore. It's
         * 1:39am on a Monday morning and I just wanna go to bed! */
        let mut offset = instruction as i32 & offset_mask;
        if instruction & (1 << 27) != 0 {
            offset |= !offset_mask;
        }

        let peek = self.peek_int_from_stack(offset)? as i32;
        self.push_int_onto_stack(peek)?;

        Ok(())
    }
}
