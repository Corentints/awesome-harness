use agentctx::{
    analysis::find_corrections,
    config::{Config, default_user_config_path},
    domain::NormalizedSession,
    ingest::{ClaudeSessionSource, CodexSessionSource, SessionSource},
    project::Project,
    repository,
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
    /// Detect a project and create its default configuration.
    Init(InitArgs),
    /// Report repeated explicit instructions and agent corrections.
    Mistakes(MistakesArgs),
}

#[derive(Debug, Args)]
struct InitArgs {
    /// Git project to initialize.
    #[arg(default_value = ".")]
    project: PathBuf,
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
        Some(Command::Init(arguments)) => init(&arguments)?,
        Some(Command::Mistakes(arguments)) => mistakes(&arguments)?,
        None => println!("Run `agentctx --help` to get started."),
    }
    Ok(())
}

fn init(arguments: &InitArgs) -> Result<()> {
    let project = Project::detect(&arguments.project).context("could not detect project")?;
    let config_path = Config::write_project_default(&project.root)?;
    let config = Config::load(default_user_config_path().as_deref(), &project.root)?;
    let facts = repository::scan(&project)?;

    println!("Project detected: {}", project.name);
    println!("Tracked files: {}", facts.tracked_files.len());
    println!(
        "Package manager: {}",
        facts.package_manager.as_deref().unwrap_or("not detected")
    );
    println!(
        "Test tools: {}",
        if facts.test_tools.is_empty() {
            "none".to_owned()
        } else {
            facts.test_tools.join(", ")
        }
    );
    println!("Existing instructions: {}", facts.instructions.len());
    print_source_availability("Claude Code", default_claude_root());
    print_source_availability("Codex", default_codex_root());
    println!("Configuration: {}", config_path.display());
    println!("Minimum occurrences: {}", config.analysis.min_occurrences);
    Ok(())
}

fn print_source_availability(name: &str, path: Option<PathBuf>) {
    match path {
        Some(path) if path.is_dir() => println!("{name} sessions: available ({})", path.display()),
        Some(path) => println!("{name} sessions: not found ({})", path.display()),
        None => println!("{name} sessions: home directory unavailable"),
    }
}

fn default_claude_root() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude/projects"))
}

fn default_codex_root() -> Option<PathBuf> {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .map(|root| root.join("sessions"))
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
            let message = evidence
                .message_id
                .as_ref()
                .map_or("unknown-message", agentctx::domain::MessageId::as_str);
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
