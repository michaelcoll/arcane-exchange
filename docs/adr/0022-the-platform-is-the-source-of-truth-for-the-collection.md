---
status: accepted, amends ADR-0010, ADR-0014
date: 2026-11-05
---

# La plateforme fait foi pour la collection

Jusqu'ici, ManaBox faisait foi : l'import remplaçait toute la collection et c'était le seul moyen de
la modifier, y compris pour refléter un échange terminé. Le scan de cartes depuis l'app iOS et
l'ajout par recherche apportent des exemplaires que ManaBox ignore, comme le contenu des produits
scellés : un réimport les effacerait. La plateforme devient donc la référence de la collection, et ManaBox ne sert plus
qu'à l'amorcer.

## Options écartées

- **Cohabitation** : l'import ne remplace que ce qui vient de ManaBox. Une même carte présente des
  deux côtés serait comptée deux fois, sans aucun moyen de le détecter.
- **ManaBox reste la référence** : le scan produit un fichier à réimporter dans ManaBox. Le
  rattachement d'une carte à son achat se perdrait au retour.

## Conséquences

- La collection s'édite sur la plateforme : ajout par recherche dans le catalogue, quantité,
  finition, retrait, et gestion des binders (création, renommage, suppression, déplacement). La
  conséquence « l'import fait foi » de l'ADR-0010 tombe : un binder est créé par le joueur.
- L'import reste un remplacement intégral, réservé au chargement initial ou à une remise à zéro
  explicitement confirmée. Il continue d'écarter les tokens et les proxies, qui ne sont pas des
  exemplaires.
- Un Trade `COMPLETED` transfère ses cartes : elles sont retirées des binders ouverts à l'échange du
  cédant et rangées dans le binder de réception du receveur.
- Une carte peut exister sans être possédée par personne : les vues construites sur la possession
  ne peuvent plus le supposer.
