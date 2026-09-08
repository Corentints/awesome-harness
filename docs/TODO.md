# Todolist

Cette liste traduit la roadmap en tâches d'implémentation. Une case n'est cochée que lorsque le code, les tests et la documentation associés sont terminés.

## P0 — fondations du prototype

- [x] Initialiser le crate Rust, le formatage, le lint et les tests.
- [x] Définir les identifiants typés et `NormalizedSession`, `Message`, `Event`.
- [x] Définir `Rule`, `Evidence`, `RuleScope`, `Visibility` et `RuleStatus`.
- [x] Créer des fixtures JSONL Claude et Codex minimales, anonymisées.
- [x] Implémenter `SessionSource` et l'adaptateur Claude sur fixtures.
- [x] Implémenter `SessionSource` et l'adaptateur Codex sur fixtures.
- [x] Lire les JSONL en streaming et tolérer les événements inconnus.
- [x] Détecter le dépôt courant et associer les sessions au projet.
- [x] Extraire des segments avec les marqueurs de correction français et anglais.
- [x] Regrouper les corrections répétées sans LLM pour le premier rapport.
- [x] Ajouter `agentctx mistakes` avec occurrences et références de preuves.
- [ ] Tester le prototype sur un historique réel sans publier ses données.
- [ ] Consigner les vrais positifs, faux positifs et signaux manquants.

## P0 — boucle V0.1

- [x] Ajouter la configuration utilisateur et `.agentctx.toml` avec fusion documentée.
- [x] Implémenter `agentctx init` et la détection des sources disponibles.
- [x] Scanner les fichiers suivis par Git avec la crate `ignore`.
- [x] Extraire package manager, commandes, outils de test et dossiers générés.
- [ ] Importer `AGENTS.md`, `CLAUDE.md` et `.claude/**` existants.
- [x] Concevoir le schéma SQLite et les migrations initiales.
- [x] Stocker les références, hashes et versions de parseur pour l'indexation incrémentale.
- [x] Persister candidats, preuves et décisions de revue.
- [ ] Définir `InferenceProvider` sans dépendance fournisseur dans le domaine.
- [ ] Définir le JSON Schema de sortie et ses validations.
- [ ] Implémenter un faux provider déterministe pour les tests.
- [ ] Implémenter le premier provider CLI choisi.
- [ ] Batcher uniquement les segments retenus par les filtres.
- [ ] Canonicaliser et dédupliquer les règles proposées.
- [ ] Implémenter un premier score explicable et borné.
- [x] Ajouter `agentctx analyze` avec résumé des données traitées.
- [x] Ajouter `agentctx review` avec accepter, rejeter, éditer et changer le scope.
- [x] Mémoriser les rejets pour éviter les propositions identiques répétées.
- [x] Ajouter `agentctx explain <rule-id>`.
- [x] Implémenter `ClaudeRenderer` pour le fichier racine.
- [x] Implémenter `CodexRenderer` pour le fichier racine.
- [x] Définir des marqueurs ou une stratégie sûre pour les sections gérées.
- [x] Ajouter des golden tests pour les deux renderers.
- [x] Ajouter `agentctx diff` sans écriture.
- [x] Ajouter `agentctx apply` avec contrôle de concurrence et écriture atomique.

## P0 — confidentialité et robustesse

- [ ] Définir le modèle de menace et les données susceptibles de quitter la machine.
- [ ] Implémenter la redaction des formats de secrets connus.
- [ ] Ajouter la détection prudente des chaînes à forte entropie.
- [ ] Garantir l'opt-in pour toute inférence distante.
- [ ] Afficher le provider et le volume de contenu avant envoi.
- [ ] Tester qu'aucun secret synthétique n'atteint le faux provider distant.
- [ ] Échapper les contenus de sessions afin qu'ils restent des données non fiables.
- [ ] Refuser les artefacts dont le chemin sort des cibles autorisées.
- [ ] Tester les symlinks et changements de fichiers entre `diff` et `apply`.
- [ ] Vérifier que les logs ne contiennent ni secrets ni extraits complets.

## P1 — V0.2 maintenance

- [ ] Formaliser les règles d'inférence de scope et leurs explications.
- [ ] Calculer la diversité de projets pour les candidats globaux.
- [ ] Implémenter `agentctx analyze --global`.
- [ ] Détecter les contradictions exactes puis sémantiques.
- [ ] Suivre `last_confirmed_at`, `last_used_at` et `valid_until`.
- [ ] Détecter une règle contredite par l'état courant du dépôt.
- [ ] Vérifier l'existence et la validité des commandes persistées.
- [ ] Implémenter `agentctx doctor`.
- [ ] Ajouter des recommandations de remplacement sans application automatique.

## P1 — V0.3 scopes natifs

- [ ] Rendre les règles Claude ciblées dans `.claude/rules/`.
- [ ] Rendre les règles Codex dans des `AGENTS.md` imbriqués.
- [ ] Gérer les scopes `Directory` et `FilePattern` de bout en bout.
- [ ] Gérer `Shared` et `Personal`, dont `CLAUDE.local.md`.
- [ ] Détecter et éviter les duplications entre niveaux.
- [ ] Construire la TUI `review` après validation du parcours à prompts.

## P2 — V0.4 évaluation

- [ ] Définir un format de cas de test historique sans fuite de solution.
- [ ] Construire un petit corpus de tâches et contraintes observables.
- [ ] Exécuter les variantes sans contexte, contexte actuel et contexte compilé.
- [ ] Mesurer réussite, corrections, violations, tokens, tours et durée.
- [ ] Calibrer le classement des règles à partir de ces résultats.
- [ ] Implémenter la sélection sous budget de tokens.
- [ ] Implémenter `agentctx optimize` avec rapport avant/après.

## P2 — préparation V1

- [ ] Choisir et documenter les plateformes officiellement supportées.
- [ ] Mettre en place les tests de compatibilité des formats de sessions.
- [ ] Stabiliser les migrations SQLite et leur récupération après échec.
- [ ] Ajouter une commande d'export, purge et diagnostic des données locales.
- [ ] Préparer les paquets et binaires de distribution.
- [ ] Documenter l'extension par nouvelle source, cible ou provider.
- [ ] Prioriser les agents supplémentaires à partir de demandes réelles.

## Décisions ouvertes

- [ ] Choisir le premier provider d'inférence CLI : Claude ou Codex.
- [ ] Choisir le mécanisme de propriété des sections Markdown rendues.
- [ ] Décider si les extraits de preuves sont stockés ou recalculés à la demande.
- [ ] Définir la durée de conservation et le comportement de purge.
- [ ] Définir la stratégie de chiffrement éventuelle de SQLite.
- [ ] Fixer les codes de sortie et le contrat JSON de la CLI.
- [ ] Valider le nom du produit, du binaire et du répertoire de configuration.

## Définition de « terminé » pour la V0.1

- [ ] Installation locale documentée sur les plateformes retenues.
- [ ] Parcours `init → analyze → review → diff → apply` testé de bout en bout.
- [ ] Chaque règle appliquée possède une preuve consultable via `explain`.
- [ ] Réanalyse incrémentale déterministe et décisions préservées.
- [ ] Aucune sortie distante sans opt-in et suite de redaction verte.
- [ ] Aucune perte de contenu utilisateur lors de l'application.
- [ ] Résultats utiles confirmés sur au moins un historique réel.
