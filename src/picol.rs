/*
    Implementation of Tcl interpreter in Rust
*/

use std::{collections::HashMap, hash::Hash, marker::PhantomData, process::Command};

#[derive(Debug, PartialEq)]
pub enum PicolResult {
    PicolOk, PicolErr, PicolReturn,PicolBreak,PicolContinue
}

#[derive(Debug, PartialEq, Clone)]
pub enum PicolType {
    PTEsc, PTStr, PTCmd, PTVar, PTSep, PTEol, PTEof
}

/* Picol Parser */
struct PicolParser<'a> {
    string : &'a String,
    pos : usize, // current text position
    len : usize, // remaining length 
    start : usize, // start of current token
    end : usize, // end of current token
    typ : PicolType,
    inside_quotes : bool,
}

struct PicolVar {
    name : String,
    value : String,
    next : u32, // Index of the next var, lets keep it around, we can remove it later if needed
}

struct PicolCmd
{
    name : String, 
    command_func : PicolCommandFunc,
    private_data : Vec<String>,
    next : Option<Box<PicolCmd>>
}

struct PicolCallFrame {
    vars : HashMap<String, PicolVar>,
    parent: Option<Box<PicolCallFrame>>
}

pub struct PicolInterpreter {
    level : u32, 
    commands_head : Option<Box<PicolCmd>>, 
    callframes_head : Option<Box<PicolCallFrame>>, 
    pub result : String
}


impl<'a> PicolParser<'a> {
    fn new(s : &'a String) -> PicolParser<'a> {
        PicolParser {
            string : s,
            pos : 0,
            len : s.len(),
            start : 0,
            end : 0,
            typ : PicolType::PTEol,
            inside_quotes : false,
        }
    }

    fn parse_sep(&mut self) -> PicolResult {
        self.start = self.pos;
        while self.pos < self.len {
            let c: char = self.string.chars().nth(self.pos).unwrap();
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                self.pos += 1;
                self.len -= 1;
            } else {
                break;
            }
        }
        self.end = self.pos-1;
        self.typ = PicolType::PTSep;
        return PicolResult::PicolOk;
    }

    fn parse_eol(&mut self) -> PicolResult {
        self.start = self.pos;
        while self.pos < self.len {
            let c: char = self.string.chars().nth(self.pos).unwrap();
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == ';' {
                self.pos += 1;
                self.len -= 1;
            } else {
                break;
            }
        }
        self.end = self.pos-1;
        self.typ = PicolType::PTEol;
        return PicolResult::PicolOk;
    }

    fn parse_command(&mut self) -> PicolResult {
        let mut level: i32 = 1;  
        let mut blevel : i32 = 0;
        self.pos += 1;
        self.start = self.pos;
        self.len -= 1;
        loop {
            if self.len == 0 { 
                break;
            }
            let c: char = self.string.chars().nth(self.pos).unwrap();
            if c == '[' && blevel == 0 {
                level += 1;
            } else if c == ']' && blevel == 0 {
                level -= 1;
                if level == 0 {
                    break;
                }
            } else if c == '{' {
                blevel += 1;
            } else if c == '}' {
                blevel -= 1;
            } else if c == '\\' {
                self.pos += 1;
                self.len -= 1;
            }
            self.pos += 1;
            self.len -= 1;
        }
        self.end = self.pos-1;
        self.typ = PicolType::PTCmd;
        let c : char = self.string.chars().nth(self.pos).unwrap();
        if c == ']' {
            self.pos += 1;
            self.len -= 1;
        }
        return PicolResult::PicolOk;
    }

    fn parse_var(&mut self) -> PicolResult {
        self.pos += 1;
        self.start = self.pos;
        self.len -= 1;
        loop {
            let c: char = self.string.chars().nth(self.pos).unwrap();
            if c.is_alphanumeric() || c == '_' {
                self.pos += 1;
                self.len -= 1;
            } else {
                break;
            }
        }
        if self.start == self.pos {
            self.start = self.pos-1;
            self.end = self.pos-1;
            self.typ = PicolType::PTStr;
        } else {
            self.end = self.pos-1;
            self.typ = PicolType::PTVar;
        }
        return PicolResult::PicolOk;
    }

    fn parse_brace(&mut self) -> PicolResult {
        let mut level: i32 = 1;
        self.pos += 1;
        self.start = self.pos;
        self.len -= 1;
        loop {
            let c: char = self.string.chars().nth(self.pos).unwrap();
            if self.len >= 2 && c == '\\' {
                self.pos += 1;
                self.len -= 1;
            } else if (self.len == 0 || c == '}') {
                level -= 1;
                if level == 0 || self.len == 0{
                    self.end = self.pos-1;
                    if self.len > 0 {
                        // Skip final closed brace
                        self.pos += 1;
                        self.len -= 1;
                    }
                    self.typ = PicolType::PTStr;
                    return PicolResult::PicolOk;
                }
            } else if (c == '{') {
                level += 1;
            }
            self.pos += 1;
            self.len -= 1;
        }
    }

    fn parse_string(&mut self) -> PicolResult {
        let is_new_word : bool = (self.typ == PicolType::PTEol || self.typ == PicolType::PTSep || self.typ == PicolType::PTStr);
        if is_new_word {
            let c : char = self.string.chars().nth(self.pos).unwrap();
            if c == '{' {
                return self.parse_brace();
            } else if c == '"' {
                self.inside_quotes = true; 
                self.pos += 1;
                self.len -= 1;
            }
        }
        self.start = self.pos;
        loop {
            if self.len == 0 {
                self.end = self.pos-1;
                self.typ = PicolType::PTEsc;
                return PicolResult::PicolOk;
            } 
            let c: char = self.string.chars().nth(self.pos).unwrap();
            if c == '\\' {
                if self.len >= 2 {
                    self.pos += 1;
                    self.len -= 1;
                }
            } else if c == '$' || c == '[' {
                self.end = self.pos-1;
                self.typ = PicolType::PTEsc;
                return PicolResult::PicolOk;
            } else if c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == ';' {
                if !self.inside_quotes {
                    self.end = self.pos-1;
                    self.typ = PicolType::PTEsc;
                    return PicolResult::PicolOk;
                }
            } else if c == '"' {
                if self.inside_quotes {
                    self.end = self.pos-1;
                    self.typ = PicolType::PTEsc;
                    self.pos += 1;
                    self.len -= 1;
                    self.inside_quotes = false;
                    return PicolResult::PicolOk;
                }
            }
            self.pos += 1;
            self.len -= 1;
        }
    }

    fn parse_comment(&mut self) -> PicolResult {
        while self.len > 0 {
            let c: char = self.string.chars().nth(self.pos).unwrap();
            if c == '\n' {
                break;
            }
            self.pos += 1;
            self.len -= 1;
        }
        return  PicolResult::PicolOk;
    }

    fn get_token(&mut self) -> PicolResult {
        loop {
            if (self.len == 0) {
                if (self.typ != PicolType::PTEol && self.typ != PicolType::PTEof) {
                    self.typ = PicolType::PTEol;
                } else {
                    self.typ = PicolType::PTEof;
                }
                return PicolResult::PicolOk;
            }
            let c: char = self.string.chars().nth(self.pos).unwrap();
            if c == ' ' || c == '\t' || c == '\r' {
                if self.inside_quotes {
                    return self.parse_string();
                } 
                return self.parse_sep();
            } else if c == '\n' || c == ';' {
                if self.inside_quotes {
                    return self.parse_string();
                } 
                return self.parse_eol();
            } else if c == '[' {
                return self.parse_command();
            } else if c == '$' {
                return self.parse_var();
            } else if c == '#' {
                if self.typ == PicolType::PTEol {
                    self.parse_comment();
                    continue;
                } 
                return self.parse_string();
            } else {
                return self.parse_string();
            }
        }
    }
}

impl PicolCallFrame {
    fn new() -> PicolCallFrame {
        PicolCallFrame {
            vars : HashMap::new(),
            parent : None
        }
    }
}

type PicolCommandFunc = fn (&mut PicolInterpreter, u32, &Vec<String>, &Vec<String>) -> PicolResult;

impl PicolCmd {
    fn new(name : String, command_func : PicolCommandFunc, private_data : Vec<String>) -> PicolCmd {
        PicolCmd {
            name : name,
            command_func : command_func,
            private_data : private_data,
            next : None
        }
    }
}

impl PicolInterpreter {
    pub fn new() -> PicolInterpreter {
        PicolInterpreter {
            level : 0,
            commands_head : None,
            callframes_head : Some(Box::new(PicolCallFrame::new())),
            result : String::new()
        }
    }

    fn set_result(&mut self, s : &String) {
        self.result = s.clone();
    }

    fn get_var(&mut self, name : &String) -> Option<&mut PicolVar> {
        let mut cf = self.callframes_head.as_mut().unwrap();
        // Get from current frame hashmap 
        return cf.vars.get_mut(name);
    }

    fn set_var(&mut self, name : &String, value : &String) -> PicolResult {
        let mut var = self.get_var(name);
        // Match 
        match var {
            Some(v) => {
                v.value = value.clone();
            },
            None => {
                let mut cf = self.callframes_head.as_mut().unwrap();
                cf.vars.insert(name.clone(), PicolVar { name : name.clone(), value : value.clone(), next : 0 });
            }
        }
        return PicolResult::PicolOk;
    }

    fn get_command(&mut self, name : &String) -> Option<&mut PicolCmd> {
        let mut c = self.commands_head.as_mut();
        while let Some(cmd) = c {
            if cmd.name == *name {
                return Some(cmd);
            }
            c = cmd.next.as_mut();
        }
        return None;
    }

    fn register_command(&mut self, name : &String, command_func : PicolCommandFunc, private_data : Vec<String>) -> PicolResult {
        // Check if command already exists
        let mut c = self.get_command(name);
        match c {
            Some(_) => {
                self.set_result(&format!("Command {} already exists", name));
                return PicolResult::PicolErr;
            },
            None => {
                let mut cmd = Box::new(PicolCmd::new(name.clone(), command_func, private_data));
                cmd.next = self.commands_head.take();
                self.commands_head = Some(cmd);
                return PicolResult::PicolOk;
            }
        }
    }

    pub fn eval(&mut self, t : &String) -> PicolResult {
        let mut parser = PicolParser::new(t);
        let mut argc : u32 = 0;
        let mut argv : Vec<String> = Vec::new();
        let mut retcode : PicolResult = PicolResult::PicolOk;
        self.set_result(&String::new());

        loop {
            let mut prev_type = &parser.typ.clone();
            let res = parser.get_token();
            if parser.typ == PicolType::PTEof {
                break;
            }

            // Get the token as a copy
            let mut token = parser.string[parser.start..parser.end].to_string();
            let tlen = token.len();

            if parser.typ == PicolType::PTVar {
                let var = self.get_var(&token);
                match var {
                    Some(v) => {
                        token = v.value.clone();
                    },
                    None => {
                        self.set_result(&format!("Unknown variable {}", token));
                        return PicolResult::PicolErr;
                    }
                }
            } else if parser.typ == PicolType::PTCmd {
                retcode = self.eval(&token);
                if (retcode != PicolResult::PicolOk) {
                    return retcode;
                }
            } else if parser.typ == PicolType::PTEsc {
                // XXX: escape handling missing
            } else if parser.typ == PicolType::PTSep {
                prev_type = &parser.typ.clone();
                continue;
            }
            /* We have a complete command + args. Call it! */
            if parser.typ == PicolType::PTEol {
                prev_type = &parser.typ.clone();
                if argc > 0 {
                    let cmd = self.get_command(&argv[0]);
                    match cmd {
                        Some(c) => {
                            let fun = c.command_func;
                            let pd = c.private_data.clone();
                            retcode = fun(self, argc, &argv, &pd);
                            if retcode != PicolResult::PicolOk {
                                return retcode;
                            }
                        },
                        None => {
                            self.set_result(&format!("Unknown command {}", argv[0]));
                            return PicolResult::PicolErr;
                        }
                    }
                }
                /* Prepare for the next command */
                argc = 0;
                argv.clear();
                continue;
            }
            /* We have a new token, append to the previous or as new arg? */
            if prev_type == &PicolType::PTSep || prev_type == &PicolType::PTEol {
                argc += 1;
                argv.push(token);
            } else { /* Interpolation */
                // Combine the last two tokens
                let last = argv.pop().unwrap();
                let new_token = last + &token;
                argv.push(new_token);
            }
            prev_type = &parser.typ.clone();
        }
        return retcode;
        
    }

    fn drop_callframe(&mut self) {
        let mut cf = self.callframes_head.as_mut().unwrap();
        cf.vars.clear();
        self.callframes_head = cf.parent.take();
    }

    pub fn register_core_commands(&mut self) {
        self.register_command(&"+".to_string(), picol_cmd_math, vec![]);
        self.register_command(&"-".to_string(), picol_cmd_math, vec![]);
        self.register_command(&"*".to_string(), picol_cmd_math, vec![]);
        self.register_command(&"/".to_string(), picol_cmd_math, vec![]);
        self.register_command(&">".to_string(), picol_cmd_math, vec![]);
        self.register_command(&"<".to_string(), picol_cmd_math, vec![]);
        self.register_command(&">=".to_string(), picol_cmd_math, vec![]);
        self.register_command(&"<=".to_string(), picol_cmd_math, vec![]);
        self.register_command(&"==".to_string(), picol_cmd_math, vec![]);
        self.register_command(&"!=".to_string(), picol_cmd_math, vec![]);
        self.register_command(&"set".to_string(), picol_cmd_set, vec![]);
        self.register_command(&"puts".to_string(), picol_cmd_puts, vec![]);
        self.register_command(&"if".to_string(), picol_cmd_if, vec![]);
        self.register_command(&"while".to_string(), picol_cmd_while, vec![]);
        self.register_command(&"break".to_string(), picol_cmd_retcodes, vec!["break".to_string()]);
        self.register_command(&"continue".to_string(), picol_cmd_retcodes, vec!["continue".to_string()]);
        self.register_command(&"proc".to_string(), picol_cmd_proc, vec![]);
        self.register_command(&"return".to_string(), picol_cmd_return, vec![]);
    }

}

/* Implementation of the actual commands */ 

fn picol_arrity_error(interpreter : &mut PicolInterpreter, name : &String) -> PicolResult {
    interpreter.set_result(&format!("Wrong number of arguments for {}", name).to_string());
    return PicolResult::PicolErr;
}

fn picol_cmd_math(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, _pd : &Vec<String>) -> PicolResult {
    if argc != 3 {
        return picol_arrity_error(interpreter, &argv[0]);
    }
    let a = argv[1].parse::<i32>().unwrap();
    let b = argv[2].parse::<i32>().unwrap();
    let result : i32;
    match argv[0].as_str() {
        "+" => result = a + b,
        "-" => result = a - b,
        "*" => result = a * b,
        "/" => {
            if b == 0 {
                interpreter.set_result(&"Division by zero".to_string());
                return PicolResult::PicolErr;
            }
            result = a / b;
        },
        ">" => result = if a > b { 1 } else { 0 },
        "<" => result = if a < b { 1 } else { 0 },
        ">=" => result = if a >= b { 1 } else { 0 },
        "<=" => result = if a <= b { 1 } else { 0 },
        "==" => result = if a == b { 1 } else { 0 },
        "!=" => result = if a != b { 1 } else { 0 },
        _ => result = 0
    }
    interpreter.set_result(&result.to_string());
    return PicolResult::PicolOk;
}

fn picol_cmd_set(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, _pd : &Vec<String>) -> PicolResult {
    if argc != 3 {
        return picol_arrity_error(interpreter, &argv[0]);
    }
    interpreter.set_var(&argv[1], &argv[2]);
    interpreter.set_result(&argv[2]);
    return PicolResult::PicolOk;
}

fn picol_cmd_puts(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, _pd : &Vec<String>) -> PicolResult {
    if argc != 2 {
        return picol_arrity_error(interpreter, &argv[0]);
    }
    println!("{}", argv[1]);
    return PicolResult::PicolOk;
}

fn picol_cmd_if(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, _pd : &Vec<String>) -> PicolResult {
    if argc != 3 && argc != 5 {
        return picol_arrity_error(interpreter, &argv[0]);
    }
    let mut retcode = interpreter.eval(&argv[1]);
    if retcode != PicolResult::PicolOk {
        return retcode;
    }
    // if interpreter result is integer 1, then evaluate the true branch
    if interpreter.result == "1" {
        return interpreter.eval(&argv[2]);
    } else if argc == 5 {
        return interpreter.eval(&argv[4]);
    }
    return PicolResult::PicolOk;
}

fn picol_cmd_while(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, _pd : &Vec<String>) -> PicolResult {
    if argc != 3 {
        return picol_arrity_error(interpreter, &argv[0]);
    }
    loop {
        let mut retcode = interpreter.eval(&argv[1]);
        if retcode != PicolResult::PicolOk {
            return retcode;
        }
        if interpreter.result != "1" {
            return PicolResult::PicolOk;
        } else {
            retcode = interpreter.eval(&argv[2]);
            if (retcode == PicolResult::PicolContinue) {
                continue;
            } else if (retcode == PicolResult::PicolBreak) {
                return PicolResult::PicolOk;
            } else if (retcode != PicolResult::PicolOk) {
                continue;
            } else {
                return retcode;
            }
        }
    }
}

fn picol_cmd_retcodes(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, _pd : &Vec<String>) -> PicolResult {
    if argc != 1 {
        return picol_arrity_error(interpreter, &argv[0]);
    }
    if argv[0] == "break" {
        return PicolResult::PicolBreak;
    } else if argv[0] == "continue" {
        return PicolResult::PicolContinue;
    } 
    return PicolResult::PicolOk;
}

fn picol_cmd_call_proc(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, pd : &Vec<String>) -> PicolResult {
    let arg_ls = pd[0].clone();
    let body = pd[1].clone();

    let mut cf = Box::new(PicolCallFrame::new());
    cf.parent = interpreter.callframes_head.take();
    interpreter.callframes_head = Some(cf);

    // Parse the arguments
    let args : Vec<&str> = arg_ls.split_whitespace().collect();
    if args.len() != (argc - 1) as usize {
        interpreter.set_result(&format!("Wrong number of arguments for {}", argv[0]));
        return PicolResult::PicolErr;
    }

    for i in 0..args.len() {
        interpreter.set_var(&args[i].to_string(), &argv[i+1]);
    }

    let mut retcode = interpreter.eval(&body);
    if retcode == PicolResult::PicolReturn {
        retcode = PicolResult::PicolOk;
    }
    interpreter.drop_callframe();
    return retcode;

}

fn picol_cmd_proc(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, _pd : &Vec<String>) -> PicolResult {
    if argc != 4 {
        return picol_arrity_error(interpreter, &argv[0]);
    }

    let procdata =  vec![argv[2].clone(), argv[3].clone()];
    return interpreter.register_command(&argv[1], picol_cmd_call_proc, procdata);
}

fn picol_cmd_return(interpreter : &mut PicolInterpreter, argc : u32, argv : &Vec<String>, _pd : &Vec<String>) -> PicolResult {
    if argc != 1 && argc != 2 {
        return picol_arrity_error(interpreter, &argv[0]);
    }
    let res = if argc == 2 { argv[1].clone() } else { String::new() };
    interpreter.set_result(&res);
    return PicolResult::PicolReturn;
}