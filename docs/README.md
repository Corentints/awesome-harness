# Documentation d'AgentContext

AgentContext est un compilateur de contexte local-first pour agents de code. Il apprend des dépôts, de l'historique Git et des sessions Claude Code/Codex afin de produire le plus petit ensemble d'instructions persistantes, utiles et vérifiables.

> Statut : phase de conception. Le périmètre initial décrit ici correspond à la V0.1, pas à un produit déjà implémenté.

## Commencer ici

- [Vision produit](product.md) : problème, proposition de valeur, principes et périmètre.
- [Architecture](architecture.md) : composants, frontières et choix techniques.
- [Modèle de domaine](domain-model.md) : règles, preuves, scopes et états.
- [Pipeline d'analyse](pipeline.md) : ingestion, détection, inférence et compilation.
- [Stratégie d'analyse sémantique](semantic-analysis.md) : couverture des inputs, priorités, providers CLI et quotas.
- [Interface en ligne de commande](cli.md) : commandes et parcours utilisateur.
- [Sécurité et confidentialité](privacy.md) : traitement local, secrets et consentement.
- [Évaluation](evaluation.md) : critères de qualité et benchmark historique.
- [Journal de validation](validation.md) : essais réels et enseignements, sans données de session.
- [Développement et installation](development.md) : prérequis, validation et extensions.
- [Roadmap](ROADMAP.md) : jalons et critères de sortie.
- [Todolist](TODO.md) : backlog d'implémentation ordonné.

## Promesse

> Les agents de code oublient. AgentContext apprend de leurs erreurs et compile les instructions minimales nécessaires pour éviter qu'elles se répètent.

La boucle produit est la suivante :

```text
dépôt + sessions + préférences
              │
              ▼
     collecte et normalisation
              │
              ▼
  règles traçables dans un IR local
              │
              ▼
    revue et validation humaine
              │
              ▼
 CLAUDE.md / AGENTS.md spécialisés
              │
              └──── nouvelles sessions ────↺
```

## Principes non négociables

1. **Déterministe d'abord** : ne pas demander à un LLM ce que le dépôt permet d'établir.
2. **Preuve** : chaque règle doit expliquer pourquoi elle existe.
3. **Scope minimal** : une règle vit au niveau le plus étroit où elle reste vraie.
4. **Budget de contexte** : chaque token persistant doit être utile.
5. **Évolution** : une règle peut être confirmée, remplacée ou devenir obsolète.
6. **Local-first** : aucune session ne quitte la machine sans consentement explicite.
7. **Validation humaine** : la V0.1 ne modifie pas les fichiers d'instructions silencieusement.

## Terminologie

Le nom de produit est **AgentContext** et le binaire proposé est `agentctx`. L'expression **Agent Context Compiler** décrit la catégorie du produit. Le README racine reste la source de réflexion initiale ; ces documents en constituent la version structurée et actionnable.
