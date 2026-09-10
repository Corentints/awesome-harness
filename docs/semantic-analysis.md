# Stratégie d'analyse sémantique

## Décision

AgentContext doit considérer chaque véritable message utilisateur, et pas seulement ceux qui contiennent un vocabulaire de correction connu. Les marqueurs déterministes restent utiles pour ordonner le travail, mais ne doivent pas créer un angle mort permanent.

Cette évolution vise à détecter des préférences récurrentes exprimées de façon indirecte, par exemple plusieurs demandes équivalentes formulées avec des mots différents.

## Parcours cible

```text
messages utilisateur normalisés
          │
          ├── exclusion : enveloppes techniques, injections, vides, doublons
          │
          ▼
priorité déterministe et explicable
          │
          ▼
lots incrémentaux sous budget
          │
          ▼
Codex CLI ou Claude Code CLI
          │
          ▼
candidats persistés + preuves + revue humaine
```

Les corrections explicites, interdictions et préférences durables sont traitées en premier. Les demandes ambiguës ou probablement ponctuelles restent dans l'index et sont analysées lorsque le budget le permet. Le hash du message et la version de l'analyse évitent de consommer à nouveau le provider sans changement pertinent.

## Providers CLI

Les deux providers cibles exposent le même contrat `InferenceProvider` :

- `codex-cli`, déjà présent, invoque Codex en mode non interactif ;
- `claude-cli`, à implémenter, invoque Claude Code en mode non interactif ;
- `auto`, à implémenter, choisit une CLI disponible et authentifiée selon un ordre configuré.

Le résultat doit respecter le JSON Schema du domaine. Le provider ne décide jamais seul d'écrire une règle : ses suggestions deviennent des candidats traçables soumis à consolidation puis à revue humaine.

Un fallback peut reprendre les lots non traités après une indisponibilité, une erreur de quota ou un rate limit. Il doit être borné, visible dans le rapport et interdit si le provider de remplacement ne bénéficie pas du même consentement d'inférence distante.

## Coût et limites

Utiliser une CLI déjà connectée évite de demander une clé API et peut éviter une facturation API séparée. Cela ne rend pas l'inférence gratuite : les appels consomment les quotas ou limites de l'abonnement Claude ou Codex associé.

Avant chaque envoi, AgentContext doit afficher le provider et le volume prévu. Après l'analyse, le rapport indique au minimum le nombre de messages et de lots traités, différés ou échoués, ainsi que l'usage remonté par la CLI lorsqu'il existe.

## Validation attendue

Un benchmark comparera deux variantes sur les mêmes historiques anonymisés :

1. filtrage actuel fondé sur les marqueurs explicites ;
2. tous les inputs utilisateur, priorisés et traités sous le même budget.

La décision finale doit s'appuyer sur le rappel des préférences utiles, le nombre de faux positifs, la consommation, la durée et la qualité des preuves. Le gain recherché est une meilleure couverture sans envoyer aveuglément l'intégralité des conversations à chaque exécution.
