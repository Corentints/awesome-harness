use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "agentctx", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print the currently implemented capabilities.
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Status) => println!("AgentContext prototype: session ingestion"),
        None => println!("Run `agentctx --help` to get started."),
    }
}
