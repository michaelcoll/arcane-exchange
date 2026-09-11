---
date: 2026-08-08
---

# La réservation d'une carte est dérivée, jamais stockée

Une carte est réservée lorsqu'elle figure dans un trade qu'au moins une partie a accepté. Cet état
n'est porté par aucune colonne : il se lit en interrogeant les trades. Le stocker aurait créé une
donnée à maintenir en cohérence à chaque transition — acceptation, modification qui ramène en
négociation, abandon, abandon en cascade — avec autant d'occasions de laisser une carte réservée par
un trade mort.

## Conséquences

- Abandonner un trade ou le ramener en négociation libère les cartes sans aucune écriture dédiée.
- Quand un trade est accepté pour la première fois, tous les autres trades actifs partageant une de
  ses cartes sont abandonnés : sans état stocké, c'est la seule façon de garantir qu'un exemplaire
  n'est engagé que dans un échange à la fois.
- La réservation est un drapeau informatif : elle n'est pas déduite de la quantité proposée et ne la
  diminue pas.
- Sur l'écran d'un trade, la réservation ne se lit pas carte par carte — elle découle directement du
  statut du trade affiché.
