# Modèle de domaine

## Agrégat principal : Rule

Une règle représente une connaissance persistable, indépendante du format de sortie.

```rust
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
```

Le Markdown rendu est un artefact compilé, jamais la source de vérité.

## Types de connaissance

| Type | Exemple |
|---|---|
| `UserPreference` | Ne pas demander confirmation pour une modification réversible. |
| `CodingConvention` | Préférer les exports nommés. |
| `ArchitectureRule` | Les contrôleurs passent par le QueryBus. |
| `Workflow` | Exécuter le typecheck après une modification TypeScript. |
| `Command` | Utiliser `pnpm test:unit` pour les tests unitaires. |
| `ProjectFact` | L'authentification repose sur NextAuth. |
| `DirectoryRule` | Ne pas utiliser Prisma directement dans `packages/api`. |
| `ToolPreference` | Utiliser pnpm plutôt que npm. |
| `Prohibition` | Ne jamais modifier `src/generated`. |
| `DebuggingKnowledge` | Régénérer le client avant ce test précis. |
| `TemporaryInstruction` | Conserver l'ancien endpoint pendant une migration. |

La V0.1 extrait prioritairement `Command`, `ToolPreference`, `Prohibition` et les corrections explicites pouvant produire les autres types.

## Scope

```rust
enum RuleScope {
    Global,
    Project(ProjectId),
    Directory(PathBuf),
    FilePattern(String),
    SessionOnly,
}
```

Le scope retenu est le plus étroit compatible avec les preuves. Une règle observée dans plusieurs projets, périodes et contextes gagne seulement alors en probabilité d'être globale.

## Visibilité

```rust
enum Visibility {
    Shared,
    Personal,
}
```

Une contrainte d'architecture appartient généralement au projet et peut être partagée. Une préférence de style de conversation appartient à l'utilisateur. Cette distinction détermine notamment si Claude reçoit la règle dans `CLAUDE.md` ou `CLAUDE.local.md`.

## Cycle de vie

```rust
enum RuleStatus {
    Candidate,
    Accepted,
    Rejected,
    Superseded,
    Stale,
}
```

Transitions principales :

```text
Candidate ── accepter ──▶ Accepted ── preuve contraire ──▶ Stale
    │                         │                              │
    └── rejeter ──▶ Rejected └── nouvelle règle ──▶ Superseded
```

Un rejet doit rester mémorisé afin d'éviter de proposer sans cesse le même candidat, sauf apparition de nouvelles preuves significatives.

## Evidence

Une preuve relie une règle à une observation vérifiable :

- instruction explicite, avec session et message ;
- correction d'un comportement d'agent ;
- fait du dépôt, avec chemin et champ ;
- changement Git daté ;
- règle existante et son emplacement.

Exemple conceptuel :

```yaml
rule:
  id: rule_0192
  text: Use pnpm instead of npm.
  occurrences: 12
  confidence: 0.99
  evidence:
    - type: repository
      path: package.json
      field: packageManager
    - type: correction
      session: def
      message: 21
```

`agentctx explain rule_0192` doit restituer cette provenance en langage clair.

## Score et budget

Le classement combine au minimum : confiance, utilité, récurrence, sévérité, récence et coût en tokens. La formule précise doit être calibrée sur des données ; elle ne constitue pas une mesure absolue de qualité.

Le compilateur sélectionne ensuite les règles sous une limite de contexte. Une règle brève, fréquente et fortement prouvée doit généralement battre un historique verbeux que l'agent peut redécouvrir dans le dépôt.

## Conflits et obsolescence

Deux règles incompatibles sont conservées comme connaissances distinctes tant que leur résolution n'est pas justifiée. La récence, l'état actuel du dépôt et les instructions explicites servent à proposer :

- la règle active ;
- la règle devenue `Stale` ;
- ou une décision humaine quand les preuves restent ambiguës.

Chaque règle acceptée suit `created_at`, `last_confirmed_at` et `last_used_at`, avec éventuellement `valid_until` pour les contraintes temporaires.
