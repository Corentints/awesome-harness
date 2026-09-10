# Pipeline d'analyse

## Étapes

```text
discover → normalize → extract evidence → find candidates
         → infer → deduplicate → resolve conflicts
         → scope → rank → review → compile → apply
```

## 1. Découverte

Le scanner identifie le dépôt courant, les fichiers d'instructions existants et les sessions Claude Code/Codex associées. Il produit des références stables sans copier l'intégralité des transcriptions.

Chemins attendus, à confirmer par les adaptateurs et tests de compatibilité :

- Claude Code : `~/.claude/projects/<project>/<session-id>.jsonl` ;
- Codex : `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl` ;
- historique utilisateur Codex, source secondaire : `~/.codex/history.jsonl`.

## 2. Normalisation

Chaque adaptateur convertit sa source vers un contrat commun :

```rust
struct NormalizedSession {
    id: SessionId,
    source: AgentSource,
    project: Option<ProjectId>,
    started_at: DateTime<Utc>,
    messages: Vec<Message>,
    events: Vec<Event>,
}
```

Les champs inconnus sont tolérés aux frontières. Des fixtures anonymisées protègent contre les changements de formats internes.

## 3. Indexation et priorisation déterministes

Le dépôt fournit directement des faits comme le package manager, le langage, les tests, les commandes CI ou les dossiers générés. Chaque véritable message utilisateur normalisé entre dans l'index d'analyse. Les enveloppes techniques, instructions injectées par l'outil, doublons et messages vides sont exclus avant toute inférence.

Les signaux comme « toujours », « jamais », « utilise », « plutôt que », « à l'avenir » et leurs équivalents anglais, ainsi que les corrections de commandes échouées, servent à attribuer une priorité. Ils ne constituent plus une condition nécessaire pour qu'un message puisse être analysé. Cette phase produit des segments candidats ordonnés et une justification de leur priorité, pas des règles définitives.

## 4. Redaction

Avant toute inférence, même locale, le pipeline détecte et masque les secrets connus : clés API, tokens Bearer, JWT, clés privées, identifiants cloud, assignations `.env` et chaînes à forte entropie. L'appel distant reste interdit sans opt-in.

## 5. Inférence sémantique par lots

Les messages nouveaux ou modifiés sont regroupés par lots, en commençant par les priorités les plus fortes et sans dépasser le budget configuré. Les entrées de faible priorité restent éligibles : elles peuvent être différées, mais ne sont pas définitivement ignorées à cause de l'absence d'un mot-clé.

Le provider doit retourner un objet conforme à un JSON Schema : texte canonique, type, scope proposé, confiance et identifiants des preuves utilisées. Les suggestions validées sont consolidées avec les candidats déterministes et persistées avec leur provenance.

Les providers `claude -p` et `codex exec` sont placés derrière la même interface `InferenceProvider`. Un futur mode `auto` choisit une CLI installée et authentifiée selon un ordre configurable. Un fallback vers l'autre CLI n'est autorisé que si elle respecte le même consentement de confidentialité. Une erreur de quota ou de rate limit interrompt proprement les lots concernés et conserve un point de reprise.

## 6. Consolidation

Le moteur :

- fusionne les paraphrases ;
- compte les occurrences indépendantes ;
- détecte les incompatibilités ;
- compare avec les règles existantes ;
- marque les connaissances potentiellement périmées ;
- propose le scope et la visibilité ;
- calcule un classement explicable.

## 7. Revue

La revue présente une règle à la fois avec son scope, sa confiance et ses preuves. L'utilisateur peut accepter, rejeter, modifier le texte ou changer le scope. La décision est persistée.

## 8. Compilation et application

Le compilateur sélectionne les règles acceptées sous le budget configuré, puis chaque renderer produit des artefacts cibles. `agentctx diff` affiche le résultat. `agentctx apply` demande une confirmation avant toute écriture dans la V0.1.

## Indexation incrémentale

Pour éviter de retraiter tout l'historique, la base conserve par source : chemin logique, taille, date de modification, hash et version de parseur. Elle conserve aussi, pour chaque message utilisateur, son hash, sa priorité, la version de l'analyse et son état de traitement. Une source ou un message est retraité lorsque son contenu, son adaptateur ou la version d'analyse change.

Le traitement des JSONL doit être en streaming. Les sessions brutes ne sont pas dupliquées dans SQLite.

## Gestion des erreurs

- un JSONL partiellement invalide produit un diagnostic localisé et poursuit l'analyse si possible ;
- un provider indisponible conserve les candidats déterministes et permet une reprise ;
- un quota atteint ne déclenche pas de retry sans limite et peut permettre le passage au provider de fallback autorisé ;
- une réponse LLM invalide est rejetée, jamais devinée ;
- une cible modifiée depuis le calcul du diff doit être relue avant application ;
- aucune erreur d'une source ne doit corrompre les règles déjà validées.
