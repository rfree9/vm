use std::env;
use std::process;
use std::fs;
use std::error;

struct VirtualMachine {
    stack: Vec<u8>,
    stack_ptr: i32,
    prog_counter: i32,
}

impl VirtualMachine {
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

        Ok(VirtualMachine {
            stack,
            stack_ptr: 4096,
            prog_counter: 0,
        })
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let vm = VirtualMachine::build(&args).unwrap_or_else(|err| {
        eprintln!("{}", err);
        process::exit(1);
    });
}
