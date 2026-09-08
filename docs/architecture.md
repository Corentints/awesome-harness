# Architecture

## Vue d'ensemble

```text
                         ┌──────────────┐
                         │     CLI      │
                         └──────┬───────┘
                                │
                    ┌───────────▼───────────┐
                    │      Application      │
                    └───────────┬───────────┘
                                │
              ┌─────────────────▼─────────────────┐
              │              Domaine              │
              │ règles · preuves · scopes · score │
              └─────────────────┬─────────────────┘
                                │
        ┌───────────────────────┼────────────────────────┐
        ▼                       ▼                        ▼
   Ingestion                 Analyse                  Rendu
 repo/sessions        candidats/LLM/conflits     Claude/Codex
        │                       │                        │
        └───────────────────────┴────────────┬───────────┘
                                             ▼
                                      SQLite local
```

Les dépendances pointent vers le domaine. Les adaptateurs JSONL, SQLite, Git, CLI et LLM ne doivent pas contaminer les types métier.

## Frontières

### Domaine

Le cœur contient des fonctions synchrones et testables pour :

- fusionner et dédupliquer des règles ;
- calculer leur score ;
- inférer ou valider leur scope ;
- détecter contradictions et obsolescence ;
- sélectionner un contexte sous contrainte de tokens.

Le domaine ignore la forme des JSONL, la base SQLite et les SDK fournisseurs.

### Ingestion

Chaque source implémente une interface d'adaptation et produit des objets normalisés. Un changement du format Claude ou Codex ne doit nécessiter qu'une modification de l'adaptateur correspondant.

```rust
trait SessionSource {
    fn discover(&self) -> Result<Vec<SessionRef>>;
    fn parse(&self, session: &SessionRef) -> Result<NormalizedSession>;
}
```

Le scanner du dépôt lit en priorité les fichiers suivis par Git, respecte les règles d'ignorance et collecte des faits déterministes depuis les manifests, configurations, workflows et instructions existantes.

### Analyse

Le pipeline commence par des filtres déterministes. Le LLM n'intervient que sur les segments ambigus ou sémantiques déjà réduits, et renvoie une réponse validée par JSON Schema. Les segments sont regroupés pour limiter coût et latence.

### Rendu

Une règle logique ne correspond pas forcément au même fichier chez tous les agents. Un renderer transforme un `CompiledContext` en artefacts adaptés à sa cible.

```rust
trait Renderer {
    fn render(&self, context: &CompiledContext) -> Result<Vec<Artifact>>;
}
```

La V0.1 cible `CLAUDE.md` et `AGENTS.md`. Les règles Claude ciblées par chemin et les `AGENTS.md` imbriqués arrivent en V0.3.

### Stockage

SQLite conserve l'IR, les preuves, les références de sessions et l'état d'indexation. Les transcriptions brutes restent dans leur emplacement d'origine ; la base stocke des références, extraits utiles, hashes et métadonnées plutôt qu'une copie exhaustive.

## Déterministe ou LLM

| Déterministe | Sémantique assistée par LLM |
|---|---|
| Détection du package manager | Sens d'une correction utilisateur |
| Présence d'un framework de test | Canonicalisation d'une règle |
| Commandes déclarées dans les manifests/CI | Classification d'une instruction ambiguë |
| Fichiers générés et ignorés | Proposition de scope quand le texte ne suffit pas |
| Candidats par marqueurs lexicaux | Comparaison de règles paraphrasées |
| Récence, occurrences, coût en tokens | Explication d'un conflit non trivial |

Une conclusion déterministe doit rester prioritaire et constituer une preuve explicite.

## Stack proposée

- Rust, avec un seul crate au départ ;
- `clap` pour la CLI ;
- `serde`, `serde_json` et `toml` pour les formats ;
- SQLite via `rusqlite` ;
- `thiserror` dans les bibliothèques et `anyhow` aux frontières CLI ;
- `tracing` pour les diagnostics ;
- `ignore` pour le parcours de fichiers ;
- Git système pour la V1 ;
- `tokio` limité aux processus, flux et appels réseau ;
- `blake3` pour l'indexation incrémentale.

`rayon`, `ratatui`, `tree-sitter`, `reqwest` et une séparation en plusieurs crates sont des optimisations ou extensions ultérieures, pas des prérequis du premier incrément.

## Structure cible initiale

```text
src/
├── cli/
├── domain/
├── ingest/
├── analysis/
├── llm/
├── render/
├── storage/
└── main.rs
```

## Contraintes architecturales

- aucun `serde_json::Value` provenant d'un agent dans le domaine ;
- aucun SDK LLM appelé directement depuis la logique métier ;
- aucune écriture de fichier d'instructions sans diff et validation en V0.1 ;
- aucune donnée distante tant que `allow_remote_inference` n'est pas explicitement activé ;
- aucune règle compilée sans provenance ;
- toute nouvelle source doit passer par le modèle normalisé.
