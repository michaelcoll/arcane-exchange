---
date: 2026-07-09
---

# Clerk est la seule autorité d'authentification, la base n'en garde qu'un miroir

L'authentification est intégralement déléguée à Clerk : le client obtient un JWT, le backend le valide
et en extrait l'identité du joueur. Aucun mot de passe, aucune session, aucun jeton ne sont produits
par la plateforme. Une table d'utilisateurs existe malgré tout, mais uniquement comme miroir local des
attributs publics portés par le jeton — identifiant, username, avatar — parce qu'ils doivent être
joignables en base pour résoudre un possesseur, filtrer une recherche par joueur ou afficher un
partenaire d'échange.

Ce miroir est alimenté par un appel explicite du client à chaque connexion, en upsert, plutôt que par
des webhooks Clerk : un seul point d'entrée, pas d'infrastructure de réception d'événements, et le
miroir se resynchronise naturellement à la connexion suivante quand un attribut change chez Clerk.

## Conséquences

- Un joueur peut être authentifié sans exister en base tant qu'il n'a pas effectué cet appel ; les
  endpoints qui ont besoin de sa ligne répondent 404 dans ce cas.
- Un changement de username ou d'avatar chez Clerk n'est visible qu'à la connexion suivante du joueur
  concerné.
- Un attribut absent du jeton n'écrase jamais une valeur déjà connue.
