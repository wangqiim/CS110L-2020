pub enum DebuggerCommand {
    Quit,
    Continue,
    BackTrace,
    Run(Vec<String>),
}

impl DebuggerCommand {
    pub fn from_tokens(tokens: &Vec<&str>) -> Option<DebuggerCommand> {
        match tokens[0] {
            "q" | "quit" => Some(DebuggerCommand::Quit),
            "c" | "cont" | "continue" => Some(DebuggerCommand::Continue),
            "r" | "run" => {
                let args = tokens[1..].to_vec();
                Some(DebuggerCommand::Run(
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            },
            "bt" | "back" | "backtrace" => Some(DebuggerCommand::BackTrace),
            // Default case:
            _ => None,
        }
    }
}
