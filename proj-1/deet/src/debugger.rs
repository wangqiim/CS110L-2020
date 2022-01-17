use crate::debugger_command::DebuggerCommand;
use crate::inferior::Inferior;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use crate::inferior::{Status, Breakpoint};
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use std::collections::HashMap;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    break_points: HashMap<usize, Breakpoint>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };

        // debug point
        debug_data.print();

        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data: debug_data,
            break_points: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    // when you pause an inferior using ctrl+c, then type run, 
                    // You should take care to kill any existing inferiors before starting new ones
                    if let Some(ref mut inferior) = self.inferior {
                        inferior.kill_and_reap();
                    }
                    if let Some(inferior) = Inferior::new(&self.target, &args, &mut self.break_points) {
                        // Create the inferior
                        self.inferior = Some(inferior);
                        // TODO (milestone 1): make the inferior run
                        // You may use self.inferior.as_mut().unwrap() to get a mutable reference
                        // to the Inferior object
                        match self.inferior.as_mut().unwrap().cont(&self.break_points) {
                            Ok(status) => {
                                status.print_status(&self.debug_data);
                                // reset self.inferior if it exit
                                match status {
                                    Status::Exited(_) | Status::Signaled(_) => self.inferior = None,
                                    _ => {},
                                }
                            },
                            Err(_) => {
                                println!("Error: continue subprocess");
                            }
                        }
                    } else {
                        println!("Error starting subprocess");
                    }
                },
                DebuggerCommand::Continue => {
                    match self.inferior {
                        Some(ref mut inferior) => {
                            match inferior.cont(&self.break_points) {
                                Ok(status) => {
                                    status.print_status(&self.debug_data);
                                    // reset self.inferior if it exit
                                    match status {
                                        Status::Exited(_) | Status::Signaled(_) => self.inferior = None,
                                        _ => {},
                                    }
                                },
                                Err(_) => {
                                    println!("Error: continue subprocess");
                                }
                            }
                        },
                        None => {
                            println!("Error: there is not a inferior, you shold type run at first");
                        }
                    }
                },
                DebuggerCommand::Quit => {
                    // if you exit DEET while a process is paused, 
                    // You should terminate the inferior if one is running.
                    if let Some(ref mut inferior) = self.inferior {
                        inferior.kill_and_reap();
                    }
                    return;
                },
                DebuggerCommand::BackTrace => {
                    match self.inferior {
                        Some(ref inferior) => {
                            inferior.print_backtrace(&self.debug_data).unwrap();
                        },
                        None => {
                            println!("Error: there is not a inferior, you should type run at first");
                        }
                    }
                },
                DebuggerCommand::BreakPoint(args) => {
                    self.break_point(args);
                },
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }

    fn break_point(&mut self, args: Vec<String>) {
        // check
        if args.len() != 1 {
            println!("Usage example: type break *0x0123456 ");
            return;
        }
        let rip: usize;
        // start with *
        if args[0].to_lowercase().starts_with("*") {
            let addr = &args[0][1..];
            rip = parse_address(addr).unwrap();
        } else if let Ok(line) = args[0].parse::<usize>() {
            rip = self.debug_data.get_addr_for_line(None, line).unwrap();
        } else if self.debug_data.get_addr_for_function(None, &args[0]).is_some() {
            rip = self.debug_data.get_addr_for_function(None, &args[0]).unwrap();
        } else {
            println!("Usage example:");
            println!("\tbreak *0x0123456 ");
            println!("\tbreak main");
            println!("\tbreak 15");
            return;
        }
        println!("Set breakpoint {} at {:#x}", self.break_points.len(), rip);
        self.break_points.insert(rip, Breakpoint::new(rip, 0));
    }
}

// breakpoint
fn parse_address(addr: &str) -> Option<usize> {
    let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
        &addr[2..]
    } else {
        &addr
    };
    usize::from_str_radix(addr_without_0x, 16).ok()
}
