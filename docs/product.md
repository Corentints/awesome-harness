# Vision produit

## Problème

Les développeurs maintiennent aujourd'hui `AGENTS.md`, `CLAUDE.md` et leurs variantes à la main. Ce contexte se dégrade : les corrections récurrentes sont oubliées, les règles obsolètes persistent, les scopes se mélangent et les mêmes erreurs reviennent dans les conversations.

Les différents agents n'organisent pas leurs instructions de la même manière. Claude Code dispose notamment de fichiers globaux, locaux et de règles ciblées par chemin ; Codex spécialise le contexte grâce à une hiérarchie de fichiers `AGENTS.md`. Une simple génération de Markdown ne suffit donc pas.

## Proposition de valeur

AgentContext détermine, pour chaque connaissance candidate :

- **quoi** retenir ;
- **pourquoi** la retenir ;
- **où** l'appliquer ;
- **comment** la rendre pour chaque agent ;
- **quand** la remplacer ou la supprimer.

Le produit compile ces décisions vers les formats natifs des agents. Son but n'est pas de maximiser la quantité d'information, mais le rapport entre utilité et coût en tokens.

## Utilisateur cible initial

Un développeur individuel qui :

- utilise Claude Code et/ou Codex sur plusieurs semaines ;
- possède des transcriptions locales exploitables ;
- maintient un dépôt Git ;
- veut réduire les corrections répétées sans envoyer son historique à un service distant.

Les usages équipe, CI, synchronisation multi-machine et intégrations GitHub sont postérieurs au MVP.

## Résultat attendu de la V0.1

Après `agentctx analyze`, l'utilisateur doit obtenir une petite liste de règles pertinentes, reliées à leurs preuves, puis pouvoir les accepter, les modifier ou les rejeter avant de prévisualiser et d'appliquer les changements.

Le test qualitatif principal est volontairement exigeant : sur un dépôt utilisé depuis plusieurs mois, les premières règles proposées doivent révéler des habitudes ou erreurs récurrentes qui méritent réellement d'être persistées. Des recommandations génériques comme « écrire du code propre » n'ont aucune valeur.

## Périmètre V0.1

### Sources

- dépôt Git courant et configurations de base ;
- historique Git utile à la récence et aux changements d'outillage ;
- instructions existantes (`AGENTS.md`, `CLAUDE.md`, `.claude/**`) ;
- sessions locales Claude Code ;
- rollouts locaux Codex.

### Connaissances détectées

- corrections explicites de l'utilisateur ;
- instructions répétées ;
- commandes de travail ;
- préférences d'outillage ;
- interdictions.

### Sorties

- candidats stockés dans un IR local ;
- preuves consultables ;
- revue interactive ;
- prévisualisation du diff ;
- rendu vers `CLAUDE.md` et `AGENTS.md`.

## Hors périmètre initial

- analyse architecturale profonde ou universelle ;
- parsing généralisé avec tree-sitter ;
- interface graphique ou TUI complète ;
- modification automatique sans revue ;
- intégration GitHub ou CI ;
- mode équipe ;
- APIs LLM distantes activées par défaut ;
- scores de qualité arbitraires non adossés à des données.

## Différenciation

AgentContext repose sur quatre propriétés liées :

| Propriété | Question traitée |
|---|---|
| Preuve | Pourquoi cette règle existe-t-elle ? |
| Scope | Quel est le niveau minimal où elle est vraie ? |
| Budget | Son utilité justifie-t-elle son coût en contexte ? |
| Évolution | Est-elle encore valide aujourd'hui ? |

## Indicateurs produit

- taux d'acceptation des candidats proposés ;
- diminution des corrections utilisateur répétées ;
- part des règles disposant de plusieurs preuves indépendantes ;
- réduction des contradictions et règles obsolètes ;
- tokens persistants économisés à couverture égale ou supérieure ;
- taux de réussite et nombre de tours sur des tâches historiques.
