# Sécurité et confidentialité

Les transcriptions d'agents peuvent contenir du code propriétaire, des données client, des URLs privées et des secrets. La confiance dans AgentContext dépend donc d'un comportement local-first vérifiable.

## Garanties par défaut

- analyse et stockage locaux ;
- inférence distante désactivée ;
- aucune télémétrie contenant les sessions ou extraits ;
- transcriptions brutes laissées à leur emplacement d'origine ;
- prévisualisation et confirmation avant écriture ;
- chemins et extraits sensibles absents des logs par défaut.

## Providers

Les CLI locales Claude Code ou Codex peuvent elles-mêmes communiquer avec un service. AgentContext doit l'indiquer explicitement : « provider CLI » ne signifie pas « modèle exécuté localement ».

Une API distante n'est utilisable que si `privacy.allow_remote_inference = true`. L'interface doit montrer quel contenu va être envoyé, vers quel provider et après quelle redaction.

## Redaction

La redaction précède tout appel de provider. Elle couvre au minimum :

- clés API et tokens connus ;
- en-têtes Bearer ;
- JWT ;
- clés privées ;
- identifiants AWS/GitHub et assimilés ;
- assignations issues de `.env` ;
- chaînes à forte entropie à proximité d'un nom sensible.

Les masques doivent être stables dans un même lot afin de conserver les relations utiles, sans permettre de reconstruire la valeur originale.

## Minimisation des données

Seuls les segments plausiblement utiles sont envoyés à l'inférence. Les messages complets et fichiers du dépôt ne doivent pas être transmis par commodité. Chaque requête garde localement la liste des preuves incluses, le provider et la version du schéma, sans stocker de secret.

## Écriture des fichiers cibles

Avant `apply` :

1. recalculer ou valider le hash du fichier cible ;
2. afficher le diff ;
3. demander confirmation en mode interactif ;
4. écrire via un fichier temporaire puis remplacement atomique si la plateforme le permet ;
5. ne pas écraser les sections non gérées.

## Menaces à tester

- exfiltration d'un secret présent dans une correction ;
- prompt injection contenue dans une session ou un fichier du dépôt ;
- chemins traversant hors du dépôt lors du rendu ;
- symlink pointant vers un fichier sensible ;
- JSONL malformé ou démesuré ;
- réponse LLM tentant de créer un artefact hors scope ;
- logs ou messages d'erreur contenant des extraits non masqués.

Le contenu analysé est une donnée non fiable, jamais une instruction donnée à AgentContext lui-même.

## Questions à trancher avant une release publique

- format et durée de conservation des extraits dans SQLite ;
- chiffrement éventuel de la base locale ;
- politique de suppression et commande de purge ;
- modèle précis de sections gérées dans les fichiers Markdown ;
- comportement non interactif en CI ;
- liste publique des données envoyées par chaque provider.
