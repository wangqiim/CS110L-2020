use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::process::Child;
use std::process::Command;
use std::os::unix::process::CommandExt;
use crate::dwarf_data::{DwarfData};

pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

impl Status {
    pub fn print_status(&self) {
        match self {
            Status::Stopped(sig, _) => {
                println!("inferior stopped due to a signal: {}", sig.as_str());
            },
            Status::Exited(code) => {
                println!("inferior exited exit status code: {}", code);
            },
            Status::Signaled(sig) => {
                println!("inferior exited due to a signal: {}", sig.as_str());
            }
        }
    }
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
        let mut cmd = Command::new(target);
        let cmd = cmd.args(args);
        unsafe {
            cmd.pre_exec(child_traceme);
        }
        let child = cmd.spawn().ok()?;
        let inferior = Inferior{ child: child };

        if let Ok(Status::Stopped(sig, _)) = inferior.wait(None) {
            if sig == signal::Signal::SIGTRAP {
                return Some(inferior);
            }
        }
        None
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn cont(&self) -> Result<Status, nix::Error> {
        match ptrace::cont(self.pid(), None) {
            Ok(_) => {
                self.wait(None)
            },
            Err(_) => {
                panic!("have't proccessed");
            },
        }
    }

    pub fn kill_and_reap(&mut self) {
        self.child.kill().expect("have't proccessed");
        self.wait(None).expect("have't proccessed");
    }

    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let regs = ptrace::getregs(self.pid()).unwrap();
        // println!("%rip register: {:#x}", regs.rip);
        // println!("{}", regs.rip as usize);
        let mut rip_ptr = regs.rip as usize;
        let mut base_ptr = regs.rbp as usize;
        loop {
            let func_name = debug_data.get_function_from_addr(rip_ptr).unwrap();
            let line = debug_data.get_line_from_addr(rip_ptr).unwrap();
            println!("{} ({})", func_name, line);
            if func_name == "main" {
                break;
            }
            rip_ptr = ptrace::read(self.pid(), (base_ptr + 8) as ptrace::AddressType).unwrap() as usize;
            base_ptr = ptrace::read(self.pid(), base_ptr as ptrace::AddressType).unwrap() as usize;
        }
        Ok(())
    }
}
