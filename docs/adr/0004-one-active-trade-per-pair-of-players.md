---
date: 2026-07-19
---

# Un seul trade actif par paire de joueurs

Deux joueurs ne peuvent avoir qu'une seule négociation en cours entre eux, quel que soit le nombre de
cartes en jeu. Demander une deuxième carte au même joueur ne crée pas un second trade : la carte
s'agrège à celui qui existe déjà. La recherche de ce trade est indépendante du sens — peu importe
lequel des deux l'a ouvert.

L'alternative, un trade par carte demandée, correspondait mal au produit : la contre-proposition
consiste précisément à poser plusieurs cartes des deux côtés et à équilibrer l'ensemble, ce qui n'a
de sens que sur une négociation unique.

## Conséquences

- La création d'un trade est idempotente par paire de joueurs : elle renvoie l'identifiant du trade
  actif existant plutôt que d'en créer un second.
- Une carte ajoutée à un trade qu'une partie avait déjà accepté ramène ce trade en négociation et
  annule les acceptations — une modification ne peut pas se glisser dans un trade verrouillé.
- Un trade déjà pleinement accepté refuse tout ajout : le joueur ne peut rien demander de plus à ce
  partenaire avant que l'échange en cours ne soit terminé ou abandonné.
