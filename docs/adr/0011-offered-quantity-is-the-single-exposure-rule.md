---
date: 2026-08-16
---

# La quantité proposée est la règle unique d'exposition à un tiers

Trois réglages gouvernent ce qu'un joueur met à l'échange : la visibilité de sa collection, les
binders qu'il a ouverts, et ses filtres par rareté avec leurs exemplaires gardés. Plutôt que de
laisser chaque endpoint les combiner à sa façon, ils sont réduits à une seule grandeur — la quantité
proposée d'un joueur pour une carte — définie une fois et consommée partout : recherche, offres,
suggestion de joueurs, ajout d'une carte à un trade.

La visibilité par défaut est `private`, et aucun binder ni aucune rareté n'est ouvert par défaut : un
joueur ne propose rien tant qu'il n'a rien décidé. Le défaut fermé a été préféré au défaut ouvert
parce qu'un réglage de confidentialité ne doit jamais s'appliquer rétroactivement à des données déjà
exposées.

## Conséquences

- Une quantité proposée nulle est indiscernable, pour un tiers, de l'absence de la carte : cibler un
  joueur privé, un joueur inexistant ou un joueur qui ne propose rien donne la même réponse vide.
  Demander une carte non proposée est traité comme une carte introuvable, sans révéler le réglage de
  son propriétaire.
- La règle ne s'applique jamais à la vision qu'un joueur a de sa propre collection.
- Elle porte sur l'état au moment de la lecture : un trade déjà engagé n'est pas invalidé
  rétroactivement quand une carte cesse d'être proposée, et retirer une carte d'un trade reste
  toujours possible.
- Ses trois entrées changent en dehors des imports : toute forme de pré-calcul doit refléter un
  changement de réglage sans attendre le prochain import.
