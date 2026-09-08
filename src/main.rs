use agentctx::{
    analysis::find_corrections,
    domain::NormalizedSession,
    ingest::{ClaudeSessionSource, CodexSessionSource, SessionSource},
    project::Project,
};
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::path::{Path, PathBuf};

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
    /// Report repeated explicit instructions and agent corrections.
    Mistakes(MistakesArgs),
}

#[derive(Debug, Args)]
struct MistakesArgs {
    /// Directory containing Claude JSONL sessions.
    #[arg(long)]
    claude_root: Option<PathBuf>,
    /// Directory containing Codex rollout JSONL sessions.
    #[arg(long)]
    codex_root: Option<PathBuf>,
    /// Git project to which sessions should be restricted.
    #[arg(long, default_value = ".")]
    project: PathBuf,
    /// Include one-off signals in addition to repeated mistakes.
    #[arg(long)]
    include_single: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Status) => println!("AgentContext prototype: session ingestion"),
        Some(Command::Mistakes(arguments)) => mistakes(&arguments)?,
        None => println!("Run `agentctx --help` to get started."),
    }
    Ok(())
}

fn mistakes(arguments: &MistakesArgs) -> Result<()> {
    let project = Project::detect(&arguments.project).context("could not detect project")?;
    let mut sessions = Vec::new();
    if let Some(root) = &arguments.claude_root {
        load_sessions(&ClaudeSessionSource::new(root), &project, &mut sessions)?;
    }
    if let Some(root) = &arguments.codex_root {
        load_sessions(&CodexSessionSource::new(root), &project, &mut sessions)?;
    }

    if arguments.claude_root.is_none() && arguments.codex_root.is_none() {
        anyhow::bail!("provide --claude-root and/or --codex-root for the prototype");
    }

    let candidates = find_corrections(&sessions);
    println!("Repeated agent mistakes for {}", project.name);
    println!();
    let mut shown = 0;
    for candidate in candidates
        .iter()
        .filter(|candidate| arguments.include_single || candidate.occurrences > 1)
    {
        shown += 1;
        println!("{}× {}", candidate.occurrences, candidate.canonical_text);
        for evidence in &candidate.evidence {
            let message = evidence.message_id.as_deref().unwrap_or("unknown-message");
            println!("  - session {} / {}", evidence.session_id, message);
        }
    }
    if shown == 0 {
        println!("No repeated corrections found. Use --include-single to inspect all signals.");
    }
    Ok(())
}

fn load_sessions(
    source: &dyn SessionSource,
    project: &Project,
    sessions: &mut Vec<NormalizedSession>,
) -> Result<()> {
    for reference in source.discover()? {
        let session = source.parse(&reference)?;
        if session
            .project
            .as_deref()
            .is_none_or(|path| paths_match_project(path, project))
        {
            sessions.push(session);
        }
    }
    Ok(())
}

fn paths_match_project(path: &Path, project: &Project) -> bool {
    project.contains_path(path)
        || path
            .file_name()
            .is_some_and(|name| name == project.root.file_name().unwrap_or_default())
}
