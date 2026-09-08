# Journal de validation

Ce journal conserve uniquement des résultats agrégés. Aucun contenu de session réel ne doit être ajouté au dépôt.

## 8 septembre 2026 — rollouts Codex locaux

Périmètre : sessions Codex associées au dépôt `awesome-harness`, analyse déterministe, aucun provider LLM, base SQLite temporaire hors du dépôt.

Résultat initial :

- 78 fichiers JSONL parsés sans erreur ;
- 14 candidats détectés ;
- au moins un faux positif critique : une enveloppe technique longue avait été interprétée comme une instruction utilisateur.

Correction appliquée :

- exclusion des messages de plus de 500 caractères pour la détection déterministe ;
- exclusion d'enveloppes techniques connues ;
- détection de `use`/`utilise` limitée aux formulations impératives en début de message ;
- incrément de la version des deux parseurs afin de forcer la réindexation.

Résultat après correction :

- 78 fichiers retraités ;
- 0 candidat pour ce dépôt ;
- aucun contenu de session enregistré dans ce journal.

Conclusion : la compatibilité du parseur avec les rollouts réels est confirmée, ainsi que l'invalidation incrémentale. Ce corpus ne permet pas encore de confirmer la valeur produit, car aucune correction utilisateur durable n'y subsiste après suppression des faux positifs. Il faut poursuivre l'évaluation sur un historique de projet comportant de vraies corrections répétées.
