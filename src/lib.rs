use std::fs;

pub struct VirtualMachine {
    stack: Vec<u8>,
    stack_pointer: i32,
    program_counter: i32,
}

impl VirtualMachine {
    /* Constructor. */
    pub fn build(args: &[String]) -> Result<VirtualMachine, &str> {
        if args.len() != 2 {
            return Err("usage: vm <file.v>");
        }

        /* Verifying the file is valid. */

        let file_result = fs::read(&args[1]);
        let mut file_buf = match file_result {
            Ok(file_buf) => file_buf,
            Err(_) => return Err("Couldn't open file."),
        };

        if file_buf.len() > (4096 + 4) {
            return Err("File too big.");
        }

        if file_buf.len() < 4 || file_buf[0..4] != vec![0xde, 0xad, 0xbe, 0xef] {
            return Err("File format is invalid.");
        }

        /* Creating the stack. */

        let mut stack = file_buf.split_off(4);
        stack.resize(4096, 0);

        /* Creating the struct. */

        Ok(VirtualMachine {
            stack,
            stack_pointer: 4096,
            program_counter: 0,
        })
    }

    /* Parse and execute instructions from the stack. */
    pub fn run(&mut self) -> Result<(), &str> {

        /* TODO: This is obviously very rudimentary, it just parses instructions until it hits an
         * exit instruction. I just wanted to put something down for the sake of having some
         * organization.
         * - Functions should be made to handle each op code.
         * - The Ok() side of the Result should be the exit code, that way main() can use it.
         * - We should probably move all of this shit to a lib.rs file sooner than later. */

        loop {
            let instruction = self.get_next_instruction();
            let opcode = VirtualMachine::get_op_code(instruction);

            print!("opcode: {:x} -- ", VirtualMachine::get_op_code(instruction));
            match opcode {
                0 => println!("Misc. instruction"),
                1 => println!("Pop instruction"),
                2 => println!("Binary arithmetic instruction"),
                3 => println!("Unary arithmetic instruction"),
                4 => println!("String print instruction"),
                5 => println!("Call instruction"),
                6 => println!("Return instruction"),
                7 => println!("Unconditional goto instruction"),
                8 => println!("Binary if instruction"),
                9 => println!("Unary if instruction"),
                12 => println!("Dup instruction"),
                13 => println!("Print instruction"),
                14 => println!("Dump instruction"),
                15 => println!("Push instruction"),
                _ => return Err("Bad instruction."),
            }

            self.increment_program_counter();
            if instruction == 0 {
                break;
            }
        }

        Ok(())
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
}
