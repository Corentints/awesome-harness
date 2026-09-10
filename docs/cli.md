# Interface en ligne de commande

Le binaire proposé est `agentctx`. La CLI doit rester utilisable sans compte distant et distinguer clairement analyse, décision et écriture.

## Parcours principal V0.1

```text
agentctx init
agentctx analyze
agentctx mistakes
agentctx review
agentctx diff
agentctx apply
agentctx explain <rule-id>
```

## Commandes

### `agentctx init`

Détecte le projet, crée la configuration locale si nécessaire et résume les sources disponibles. La commande ne lance pas d'inférence et ne modifie pas les fichiers d'agents.

### `agentctx analyze`

Indexe les sources nouvelles ou modifiées, extrait les signaux et consolide les règles candidates. Sa sortie distingue faits déterministes, segments envoyés au provider et candidats produits.

Options structurantes envisagées :

- `--global` pour l'analyse multi-projets, à partir de la V0.2 ;
- `--no-llm` pour limiter l'analyse aux faits et heuristiques ;
- `--provider <name>` pour surcharger le provider configuré ;
- `--batch-size <n>` et `--batch-characters <n>` pour borner chaque requête d'inférence ;
- `--format json` pour l'automatisation future.

### `agentctx mistakes`

Expose la valeur avant même la génération des fichiers : erreurs répétées, nombre d'occurrences, preuves et règle suggérée. Cette vue est un objectif prioritaire du prototype.

### `agentctx review`

Présente les candidats non décidés. Actions minimales : accepter, rejeter, éditer, choisir le scope et la visibilité. Une interface à prompts suffit en V0.1 ; la TUI est prévue en V0.3.

### `agentctx diff`

Compile sans écrire et affiche le diff prévu par fichier cible. La sortie doit signaler les règles omises à cause du budget ou d'un conflit non résolu.

### `agentctx apply`

Relit les cibles, vérifie qu'elles n'ont pas changé depuis le diff, affiche le résumé et demande confirmation. L'écriture doit être atomique autant que possible et préserver le contenu non géré par AgentContext.

### `agentctx explain <rule-id>`

Affiche texte canonique, statut, scope, score et preuves d'une règle. Cette commande doit fonctionner hors ligne.

## Commandes postérieures

- V0.2 : `doctor` pour contradictions, obsolescence, duplication et commandes invalides ;
- V0.4 : `optimize` pour réduire le budget sans dégrader la couverture ;
- futur CI : `check` et `update` non interactifs.

## Configuration

Configuration utilisateur proposée : `~/.config/agentctx/config.toml`.

Configuration projet : `.agentctx.toml`.

```toml
[general]
language = "auto"

[analysis]
min_occurrences = 2
max_context_tokens = 2500

[llm]
provider = "codex-cli"
max_batch_characters = 24000

[privacy]
allow_remote_inference = false
redact_secrets = true

[targets]
claude = true
codex = true
```

Les valeurs projet spécialisent les valeurs utilisateur sans pouvoir désactiver silencieusement les protections de confidentialité.

## Codes de sortie

La convention exacte reste à figer, mais les catégories doivent distinguer : succès, erreur de configuration, source illisible, provider indisponible, validation LLM échouée, conflit d'écriture et violations détectées par un futur `check`.
