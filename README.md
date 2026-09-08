# awesome-harness

> La version structurée du projet se trouve dans [`docs/`](docs/README.md), avec la [roadmap](docs/ROADMAP.md) et la [todolist](docs/TODO.md).

Agent Context Compiler
1. Vision

Créer un outil local-first capable d'apprendre automatiquement :

comment un développeur aime travailler avec les coding agents ;
comment un projet spécifique fonctionne ;
quelles erreurs Claude Code / Codex répètent ;
quelles instructions méritent réellement d'être persistées ;
à quel niveau ces instructions doivent s'appliquer.

Puis compiler cette connaissance vers les formats natifs des différents agents :

~/.claude/CLAUDE.md
~/.codex/AGENTS.md

project/CLAUDE.md
project/AGENTS.md

project/.claude/rules/*.md
project/subdirectory/AGENTS.md

Le produit ne doit pas être pensé comme un AGENTS.md generator.

Il doit être pensé comme un :

context compiler / optimizer for coding agents

Analogie :

ESLint       → qualité du code
TypeScript   → cohérence des types
Prettier     → formatage
AgentContext → qualité du contexte donné aux agents
2. Problème

Aujourd'hui, les développeurs construisent leurs AGENTS.md et CLAUDE.md manuellement.

Cela pose plusieurs problèmes :

ils oublient d'y ajouter les corrections récurrentes ;
ils ajoutent trop d'informations inutiles ;
des règles deviennent obsolètes ;
des règles globales se retrouvent dans un projet ;
des règles spécifiques à un dossier polluent le contexte global ;
différentes instructions se contredisent ;
Claude et Codex utilisent des mécanismes différents ;
les mêmes erreurs sont corrigées encore et encore dans les conversations.

Claude Code propose déjà plusieurs scopes pour ses instructions, notamment ~/.claude/CLAUDE.md, ./CLAUDE.md, CLAUDE.local.md et .claude/rules/. Anthropic recommande également des instructions spécifiques et concises.

Codex utilise quant à lui AGENTS.md, avec une hiérarchie où les fichiers plus profonds peuvent spécialiser les instructions du niveau supérieur.

Le rôle d'Agent Context Compiler est donc de décider :

QUOI retenir
POURQUOI le retenir
OÙ le mettre
COMMENT l'écrire
QUAND le supprimer
3. Proposition de valeur

Entrée :

repository
+
git history
+
existing instructions
+
Claude Code sessions
+
Codex sessions
+
developer preferences

Transformation :

discover
    ↓
normalize
    ↓
extract evidence
    ↓
infer rules
    ↓
deduplicate
    ↓
resolve conflicts
    ↓
scope
    ↓
rank
    ↓
compile

Sortie :

optimal persistent context
4. Principe fondamental

Le système doit être :

Deterministic first

Tout ce qui peut être déterminé sans LLM doit l'être sans LLM.

Exemples :

package.json → pnpm
Cargo.toml → Rust
vitest.config.ts → Vitest
.github/workflows → commandes CI
tsconfig.json → TypeScript config
.gitignore → generated dirs
existing AGENTS.md → règles actuelles

Pas besoin d'envoyer ça à un modèle pour demander :

Quel package manager utilise le projet ?

Le LLM intervient pour les problèmes sémantiques :

"pour la troisième fois n'utilise pas repository.findAll ici,
on passe toujours par le QueryBus"

↓

rule:
  text: "Use the QueryBus instead of directly querying repositories."
  type: architecture
  scope: project
  confidence: 0.96
5. Sources analysées
5.1 Repository

Scanner notamment :

package.json
pnpm-lock.yaml
yarn.lock
package-lock.json

Cargo.toml
Cargo.lock

composer.json

pyproject.toml
requirements.txt

go.mod

Dockerfile
docker-compose.yml

Makefile
justfile

README.md

.github/workflows/

eslint.config.*
tsconfig.json
biome.json
prettier.config.*

vitest.config.*
jest.config.*
playwright.config.*

existing:
AGENTS.md
CLAUDE.md
.claude/**
6. Sessions Claude Code

Claude Code conserve officiellement ses transcriptions locales en JSONL dans :

~/.claude/projects/<project>/<session-id>.jsonl

Chaque ligne peut représenter un message, un tool call ou des métadonnées. À noter : Claude Code supprime ces transcriptions après 30 jours par défaut, sauf changement de cleanupPeriodDays.

Donc :

trait SessionSource {
    fn discover(&self) -> Result<Vec<SessionRef>>;
    fn parse(&self, session: &SessionRef) -> Result<NormalizedSession>;
}

avec :

ClaudeSessionSource
CodexSessionSource
...
7. Sessions Codex

Codex possède des rollouts JSONL dans son dossier sessions.

Son code actuel utilise notamment des structures du type :

$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl

avec les métadonnées de session, messages et événements.

Il existe également :

~/.codex/history.jsonl

pour l'historique global des messages utilisateur.

Je privilégierais :

rollout JSONL

car beaucoup plus riche.

8. Important : ne pas dépendre du format JSONL interne

Créer immédiatement une couche d'adaptation.

Jamais :

fn analyze_codex_json(json: Value)

dans le cœur métier.

Mais :

Claude JSONL ─┐
              │
Codex JSONL ──┼──▶ NormalizedSession
              │
Future Agent ─┘

Avec par exemple :

struct NormalizedSession {
    id: SessionId,
    source: AgentSource,
    project: Option<ProjectId>,
    started_at: DateTime<Utc>,
    messages: Vec<Message>,
    events: Vec<Event>,
}

Et :

struct Message {
    role: Role,
    content: String,
    timestamp: Option<DateTime<Utc>>,
}

Ça permet de survivre aux changements de Claude/Codex.

9. Types de connaissances à extraire

Je séparerais au minimum :

enum KnowledgeKind {
    UserPreference,
    CodingConvention,
    ArchitectureRule,
    Workflow,
    Command,
    ProjectFact,
    DirectoryRule,
    ToolPreference,
    Prohibition,
    DebuggingKnowledge,
    TemporaryInstruction,
}

Exemples.

User preference
Don't ask for confirmation before reversible edits.
Coding convention
Prefer named exports.
Architecture
Controllers never access repositories directly.
Workflow
Run pnpm typecheck before considering a task complete.
Command
Use pnpm test:unit to execute unit tests.
Project fact
Authentication is implemented using NextAuth.
Prohibition
Never modify src/generated.
10. Evidence

Chaque règle doit être traçable.

C'est extrêmement important.

Ne jamais avoir seulement :

rule:
  text: "Always use pnpm"

Mais :

rule:
  id: rule_0192
  text: "Use pnpm instead of npm."

  evidence:
    - type: explicit_user_instruction
      session: abc
      message: 58

    - type: correction
      session: def
      message: 21

    - type: repository
      path: package.json
      field: packageManager

  occurrences: 12
  confidence: 0.99

Ainsi :

agentctx explain rule_0192

peut répondre :

Use pnpm instead of npm.

Confidence: 99%

Evidence:
  package.json declares pnpm
  explicitly requested in 7 sessions
  corrected npm usage in 5 sessions

C'est une grosse différence produit par rapport à un simple générateur LLM.

11. Détection des corrections

C'est probablement le signal le plus intéressant.

Exemple :

Agent:
I'll install it with npm.

User:
Non utilise pnpm sur ce projet.

Tu détectes :

agent behavior
      ↓
user correction
      ↓
candidate rule

Autres patterns :

"non, ..."
"je t'ai déjà dit..."
"toujours..."
"jamais..."
"sur ce projet..."
"à l'avenir..."
"utilise X plutôt que Y"
"pas besoin de..."

Mais surtout : ne pas faire uniquement du regex.

Les regex détectent les candidats.

Le LLM détermine leur signification.

12. Le Scope Engine

C'est selon moi la fonctionnalité centrale.

Une règle candidate doit recevoir un scope.

enum RuleScope {
    Global,
    Project(ProjectId),
    Directory(PathBuf),
    FilePattern(String),
    SessionOnly,
}

Exemple :

"Réponds-moi brièvement"

→ global.

"On utilise pnpm sur tous mes projets"

→ global.

"Sur ce projet on utilise pnpm"

→ project.

"Dans packages/api on n'utilise jamais Prisma directement"

→ directory.

"Pour cette migration, conserve l'ancien endpoint"

→ probablement temporaire.

13. Shared vs personal

Autre distinction indispensable.

enum Visibility {
    Shared,
    Personal,
}

Exemple :

Architecture du projet

→ shared.

Je préfère que Claude soit très concis

→ personal.

Donc Claude pourrait recevoir :

./CLAUDE.md

pour la connaissance partagée.

Et :

./CLAUDE.local.md

pour les préférences personnelles spécifiques au projet.

Claude Code supporte justement CLAUDE.local.md pour ce cas.

14. Intermediate Representation

Le markdown ne doit surtout pas être ta base de données.

Créer ton propre IR.

Exemple :

struct Rule {
    id: RuleId,

    canonical_text: String,

    kind: KnowledgeKind,

    scope: RuleScope,
    visibility: Visibility,

    confidence: f32,
    importance: f32,

    first_seen_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,

    occurrences: u32,

    evidence: Vec<Evidence>,

    status: RuleStatus,
}

Puis :

enum RuleStatus {
    Candidate,
    Accepted,
    Rejected,
    Superseded,
    Stale,
}
15. Compilation

À partir de l'IR :

Knowledge IR
      ↓
 ┌────┴─────────┐
 │              │
Claude       Codex
 │              │
 ▼              ▼
CLAUDE.md     AGENTS.md
rules/*.md    nested AGENTS.md

Le renderer connaît les particularités de chaque agent.

trait Renderer {
    fn render(&self, context: &CompiledContext) -> Result<Vec<Artifact>>;
}

Implémentations :

ClaudeRenderer
CodexRenderer

Plus tard :

CursorRenderer
CopilotRenderer
GeminiRenderer
WindsurfRenderer
16. Ne pas forcément générer le même contenu pour Claude et Codex

Très important.

Tu pourrais avoir une règle interne :

frontend files require pnpm lint

Pour Claude :

.claude/rules/frontend.md

avec scope approprié.

Pour Codex :

frontend/AGENTS.md

Les deux contiennent la même connaissance logique, mais leur organisation optimale diffère.

Claude prend désormais en charge .claude/rules/ et les règles path-scoped.

Codex, lui, applique naturellement les AGENTS.md selon leur position dans l'arborescence.

17. Context budget

L'objectif n'est PAS :

maximum information

L'objectif est :

maximum useful information / token

Une règle pourrait avoir :

struct RuleScore {
    usefulness: f32,
    confidence: f32,
    recurrence: f32,
    severity: f32,
    recency: f32,
    token_cost: usize,
}

Puis une fonction du genre :

score =
  confidence
  × usefulness
  × recurrence
  × severity
  × recency_decay
  ÷ token_cost_penalty

Le calcul réel sera plus subtil, mais l'idée est importante.

18. Exemple

On trouve :

Rule A
Use pnpm.

tokens: 4
confidence: .99
occurrences: 23

Très forte valeur.

Rule B
The application was initially built by three developers in 2022
and later migrated through multiple versions...

tokens : 80.

Mais Claude peut le retrouver dans le repo et ça n'influence quasiment aucune tâche.

→ supprimer.

19. Détection des contradictions

Exemple :

Rule #17
Always use yarn.

Rule #94
Always use pnpm.

Le système doit détecter :

CONFLICT

et chercher à l'expliquer :

yarn:
last evidence 2024

pnpm:
packageManager = pnpm@10
last evidence yesterday

Résultat :

Rule #17 → stale
Rule #94 → active
20. Obsolescence

Toutes les règles devraient posséder :

created_at
last_confirmed_at
last_used_at

Et éventuellement :

valid_until

On peut produire :

agentctx doctor
Potentially stale rules

⚠ "Use Jest"
  Last evidence: 9 months ago
  Repository now contains Vitest configuration.

→ Replace with "Use Vitest"?
21. Architecture proposée

Je choisirais :

                ┌──────────────┐
                │     CLI      │
                └──────┬───────┘
                       │
        ┌──────────────▼──────────────┐
        │         Application         │
        └──────────────┬──────────────┘
                       │
             ┌─────────▼─────────┐
             │       Core        │
             │ rules / evidence  │
             │ scope / scoring   │
             └─────────┬─────────┘
                       │
      ┌────────────────┼──────────────────┐
      ▼                ▼                  ▼
   Ingest           Analyzer           Render
      │                │                  │
 Claude             Repo             Claude
 Codex              Sessions         Codex
 future             LLM              future
      │
      ▼
   Storage
   SQLite
22. Rust : oui
Mon choix

Rust pour tout le core + CLI.

Pourquoi :

Distribution
brew install agentctx

ou :

curl ... | sh

et c'est fini.

Pas :

install node
npm install
npx ...
Performances

Tu vas potentiellement traiter :

50 000 fichiers
5 000 sessions
plusieurs GB de JSONL

Rust est parfait pour du streaming et du traitement filesystem.

Fiabilité

Ton application va :

scanner
parser
modifier des fichiers
fusionner des règles
manipuler git

Les types Rust apportent réellement quelque chose ici.

Cohérence avec le produit

Codex CLI lui-même est aujourd'hui très majoritairement écrit en Rust.

Ça ne signifie évidemment pas qu'il faut utiliser Rust parce qu'OpenAI le fait, mais le type de problème est très similaire :

CLI
filesystem
JSONL
process spawning
configuration
streaming
cross-platform
23. Stack Rust recommandée
CLI
clap

Avec derive :

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

clap reste une solution très mature et sa version actuelle est dans la branche 4.6.

Serialization
serde
serde_json
toml
serde_yaml_ng éventuellement

Pour :

JSONL
config.toml
IR
JSON Schema
Errors
thiserror
anyhow

Convention :

thiserror → erreurs métier / bibliothèque
anyhow    → couche application/CLI
Logs
tracing
tracing-subscriber

Puis :

agentctx --verbose
agentctx --debug
24. SQLite

Je recommande fortement :

SQLite
+
rusqlite

rusqlite reste le wrapper SQLite Rust direct et ergonomique.

Pas PostgreSQL.

Pas besoin de serveur.

Tout reste local.

Exemple :

~/.local/share/agentctx/agentctx.db

ou chemins natifs selon l'OS.

25. Tables

Je commencerais avec :

projects
sessions
messages
rules
evidence
rule_evidence
analysis_runs
artifacts

Par exemple :

rules
-----
id
canonical_text
kind
scope_type
scope_value
visibility
confidence
importance
status
first_seen_at
last_seen_at
26. Ne stocke pas forcément tous les messages en SQLite

Important.

Les JSONL originaux restent la source.

SQLite peut conserver :

session metadata
hash
path
mtime
analysis status
important extracted evidence

Cela permet :

incremental indexing

Sans recopier des gigas de discussions.

27. Indexation incrémentale

À chaque passage :

path
size
mtime
hash

Si rien n'a changé :

skip

Donc :

agentctx update

peut analyser uniquement :

12 nouvelles sessions
34 nouveaux commits
3 fichiers config modifiés

au lieu de tout recalculer.

28. File walking

Je prendrais :

ignore

ou walkdir.

Préférence à ignore car il comprend naturellement les logiques .gitignore.

Il faut absolument éviter :

node_modules
target
vendor
.next
dist
.git
build
generated artifacts
29. Git

Pour le MVP :

je n'utiliserais pas git2.

Je lancerais :

git rev-parse
git log
git status
git diff
git ls-files

via :

std::process::Command

Pourquoi ?

Parce que :

Git sera pratiquement toujours installé ;
moins de dépendances natives ;
comportement identique au Git du développeur ;
beaucoup plus simple.

Plus tard, éventuellement :

gix

si tu as besoin d'une intégration profonde.

30. Tree-sitter

Oui, mais pas au MVP.

Tree-sitter dispose de bindings Rust actifs.

À terme il serait génial pour détecter :

patterns architecture
imports
module boundaries
function structure
naming conventions

Mais pour V1 :

manifests
configs
folder layout
git
sessions

suffisent largement.

Ne transforme pas le MVP en analyseur statique universel.

31. Couche LLM

Créer une abstraction.

#[async_trait]
trait InferenceProvider {
    async fn infer<T>(
        &self,
        request: InferenceRequest
    ) -> Result<T>;
}

Providers :

ClaudeCliProvider
CodexCliProvider

OpenAIApiProvider
AnthropicApiProvider

Future:
OllamaProvider
OpenRouterProvider
32. Première version : utiliser les CLI existants

C'est une solution particulièrement intéressante.

Si la personne possède Claude Code :

claude -p ...

Claude Code supporte officiellement l'exécution programmatique ainsi que des sorties contraintes par JSON Schema.

Donc AgentContext peut appeler :

claude -p
--output-format json
--json-schema ...

Même principe avec Codex :

codex exec

Codex supporte également --output-schema.

Tu peux donc proposer :

[llm]
provider = "claude-cli"

ou :

[llm]
provider = "codex-cli"
33. Puis API directement

Pour les utilisateurs voulant :

API key
CI
headless server

ajouter :

[llm]
provider = "openai"
model = "..."

ou :

provider = "anthropic"

Je coderais les appels HTTP avec :

reqwest

derrière ton interface.

Je ne laisserais jamais la logique métier dépendre directement d'un SDK OpenAI/Anthropic.

34. JSON Schema partout

Un LLM ne doit jamais retourner :

Voilà les règles que j'ai trouvées :
1. ...

Il doit retourner :

{
  "rules": [...]
}

validé.

Exemple :

{
  "rules": [
    {
      "text": "Use pnpm.",
      "kind": "tool_preference",
      "scope": "project",
      "confidence": 0.98,
      "evidence_message_ids": ["m17", "m52"]
    }
  ]
}
35. Batching

Surtout ne fais pas :

1 session
→
1 LLM request

Tu vas exploser coût et latence.

Pipeline :

deterministic filtering
       ↓
extract likely interesting segments
       ↓
batch 20-100 segments
       ↓
LLM classification

Un historique de 2 000 sessions pourrait n'envoyer que :

300-500 segments intéressants

au modèle.

36. Phase de candidate detection sans LLM

Exemples de signaux :

always
never
prefer
don't
instead
from now on

toujours
jamais
préfère
utilise
n'utilise pas
à l'avenir

package manager mismatch

command fails
→ user provides corrected command

Ainsi le LLM ne reçoit que le matériel pertinent.

37. Privacy

Très gros sujet.

Les transcripts peuvent contenir :

tokens
passwords
.env
customer data
URLs privées
code propriétaire

Par défaut :

LOCAL ONLY

Et avant tout appel distant :

secret redaction

Patterns :

API keys
Bearer tokens
JWT
private keys
AWS credentials
GitHub tokens
.env assignments
high entropy strings

Option :

[privacy]
allow_remote_inference = false

L'utilisateur doit explicitement activer l'envoi distant.

38. Configuration

Je proposerais :

~/.config/agentctx/config.toml

Exemple :

[general]
language = "auto"

[analysis]
min_occurrences = 2
max_context_tokens = 2500

[llm]
provider = "claude-cli"

[privacy]
allow_remote_inference = false
redact_secrets = true

[targets]
claude = true
codex = true

Et projet :

.agentctx.toml
39. CLI

Je partirais sur un exécutable extrêmement simple :

agentctx

ou, nom plus court :

ctx

mais ctx est probablement trop générique.

40. Commandes V1
init
agentctx init

Résultat :

Project detected: my-app

✓ Next.js
✓ TypeScript
✓ pnpm
✓ Vitest
✓ Playwright

Claude Code sessions: 183
Codex sessions: 74

Existing:
✓ CLAUDE.md
✗ AGENTS.md

Run `agentctx analyze` to continue.
analyze
agentctx analyze
Scanning...

Repository
  487 tracked files
  18 relevant configuration files

History
  183 Claude sessions
  74 Codex sessions
  2,841 user messages

Potential signals
  86 corrections
  34 repeated instructions
  17 workflow patterns

Extracted
  41 candidate rules
41. review

C'est essentiel.

agentctx review

TUI :

┌ Use pnpm instead of npm ─────────────────────────┐

Scope       PROJECT
Confidence  99%
Occurrences 12

Evidence:
✓ package.json
✓ 8 explicit instructions
✓ 4 corrections

[A] Accept
[R] Reject
[E] Edit
[G] Global
[P] Project
[D] Directory

└──────────────────────────────────────────────────┘

Je n'écrirais pas automatiquement 30 règles dans AGENTS.md sans validation au début du produit.

42. apply
agentctx apply
Changes:

~/.claude/CLAUDE.md
  +2 rules

~/.codex/AGENTS.md
  +2 rules

./CLAUDE.md
  +8 rules
  -2 stale rules

./AGENTS.md
  +7 rules

Write changes? [Y/n]
43. doctor

Probablement une des meilleures commandes.

agentctx doctor
Context health: 82/100

2 contradictions
3 stale instructions
4 duplicated rules
1 invalid command
613 unnecessary tokens

Potential saving:
32% context size
44. explain
agentctx explain <rule>

Permet d'avoir la provenance exacte.

Très utile pour rendre le système digne de confiance.

45. diff
agentctx diff

affiche :

 ## Testing

- Run npm test before completing work.
+ Run pnpm test before completing work.

+ Run pnpm typecheck after TypeScript changes.
46. Global mode

Commande :

agentctx analyze --global

Analyse :

all Claude projects
all Codex projects

Et cherche uniquement ce qui semble transversal.

Exemple :

27 projects use pnpm
23 explicit pnpm requests

↓

GLOBAL CANDIDATE
Prefer pnpm when the repository supports it.

Mais :

Use Symfony Messenger for async workloads.

vu seulement dans un projet Symfony :

→ pas global.

47. Multi-project inference

Une règle devient plus probablement globale si elle apparaît :

plusieurs projets
+
plusieurs périodes
+
contextes différents

Tu peux modéliser :

global_score =
    project_diversity
  × recurrence
  × explicitness

C'est une très belle fonctionnalité différenciante.

48. Structure Rust

Pour commencer, je ne ferais pas 15 crates.

Je ferais :

agentctx/
├── Cargo.toml
├── src/
│
├── cli/
│   ├── mod.rs
│   ├── init.rs
│   ├── analyze.rs
│   ├── review.rs
│   ├── apply.rs
│   └── doctor.rs
│
├── domain/
│   ├── rule.rs
│   ├── evidence.rs
│   ├── project.rs
│   └── session.rs
│
├── ingest/
│   ├── claude.rs
│   ├── codex.rs
│   └── repository.rs
│
├── analysis/
│   ├── candidates.rs
│   ├── inference.rs
│   ├── scope.rs
│   ├── conflicts.rs
│   └── scoring.rs
│
├── llm/
│   ├── mod.rs
│   ├── claude_cli.rs
│   ├── codex_cli.rs
│   └── api.rs
│
├── render/
│   ├── claude.rs
│   └── codex.rs
│
├── storage/
│   ├── sqlite.rs
│   └── migrations.rs
│
└── main.rs

Quand le projet grossit, extraire :

agentctx-core
agentctx-ingest
agentctx-cli
49. Tokio ?

Oui, mais pas partout.

Je prendrais :

tokio

principalement pour :

LLM calls
subprocess streams
future networking
parallel analysis

Mais le cœur métier :

fn score_rule(...)
fn merge_rules(...)
fn classify_scope(...)

reste synchrone et pur.

50. Rayon ?

Éventuellement.

Pour analyser de gros volumes :

rayon

peut paralléliser :

session parsing
hashing
repository analysis

Mais pas nécessaire au jour 1.

51. TUI

Après le MVP :

ratatui

Ça correspond très bien à agentctx review.

Interface :

Candidates       Evidence               Preview
──────────       ────────               ───────
> Use pnpm       session #81             ## Tooling
  Named exports  session #47             - Use pnpm.
  Tests...       package.json

Mais je commencerais par un prompt interactif simple.

52. Pas de GUI au départ

Surtout pas Electron.

CLI d'abord.

Puis si le produit fonctionne :

Tauri

serait logique car tu pourrais réutiliser ton backend Rust.

Architecture :

agentctx-core
     ↓
 ┌───┴────┐
 CLI    Tauri
53. Evaluation

C'est ce qui pourrait transformer ce projet en excellent produit.

Tu dois être capable de répondre à :

Est-ce que le nouveau AGENTS.md améliore réellement le coding agent ?

54. Benchmark historique

Une session possède :

task
↓
agent actions
↓
error
↓
user correction
↓
solution

Tu construis un testcase :

Task:
Implement X.

Expected constraints:
- use pnpm
- don't edit generated/
- QueryBus only

Puis comparer :

without context
vs
current context
vs
optimized context
55. Métriques
task success rate
user corrections
incorrect commands
architecture violations
test failures
tokens consumed
agent turns
time to completion
56. Context score

Tu pourrais produire :

Agent Context Score
────────────────────────

Specificity       94
Freshness         87
Consistency       100
Token efficiency   82
Evidence quality   91
Coverage           78

Overall            89/100

Ça devient très vendable.

57. agentctx optimize

À terme :

agentctx optimize

Effectue :

remove redundant rules
shorten verbose rules
move directory-specific rules
remove stale knowledge
merge duplicates
resolve conflicts

Avant :

4,813 tokens

Après :

2,160 tokens

avec :

estimated coverage:
94% → 96%
58. Éviter un piège

Ne présente jamais un score LLM arbitraire comme une vérité.

Par exemple :

Estimated AGENTS quality: 96%

sans benchmark réel = gadget.

Les métriques doivent venir autant que possible de données :

number of corrections
rule recurrence
repo evidence
historical task performance
conflicts
tokens
59. MVP

Je ferais un MVP beaucoup plus petit que la vision finale.

V0.1

Seulement :

Claude Code
Codex
Git repo

Sources :

sessions
existing CLAUDE.md
existing AGENTS.md
basic project configs

Fonctions :

agentctx init
agentctx analyze
agentctx review
agentctx apply
Analyse V0.1

Détecter seulement :

explicit user corrections
repeated instructions
commands
tool preferences
prohibitions

Pas encore :

deep architecture inference
tree-sitter
automatic evals
GUI
GitHub integration
team mode
60. V0.2

Ajouter :

scope inference
global analysis
conflict detection
staleness
agentctx doctor
61. V0.3

Ajouter :

.claude/rules generation
nested AGENTS.md
directory scopes
TUI review
62. V0.4

Ajouter :

historical evals
context optimization
rule usefulness metrics
63. V1

À ce stade :

Claude Code
Codex
Cursor
Copilot
Gemini

éventuellement.

Mais le produit n'est plus :

AGENTS.md generator

Il devient :

universal coding-agent context manager
64. Modèle économique potentiel

Même si tu commences open source :

Free
local CLI
repo analysis
Claude/Codex import
AGENTS/CLAUDE generation
Pro

Potentiellement :

advanced analytics
historical evals
multi-machine sync
context performance
automatic continuous learning
Team
shared rules
organization policies
team analytics
PR context checks
CI integration

Mais je garderais la base open source + local-first.

C'est important pour gagner la confiance nécessaire lorsqu'on lit l'intégralité des sessions de développement.

65. CI future

Imagine :

- run: agentctx check

Et une PR échoue avec :

AGENTS.md appears stale.

Detected:
package.json now uses Vitest

Current AGENTS.md:
"Use Jest for unit tests."

Run:
agentctx update

Là ton produit devient un véritable outil d'ingénierie.

66. GitHub bot

Plus tard :

PR changes:
package manager npm → pnpm

Bot :

⚠ Project instructions still reference npm in 3 places.

Même logique que Dependabot / linting.

67. Différenciation essentielle

Je construirais le produit autour de ces quatre mots :

Evidence

Chaque règle doit avoir une raison d'exister.

Scope

Chaque règle doit être placée au niveau minimal pertinent.

Budget

Chaque token de contexte doit justifier son coût.

Evolution

Les règles doivent pouvoir apparaître, évoluer et disparaître.

68. Ce que je ne ferais PAS

Pas de :

agentctx generate
→ donne tout le repo à GPT
→ "write the perfect AGENTS.md"

Ça serait facile à copier et peu intéressant.

Pas de :

500 lignes d'architecture automatique

Le code est déjà là pour ça.

Le fichier doit principalement contenir ce que l'agent ne peut pas raisonnablement déduire tout seul.

OpenAI recommande d'ailleurs d'utiliser AGENTS.md pour les conventions, comportements, contraintes et particularités durables du repo.

69. Question centrale du moteur

Pour chaque information :

Can the coding agent easily infer this itself?

Si oui :

don't persist

Exemple :

src/components contains React components

→ inutile.

Mais :

Don't import components from another feature directly;
use the public feature index.

→ précieux.

70. Stack finale recommandée
Language
└── Rust

CLI
└── clap

Async
└── tokio

Serialization
├── serde
├── serde_json
└── toml

Database
├── SQLite
└── rusqlite

Errors
├── anyhow
└── thiserror

Logging
├── tracing
└── tracing-subscriber

Filesystem
├── ignore
└── std::fs

HTTP
└── reqwest

Dates
└── chrono

Hash
└── blake3

Terminal
├── indicatif
├── dialoguer
└── ratatui plus tard

Code parsing V2
└── tree-sitter

Git V1
└── system git

LLMs
├── Claude Code CLI
├── Codex CLI
├── Anthropic API
└── OpenAI API

Storage
└── local-first SQLite
71. Architecture LLM que je choisirais
                       ┌──────────────┐
                       │ Repository   │
                       └──────┬───────┘
                              │
Claude JSONL ──────┐           │
                   │           │
Codex JSONL ───────┼─────┐     │
                   │     │     │
future agents ─────┘     ▼     ▼
                   ┌──────────────────┐
                   │ Normalization    │
                   └────────┬─────────┘
                            ▼
                   ┌──────────────────┐
                   │ Candidate Finder │
                   │ deterministic    │
                   └────────┬─────────┘
                            ▼
                   ┌──────────────────┐
                   │ Semantic Extract │
                   │       LLM        │
                   └────────┬─────────┘
                            ▼
                   ┌──────────────────┐
                   │ Knowledge Graph  │
                   │ SQLite / IR      │
                   └────────┬─────────┘
                            ▼
          ┌─────────────────┼─────────────────┐
          ▼                 ▼                 ▼
       scope()           score()          conflict()
          └─────────────────┬─────────────────┘
                            ▼
                   ┌──────────────────┐
                   │ Context Compiler │
                   └────────┬─────────┘
                            │
                 ┌──────────┴──────────┐
                 ▼                     ▼
             CLAUDE.md             AGENTS.md
72. Mon choix pour lancer réellement le projet

Je construirais :

Rust CLI
+
SQLite
+
Claude/Codex JSONL ingestion
+
Claude CLI ou Codex CLI pour l'inférence
+
IR propriétaire
+
interactive review

Et rien de plus au premier prototype.

Le premier objectif doit être :

Je lance agentctx analyze dans un repo sur lequel j'ai travaillé pendant trois mois et il me ressort 10 règles extrêmement pertinentes auxquelles je n'avais pas pensé.

Si tu obtiens ça, tu tiens quelque chose.

Si le résultat est juste :

- Use TypeScript
- Run tests
- Follow coding conventions
- Keep code clean

le produit ne vaut rien.

73. Killer feature

Je pense même que la feature qui doit être développée avant le générateur final est :

agentctx mistakes

Exemple :

Repeated agent mistakes
────────────────────────────────────────────────────

12× Used npm instead of pnpm
  → suggested global/project rule

7× Edited generated files
  → suggested project rule

5× Accessed repository directly instead of QueryBus
  → suggested src/backend scoped rule

4× Asked for confirmation although task was reversible
  → suggested personal global preference

C'est immédiatement compréhensible.

Et ça justifie naturellement la création des règles.

74. Positionnement

Je ne l'appellerais pas :

AI AGENTS.md Generator

Je dirais :

AgentContext learns from your coding-agent history and compiles the smallest set of instructions needed to stop agents from making the same mistakes twice.

Ou :

Your coding agents forget. AgentContext doesn't.

Ou, pour les développeurs :

A compiler for coding-agent context.

75. Résumé du produit

La boucle finale serait :

                    developer
                        │
                        ▼
                 coding agents
                Claude / Codex
                        │
                        ▼
                    sessions
                        │
                        ▼
                 ┌────────────┐
                 │ AgentContext│
                 └─────┬──────┘
                       │
          learns from corrections
                       │
                       ▼
                  Knowledge IR
                       │
                       ▼
             optimal instructions
                       │
          ┌────────────┴────────────┐
          ▼                         ▼
      CLAUDE.md                 AGENTS.md
          │                         │
          └────────────┬────────────┘
                       ▼
                better sessions
                       │
                       └───────────↺

C'est cette boucle d'amélioration continue qui, à mon sens, constitue le vrai produit.
