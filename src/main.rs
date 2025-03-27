use std::env;

mod picol;

fn main() {
    let mut interpreter = picol::PicolInterpreter::new();
    interpreter.register_core_commands();

    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        loop {
            // Print picol> 
            print!("picol> ");
            // Read a line from the user
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            // Evaluate the input
            let retcode = interpreter.eval(&input);
            if interpreter.result.len() > 0 {
                println!("{:?} {}", retcode, interpreter.result);
            }
        }
    } else if args.len() == 2 {
        // Read the file 
        let filename = &args[1];
        let contents = std::fs::read_to_string(filename).expect("Something went wrong reading the file");
        // Evaluate the input
        let retcode = interpreter.eval(&contents);
        if interpreter.result.len() > 0 {
            println!("{:?} {}", retcode, interpreter.result);
        }
    }
}
