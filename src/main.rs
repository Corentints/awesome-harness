use agentctx::{
    analysis::find_corrections,
    config::{Config, default_user_config_path},
    domain::NormalizedSession,
    ingest::{ClaudeSessionSource, CodexSessionSource, SessionSource},
    project::Project,
    repository,
    storage::{Database, DecisionStatus, ReviewDecision, SourceFingerprint, candidate_id},
};
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
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
    /// Analyze changed session sources and persist candidates.
    Analyze(AnalyzeArgs),
    /// List or decide candidate rules.
    Review(ReviewArgs),
    /// Explain a candidate and show its evidence.
    Explain(ExplainArgs),
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

#[derive(Debug, Args)]
struct AnalyzeArgs {
    #[arg(long)]
    claude_root: Option<PathBuf>,
    #[arg(long)]
    codex_root: Option<PathBuf>,
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[arg(long)]
    database: Option<PathBuf>,
}

#[derive(Clone, Debug, ValueEnum)]
enum ScopeArg {
    Global,
    Project,
    Session,
}

#[derive(Debug, Args)]
struct ReviewArgs {
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[arg(long)]
    database: Option<PathBuf>,
    #[arg(long, conflicts_with = "reject")]
    accept: Option<String>,
    #[arg(long, conflicts_with = "accept")]
    reject: Option<String>,
    #[arg(long, requires = "accept")]
    text: Option<String>,
    #[arg(long, value_enum, default_value = "project")]
    scope: ScopeArg,
    #[arg(long)]
    personal: bool,
}

#[derive(Debug, Args)]
struct ExplainArgs {
    id: String,
    #[arg(long, default_value = ".")]
    project: PathBuf,
    #[arg(long)]
    database: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Status) => println!("AgentContext prototype: session ingestion"),
        Some(Command::Init(arguments)) => init(&arguments)?,
        Some(Command::Mistakes(arguments)) => mistakes(&arguments)?,
        Some(Command::Analyze(arguments)) => analyze(&arguments)?,
        Some(Command::Review(arguments)) => review(&arguments)?,
        Some(Command::Explain(arguments)) => explain(&arguments)?,
        None => println!("Run `agentctx --help` to get started."),
    }
    Ok(())
}

fn analyze(arguments: &AnalyzeArgs) -> Result<()> {
    let project = Project::detect(&arguments.project).context("could not detect project")?;
    let facts = repository::scan(&project)?;
    let mut database = Database::open(&database_path(&project, arguments.database.as_deref()))?;
    let roots = analysis_roots(arguments);
    if roots.is_empty() {
        anyhow::bail!("no session source found; pass --claude-root and/or --codex-root");
    }

    let mut processed = 0;
    let mut skipped = 0;
    for (source, parser_version) in roots {
        let (new, current) =
            process_changed_sessions(source.as_ref(), parser_version, &project, &mut database)?;
        processed += new;
        skipped += current;
    }
    let candidates = database.load_candidates()?;
    println!("Analysis complete for {}", project.name);
    println!("  Repository files: {}", facts.tracked_files.len());
    println!("  Processed sessions: {processed}");
    println!("  Unchanged sessions: {skipped}");
    println!("  Candidate rules: {}", candidates.len());
    Ok(())
}

fn analysis_roots(arguments: &AnalyzeArgs) -> Vec<(Box<dyn SessionSource>, u32)> {
    let mut roots: Vec<(Box<dyn SessionSource>, u32)> = Vec::new();
    let claude = arguments.claude_root.clone().or_else(default_claude_root);
    if let Some(root) = claude.filter(|root| root.is_dir()) {
        roots.push((Box::new(ClaudeSessionSource::new(root)), 1));
    }
    let codex = arguments.codex_root.clone().or_else(default_codex_root);
    if let Some(root) = codex.filter(|root| root.is_dir()) {
        roots.push((Box::new(CodexSessionSource::new(root)), 1));
    }
    roots
}

fn process_changed_sessions(
    source: &dyn SessionSource,
    parser_version: u32,
    project: &Project,
    database: &mut Database,
) -> Result<(usize, usize)> {
    let mut processed = 0;
    let mut skipped = 0;
    for reference in source.discover()? {
        let fingerprint = SourceFingerprint::from_path(&reference.path, parser_version)?;
        if database.is_source_current(&fingerprint)? {
            skipped += 1;
            continue;
        }
        let session = source.parse(&reference)?;
        let candidates = if session
            .project
            .as_deref()
            .is_none_or(|path| paths_match_project(path, project))
        {
            find_corrections(std::slice::from_ref(&session))
        } else {
            Vec::new()
        };
        database.replace_source_candidates(&fingerprint, &candidates)?;
        processed += 1;
    }
    Ok((processed, skipped))
}

fn review(arguments: &ReviewArgs) -> Result<()> {
    let project = Project::detect(&arguments.project).context("could not detect project")?;
    let database = Database::open(&database_path(&project, arguments.database.as_deref()))?;
    let candidates = database.load_candidates()?;
    let action = arguments
        .accept
        .as_ref()
        .map(|id| (id, DecisionStatus::Accepted))
        .or_else(|| {
            arguments
                .reject
                .as_ref()
                .map(|id| (id, DecisionStatus::Rejected))
        });

    if let Some((id, status)) = action {
        let candidate = candidates
            .iter()
            .find(|candidate| candidate_id(&candidate.canonical_text) == *id)
            .with_context(|| format!("candidate {id} was not found"))?;
        let scope = match arguments.scope {
            ScopeArg::Global => agentctx::domain::RuleScope::Global,
            ScopeArg::Project => agentctx::domain::RuleScope::Project(project.root.clone()),
            ScopeArg::Session => agentctx::domain::RuleScope::SessionOnly,
        };
        let decision = ReviewDecision {
            status,
            edited_text: arguments.text.clone(),
            scope,
            visibility: if arguments.personal {
                agentctx::domain::Visibility::Personal
            } else {
                agentctx::domain::Visibility::Shared
            },
        };
        database.record_decision(&candidate.canonical_text, &decision)?;
        println!("Recorded decision for {id}");
        return Ok(());
    }

    for candidate in candidates {
        let id = candidate_id(&candidate.canonical_text);
        let status = database
            .decision(&candidate.canonical_text)?
            .map_or("pending".to_owned(), |decision| {
                format!("{:?}", decision.status).to_lowercase()
            });
        println!(
            "{id} [{status}] {}× {}",
            candidate.occurrences, candidate.canonical_text
        );
    }
    Ok(())
}

fn explain(arguments: &ExplainArgs) -> Result<()> {
    let project = Project::detect(&arguments.project).context("could not detect project")?;
    let database = Database::open(&database_path(&project, arguments.database.as_deref()))?;
    let candidate = database
        .load_candidates()?
        .into_iter()
        .find(|candidate| candidate_id(&candidate.canonical_text) == arguments.id)
        .with_context(|| format!("candidate {} was not found", arguments.id))?;
    println!("{}", candidate.canonical_text);
    println!("Occurrences: {}", candidate.occurrences);
    if let Some(decision) = database.decision(&candidate.canonical_text)? {
        println!("Decision: {:?}", decision.status);
        println!("Scope: {:?}", decision.scope);
        println!("Visibility: {:?}", decision.visibility);
    } else {
        println!("Decision: pending");
    }
    println!("Evidence:");
    for evidence in candidate.evidence {
        println!(
            "  - session {}: {}",
            evidence.session_id, evidence.user_text
        );
        if let Some(agent_text) = evidence.preceding_agent_text {
            println!("    preceding agent output: {agent_text}");
        }
    }
    Ok(())
}

fn database_path(project: &Project, override_path: Option<&Path>) -> PathBuf {
    override_path.map_or_else(
        || project.root.join(".agentctx/state.db"),
        Path::to_path_buf,
    )
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
