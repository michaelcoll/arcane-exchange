---
date: 2026-08-09
---

# Le username est le seul identifiant de joueur exposé par l'API

L'identifiant interne d'un joueur — celui que porte le fournisseur d'authentification — n'apparaît
nulle part dans les charges utiles publiques. Un partenaire d'échange, un possesseur, un joueur ciblé
par une recherche sont tous désignés par leur username. La création d'un trade a d'ailleurs été
reconstruite pour cette raison, l'identifiant interne qu'elle exigeait n'étant obtenable par aucune
autre route de l'API.

## Conséquences

- Les clients n'ont jamais à connaître ni à stocker d'identifiant interne, et l'API ne diffuse pas
  l'identifiant d'un compte tiers.
- La correspondance sur username est exacte à la casse près, et le username doit rester unique en
  pratique — garantie par le fournisseur d'authentification, pas par la plateforme.
- Un profil public, consultable par username, expose l'avatar d'un joueur indépendamment de la
  visibilité de sa collection : c'est une identité, pas un contenu.
