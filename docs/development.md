# Développement et installation

## Support actuel

Le prototype est validé sur macOS avec Rust 1.96. Linux doit rester compatible grâce aux bibliothèques multiplateformes utilisées, mais n'est considéré comme supporté qu'après validation continue. Windows n'est pas encore testé, notamment pour les remplacements atomiques de fichiers et les chemins de sessions.

## Prérequis

- une toolchain Rust stable récente ;
- Git disponible dans le `PATH` ;
- Codex CLI uniquement si `llm.provider = "codex-cli"` est activé.

## Installation locale

```bash
cargo install --path .
agentctx --help
```

Pour contribuer sans installation :

```bash
cargo run -- --help
```

## Validation

La même séquence est exécutée en CI :

```bash
cargo fmt --all -- --check
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
```

Les tests d'intégration couvrent les fixtures Claude/Codex, les migrations SQLite, le workflow CLI local, la redaction, les renderers et la préservation des instructions existantes.

## Ajouter une source de sessions

1. Implémenter `SessionSource` dans `src/ingest/`.
2. Convertir le format externe vers `NormalizedSession` sans exposer ses objets JSON au domaine.
3. Ajouter une fixture minimale anonymisée et des cas de champs inconnus.
4. Versionner le parseur afin de déclencher une réindexation lors d'un changement incompatible.

## Ajouter un provider

1. Implémenter `InferenceProvider` dans `src/llm/`.
2. Accepter uniquement `InferenceRequest` déjà minimisé et redacted.
3. Valider la réponse contre le contrat de `output_schema()`.
4. Ajouter un opt-in explicite si le provider peut communiquer avec un service distant.
5. Ne jamais appeler le provider depuis le domaine ou le stockage.

## Ajouter une cible

1. Implémenter `Renderer` dans `src/render.rs` ou un module dédié.
2. Mapper chaque scope vers le mécanisme natif de la cible.
3. Ajouter des golden files.
4. Autoriser explicitement les nouveaux chemins dans la validation des artefacts.
5. Tester les fichiers existants, marqueurs incomplets, symlinks et changements concurrents.

## Données locales

Par défaut, l'état projet se trouve dans `.agentctx/state.db`, répertoire ignoré par Git. Les tests réels doivent consigner uniquement des métriques agrégées dans `docs/validation.md`.
