---
date: 2026-09-09
---

# L'import de collection est asynchrone et suivi en base

L'import gardait la requête HTTP ouverte pendant tout le traitement : sur une grande collection, le
client restait bloqué sans progression et s'exposait à un timeout de proxy ou de navigateur. L'import
rend désormais la main immédiatement et le traitement se poursuit en tâche de fond, son état, sa
progression et ses erreurs de ligne étant persistés pour survivre à un redémarrage et être consultés
par les clients.

Le découpage retenu tient en une règle : ce que l'utilisateur doit apprendre tout de suite reste
synchrone. L'encodage et le format du fichier sont donc validés avant de répondre — une erreur de
format est une faute de l'appelant, pas un résultat à découvrir plus tard dans un statut.

## Conséquences

- Un seul import à la fois par joueur, garanti par la base et non par une simple vérification
  applicative, pour rester correct en cas de requêtes concurrentes. Une demande concurrente est
  refusée sans perturber l'import en cours.
- Un redémarrage du backend marque en échec les imports interrompus, sans reprise automatique : le
  joueur relance. Cela libère mécaniquement le verrou d'unicité.
- L'historique est borné aux imports récents de chaque joueur, ce qui rend un identifiant d'import
  consultable temporairement seulement.
- Une ligne invalide n'échoue pas l'import : elle est collectée et rattachée au compte-rendu. Un
  fichier dont aucune ligne n'est exploitable échoue sans vider la collection existante.
