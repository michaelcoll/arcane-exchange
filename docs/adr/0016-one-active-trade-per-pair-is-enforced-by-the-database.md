---
status: amends ADR-0004
date: 2026-09-24
---

# L'unicité du trade actif par paire est garantie par la base

ADR-0004 fixe qu'une paire de joueurs n'a qu'un seul trade actif. Cette règle n'était tenue que par
un verrou en mémoire du process, autour d'une recherche du trade existant suivie de sa création :
l'invariant vivait dans l'application, pas dans la donnée. Rien n'empêchait une écriture hors de ce
chemin de le violer, et il cessait de tenir dès qu'un second process écrivait dans la même base.

La base porte désormais l'invariant : une contrainte d'unicité, indépendante du sens de la paire,
couvre les seuls trades actifs. La création d'un trade devient une opération atomique « créer ou
renvoyer l'actif » : une création concurrente n'échoue pas, elle converge vers le même trade.
C'est le même idiome que pour l'import de collection (ADR-0014), qui résout le même besoin.

## Conséquences

- Les créations concurrentes, y compris croisées quand les deux joueurs ouvrent un trade l'un vers
  l'autre au même moment, aboutissent à un seul trade.
- L'ajout d'une carte vérifie la quantité disponible, la réservation et le statut du trade dans la
  même transaction que l'écriture, sous verrou du trade : deux ajouts concurrents ne peuvent pas
  dépasser ensemble la quantité proposée, et une acceptation survenue entre-temps fait échouer
  l'ajout plutôt que de laisser une carte se glisser dans un trade accepté.
- Les doublons antérieurs à la contrainte ont été résolus en gardant, pour chaque paire, le trade le
  plus avancé dans la négociation puis le plus ancien ; les autres sont abandonnés.
- La réservation d'une carte lit l'état des autres trades : une acceptation concurrente sur un autre
  trade n'est pas sérialisée par ce verrou. Cet écart, antérieur à cette décision, reste ouvert.
