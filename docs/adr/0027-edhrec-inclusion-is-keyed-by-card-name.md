---
date: 2026-11-20
status: accepted
---

# Le taux d'inclusion EDHREC est indexé par nom de carte

L'écran d'échange peut comparer les deux côtés par taux d'inclusion EDHREC plutôt que par prix.
EDHREC calcule ce taux par carte au sens du jeu : il est le même pour toutes les impressions, toutes
les langues et toutes les finitions d'un nom. On le stocke donc dans sa propre table, une ligne par
nom anglais, et non sur la Card. C'est un écart assumé à l'ADR-0001 : le nom ne devient pas une
identité de Card, il est seulement la clé d'une donnée externe qui ne connaît pas mieux. Le copier
sur chaque Card l'aurait dupliqué sur autant de lignes et aurait fait d'une mise à jour une écriture
multiple.

EDHREC n'a ni API officielle ni export en masse. Le taux se lit carte par carte sur ses pages JSON
publiques, à un rythme limité. Il est récupéré à l'arrivée d'une nouvelle carte, puis rafraîchi
chaque semaine pour tous les noms en collection. On stocke les deux nombres bruts (decks qui jouent
la carte, decks qui pourraient la jouer) plutôt que le pourcentage.

## Conséquences

- Un nom absent d'EDHREC ou mal converti en slug n'a pas de taux : on ne mémorise pas cette absence,
  il sera retenté au passage suivant.
- Le taux peut avoir jusqu'à une semaine de retard, et dépend d'un format de page qu'EDHREC peut
  changer sans prévenir.
