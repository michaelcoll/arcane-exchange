---
date: 2026-07-29
---

# La collection privée et la recherche publique sont deux endpoints distincts

Un unique endpoint paginé a d'abord servi les deux usages, un drapeau `owned` distinguant « mes
cartes » de « les cartes de tout le monde ». Ce drapeau mélangeait deux contrats de confidentialité
dans une même route : la route ne disait pas ce qu'elle exposait, et son défaut décidait à lui seul
si des données privées fuyaient ou non.

Les deux usages ont été séparés : un endpoint de collection, toujours restreint au joueur
authentifié, sans aucun moyen de le désactiver ; un endpoint de recherche, qui ne parle jamais que des
cartes des autres. Dans la foulée, les endpoints agissant sur une carte unique ont été regroupés sous
leur propre préfixe, la recherche a été préfixée par type de ressource pour accueillir de futures
recherches, et aucun alias de compatibilité n'a été conservé.

## Conséquences

- Le prix d'achat et la date d'ajout ne peuvent structurellement plus sortir par la voie publique :
  la question « ai-je le droit de voir ce champ ? » est tranchée par le choix de l'endpoint.
- Les deux endpoints continuent de partager filtres, tri, pagination et forme de réponse ; leurs
  divergences de contenu sont explicites plutôt que dérivées d'un paramètre.
- Toute rupture de route est assumée sans période de transition, les clients étant versionnés avec le
  backend.
