use std::env;
use std::process;
use vm::VirtualMachine;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut vm = VirtualMachine::build(&args).unwrap_or_else(|err| {
        eprintln!("{}", err);
        process::exit(1);
    });

    let vm_result = vm.run();

    match vm_result {
        Ok(exit_code) => process::exit(exit_code),
        Err(error) => {
            eprintln!("{}", error);
            process::exit(1);
        }
    }
}
