---
date: 2026-08-16
---

# Le binder est une propriété de l'exemplaire possédé, pas de la carte

L'export ManaBox indique dans quel classeur chaque exemplaire est rangé. Cette information est
conservée sur l'entrée de collection, et non ajoutée à l'identité de la carte : une même carte rangée
dans deux classeurs différents donne deux entrées distinctes pour un même joueur.

C'était la condition pour qu'un joueur puisse un jour n'ouvrir à l'échange qu'une partie de sa
collection. Fusionner les exemplaires comme auparavant aurait effacé la seule information permettant
de distinguer ce qui est proposé de ce qui est gardé.

## Conséquences

- Toute lecture qui présente « une carte d'un joueur » doit agréger ses entrées : quantités sommées et
  plafonnées explicitement, prix d'achat moyenné par les quantités, date d'ajout la plus ancienne.
  Une lecture non agrégée est un défaut, pas une variante.
- Une entrée peut n'avoir aucun binder : l'unicité d'une entrée doit traiter cette absence comme une
  valeur à part entière.
- Les binders ne sont ni créés, ni renommés, ni supprimés par la plateforme. L'import fait foi, et un
  binder disparu d'un nouvel export emporte les sélections d'échange qui le visaient.
