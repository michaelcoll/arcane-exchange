---
date: 2026-07-19
---

# L'état de chaque partie d'un trade est porté par des colonnes appariées

Acceptation, confirmation de l'échange physique, note et montant dû sont stockés en deux colonnes
côté trade — une pour l'initiateur, une pour le répondant — plutôt que dans une table de
participants. Un trade a exactement deux parties, fixes et connues dès sa création : une table de
jointure n'apporterait que des jointures supplémentaires pour une cardinalité qui ne variera pas.

Les acceptations et confirmations sont des horodatages plutôt que des booléens : ils disent aussi
_quand_, pour un coût identique.

## Conséquences

- Le statut du trade n'est jamais une donnée indépendante : il se recalcule à partir de ces colonnes
  (une seule acceptation renseignée, ou les deux) à chaque transition.
- Ces colonnes ne sont jamais exposées telles quelles. L'API bascule systématiquement le point de vue
  vers « moi » et « le partenaire », pour que les clients n'aient pas à savoir quel rôle ils jouent.
- Ouvrir un jour un trade à plus de deux parties imposerait de reprendre le modèle entièrement.
