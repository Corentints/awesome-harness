# Évaluation

AgentContext ne doit pas présenter une note produite par un LLM comme une mesure objective. La qualité se mesure à partir de comportements observables et de tâches reproductibles.

## Questions

1. Les règles proposées sont-elles acceptées par l'utilisateur ?
2. Empêchent-elles la répétition des erreurs observées ?
3. Le même niveau de réussite est-il obtenu avec moins de tokens ?
4. Les règles restent-elles correctes avec le temps ?
5. Le placement par scope évite-t-il de polluer les autres tâches ?

## Benchmark historique

Une session passée peut devenir un cas de test contenant : tâche, actions, erreur, correction et solution. Les informations postérieures à la tâche doivent être séparées du prompt de départ pour éviter toute fuite.

Le format versionné est un fichier JSON par cas. `prompt` contient uniquement l'information disponible au début de la tâche ; `oracle` reste réservé au harness et décrit les commandes et chemins requis ou interdits. `fixture_files` permet de reconstruire un dépôt minimal. Tous les chemins doivent être relatifs et rester dans la fixture. Des exemples synthétiques vivent dans `tests/fixtures/evaluation/`.

Trois variantes sont comparées :

- sans contexte persistant ;
- avec le contexte actuel ;
- avec le contexte compilé par AgentContext.

Le harness vérifie ensuite des contraintes observables, par exemple l'utilisation de pnpm, l'absence de modification d'un dossier généré ou le passage par un QueryBus.

## Métriques

- réussite de la tâche ;
- nombre de corrections utilisateur simulées ou nécessaires ;
- commandes incorrectes ;
- violations d'architecture détectables ;
- échecs de tests ;
- tokens consommés ;
- nombre de tours et durée ;
- précision, rappel et taux d'acceptation des règles candidates.

## Score de santé du contexte

Un futur `agentctx doctor` peut agréger des sous-métriques calculables : spécificité, fraîcheur, cohérence, coût en tokens, qualité des preuves et couverture du benchmark. Chaque composante doit afficher sa méthode de calcul et ses données sources.

## Jeux de tests nécessaires

- fixtures minimales Claude et Codex, versionnées et anonymisées ;
- sessions contenant corrections vraies et faux positifs ;
- règles contradictoires séparées dans le temps ;
- instructions globales, projet, dossier et temporaires ;
- secrets synthétiques pour tester la redaction ;
- dépôts exemples avec npm, pnpm, Cargo et configurations de test variées ;
- golden files pour les rendus Claude/Codex.

## Critères V0.1

Avant de déclarer la V0.1 utilisable :

- les adaptateurs traitent les fixtures sans charger le JSONL complet en mémoire ;
- chaque règle proposée possède au moins une preuve consultable ;
- les décisions de revue survivent à une nouvelle analyse ;
- aucun secret synthétique n'atteint un faux provider distant ;
- le diff est déterministe à état identique ;
- l'application ne détruit pas le contenu non géré ;
- un test utilisateur sur un historique réel produit au moins quelques candidats jugés spécifiques et utiles.
