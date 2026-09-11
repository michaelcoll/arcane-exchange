---
status: amended by ADR-0015
date: 2026-07-08
---

# L'identité d'une carte est une clé composite métier

Une carte est identifiée par son set, son numéro de collection et sa langue — jamais par son
identifiant Scryfall ou Cardmarket. Ces identifiants externes sont des attributs enrichis après coup
par des flux asynchrones : ils peuvent être absents, et un même identifiant Scryfall couvre plusieurs
lignes du catalogue (langues, et à l'origine finitions). Les prendre pour clé rendrait l'identité
d'une carte dépendante de la disponibilité d'un service tiers et non déterministe.

## Conséquences

- Toutes les tables qui référencent une carte propagent ce triplet, et toute charge utile d'API qui
  désigne une carte transporte ses composantes séparément.
- Une résolution par identifiant Scryfall peut correspondre à plusieurs cartes : elle doit être
  rendue déterministe par un ordre explicite, jamais laissée arbitraire.
- La finition faisait initialement partie de cette clé ; l'ADR-0015 l'en a retirée.
