pub enum Anchor {
    EYES,
    FEET,
}

pub enum Command {
    Breakpoint,
    FunctionCall { name: String, anchor: Anchor },
    OtherCommand,
}

pub fn parseCommand(line: &str) -> Command {
    unimplemented!()
}
