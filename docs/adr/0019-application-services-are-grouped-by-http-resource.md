---
date: 2026-10-14
---

# Les services applicatifs sont rangés par ressource HTTP, un fichier par use case

`application/service/` mélangeait des fichiers portant jusqu'à neuf use cases et des modules qui
n'en implémentent aucun (authentification, file d'enrichissement, worker, parsing CSV). Chaque use
case a désormais son fichier `service/<ressource>/<action>.rs`, où `<ressource>` reprend le nom du
dossier `infrastructure/adapter_in/` dont le contrôleur l'appelle ; un use case sans route va avec
la ressource qui le déclenche. Le code partagé par les use cases d'une ressource vit dans le module
parent `<ressource>.rs`. `service/` ne contient plus que des use cases : le reste remonte à la racine
d'`application/` ou devient un port et son adapter.

## Options écartées

- **Fichiers à plat, nommés d'après le use case** : aucun dossier, mais une quarantaine de fichiers
  et nulle part où ranger le code partagé.
- **Regroupement par concept métier** (`card_import/`, `enrichment/`…) : plus juste sur le fond,
  mais il fait diverger les arborescences de `service/` et d'`adapter_in/`, alors qu'on retrouve un
  use case depuis sa route.

## Conséquences

- Une ressource à un seul use case a quand même son dossier : la règle ne connaît pas d'exception.
- Déplacer une route d'un dossier `adapter_in/` à un autre implique de déplacer ses use cases.
