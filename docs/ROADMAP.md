# Roadmap

La roadmap est organisée par résultats démontrables. Les dates ne sont volontairement pas fixées tant que le débit du projet n'est pas connu.

## Prototype — prouver le signal

**Objectif :** démontrer que l'historique contient des erreurs répétées exploitables.

Livrables :

- import minimal d'un format Claude et d'un format Codex ;
- modèle `NormalizedSession` ;
- détection déterministe de corrections et marqueurs explicites ;
- commande expérimentale `agentctx mistakes` ;
- rapport local avec occurrences et liens vers les preuves.

Critère de sortie : sur au moins un historique réel, le rapport fait émerger plusieurs erreurs spécifiques que l'utilisateur juge dignes d'une règle persistante.

## V0.1 — boucle locale complète

**Objectif :** passer des sources à des fichiers d'instructions validés.

Livrables :

- `init`, `analyze`, `mistakes`, `review`, `diff`, `apply`, `explain` ;
- scan déterministe du dépôt et des instructions existantes ;
- ingestion incrémentale Claude/Codex ;
- IR SQLite pour règles, preuves et décisions ;
- extraction de corrections, répétitions, commandes, préférences d'outil et interdictions ;
- provider Claude CLI ou Codex CLI avec réponses JSON Schema ;
- rendu racine `CLAUDE.md` et `AGENTS.md` ;
- redaction et inférence distante désactivée par défaut.

Critère de sortie : un utilisateur peut analyser, revoir et appliquer un contexte traçable sans perte de contenu ni envoi distant implicite.

## V0.2 — qualité et maintenance

**Objectif :** maintenir le contexte dans le temps.

Livrables :

- indexation de tous les véritables messages utilisateur, sans dépendre uniquement de mots-clés ;
- priorisation déterministe et traitement sémantique par lots sous budget ;
- providers CLI Codex et Claude avec sélection automatique et fallback contrôlé ;
- reprise explicite après indisponibilité, erreur d'authentification ou quota atteint ;
- inférence de scope plus robuste ;
- analyse globale multi-projets ;
- détection des conflits et de l'obsolescence ;
- `agentctx doctor` ;
- vérification des commandes par rapport au dépôt actuel ;
- explication des scores et alertes.

Critère de sortie : l'outil retrouve une préférence récurrente formulée sans mot-clé attendu, détecte un changement d'outillage réel et explique la règle ou le remplacement proposé, sans retraiter tout l'historique.

## V0.3 — scopes natifs et revue avancée

**Objectif :** exploiter correctement les mécanismes propres à chaque agent.

Livrables :

- génération de `.claude/rules/*.md` ;
- génération d'`AGENTS.md` imbriqués ;
- scopes par dossier et motif de fichier ;
- distinction complète partagé/personnel ;
- TUI de revue avec preuve et preview côte à côte.

Critère de sortie : une même règle logique est placée correctement pour Claude et Codex sans duplication inutile au niveau racine.

## V0.4 — mesure et optimisation

**Objectif :** démontrer l'amélioration au lieu de seulement la supposer.

Livrables :

- benchmark de tâches historiques ;
- métriques d'utilité des règles ;
- optimisation sous budget de tokens ;
- `agentctx optimize` ;
- comparaison avant/après sur réussite, corrections, tours et tokens.

Critère de sortie : le contexte optimisé égale ou améliore la couverture mesurée tout en réduisant le coût sur un benchmark reproductible.

## V1 — gestionnaire de contexte extensible

**Objectif :** stabiliser le produit et ouvrir de nouvelles cibles.

Livrables candidats :

- formats et migrations stables ;
- compatibilité multiplateforme et distribution simple ;
- fournisseurs API optionnels ;
- nouvelles cibles comme Cursor, Copilot ou Gemini, selon la demande ;
- mode CI `check` ;
- documentation de création d'adaptateurs et renderers.

Critère de sortie : AgentContext est installable, récupérable après erreur, documenté et extensible sans modifier son domaine central.

## Après V1

À valider par l'usage : synchronisation multi-machine, règles d'équipe, politiques d'organisation, analytics, bot GitHub, intégrations PR et éventuelle interface Tauri. Ces axes ne doivent pas retarder la preuve de valeur locale.

## Dépendances entre jalons

```text
Prototype
   └── V0.1 : boucle complète
          ├── V0.2 : maintenance
          │      └── V0.4 : mesures fiables
          └── V0.3 : scopes natifs
                    └──────────────┬── V1
                         V0.4 ─────┘
```
