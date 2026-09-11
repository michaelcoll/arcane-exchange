---
date: 2026-09-04
---

# Le dernier prix connu d'une carte est matérialisé, et l'ordre de rafraîchissement est contraignant

« Le dernier prix Cardmarket d'une carte » était recalculé à chaque requête sur un relevé de plusieurs
millions de lignes, et la seule source pré-calculée existante était jointe aux collections : elle ne
connaissait que les cartes possédées par quelqu'un, et ne pouvait donc pas servir de prix de
référence. Le calcul est désormais matérialisé une fois, indépendamment de toute possession, et
devient le seul endroit où l'agrégat est fait — la vue des collections s'y adosse au lieu de le
refaire.

Le prix n'est plus lu en temps réel : c'est la contrepartie assumée de la matérialisation.

## Conséquences

- **L'ordre de rafraîchissement est porteur de sens** : la source des derniers prix doit être
  rafraîchie avant celle des collections. L'ordre inverse ne lève aucune erreur et sert simplement des
  prix en retard d'un cycle — une régression entièrement silencieuse, à couvrir par un test.
- Les deux sources dépendent du relevé de prix _et_ du catalogue, dont les identifiants externes sont
  résolus par des flux distincts : elles doivent être rafraîchies à chacun de ces flux, pas seulement
  à l'import de prix.
- Un échec de rafraîchissement n'invalide jamais les données déjà écrites, mais doit remonter à
  l'observabilité : son seul symptôme visible serait sinon une source figée servant des prix périmés
  indéfiniment.
- Le prix étant indépendant de la langue, la clé de cette source l'ignore — elle ne peut donc pas
  servir à décider qu'une carte existe, ce contrôle devant rester discriminant sur l'identité complète.
