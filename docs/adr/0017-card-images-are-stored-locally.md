---
date: 2026-09-24
---

# Les images de cartes sont téléchargées et servies par la plateforme

Remplace [0002](0002-card-images-scraped-from-gatherer.md).

Jusqu'ici, on stockait l'identifiant d'image Gatherer, résolu dans la seule langue de la carte, et
chaque client construisait l'URL, Gatherer ou, à défaut, Scryfall. Désormais, un enrichissement
asynchrone après l'import télécharge les images elles-mêmes. Le backend les écrit sur un volume
partagé avec le front, et Nitro les sert. Les clients ne dépendent plus des sites tiers, et l'image
retenue est la meilleure disponible, décidée une seule fois côté serveur.

La source d'image suit une préférence stricte : Gatherer dans la langue, puis Gatherer en anglais,
puis Scryfall. Gatherer passe en premier parce que ses images sont officielles, plus grandes que la
version Scryfall affichée jusqu'ici, et déjà en WebP : on les garde octet pour octet. Le dernier
recours Scryfall télécharge la version `normal` (JPEG 488 × 680) et la réencode en WebP avec perte. Le scraping de
Gatherer (ADR 0002) reste nécessaire pour découvrir les URL d'image, y compris celle du verso d'une
carte double face.

Le nom de fichier se déduit du CardId, jamais d'un identifiant externe : `{SET}_{numéro}_{LANGUE}.webp`
pour le recto, suffixé `_back` pour le verso, où la langue est celle de l'image elle-même. Une carte
en fallback (sans image Gatherer dans sa langue) utilise l'image anglaise de la même carte, qu'elle
vienne de Gatherer en anglais ou de Scryfall, dont l'image est toujours l'impression anglaise. Cette
image est enregistrée une seule fois, sous le nom de la carte anglaise, et partagée : une carte
anglaise importée plus tard la trouve déjà présente.

## Conséquences

- Une carte n'accepte qu'une source qui fournit toutes ses faces, pour éviter un recto Gatherer
  associé à un verso Scryfall.
- Gatherer sert parfois des vignettes (environ 200 px de large) au lieu d'une vraie image. Une
  image Gatherer de moins de 500 px de large est rejetée comme introuvable. Le seuil vise ces
  vignettes, pas les images légèrement plus petites que les autres : certaines éditions (PIP, par
  exemple) n'ont sur Gatherer que des images de 646 px, qu'on préfère à un JPEG Scryfall réencodé.
  Il est placé juste au-dessus de Scryfall `normal` (488 px), pour qu'une image Gatherer retenue ne
  soit jamais plus petite que le dernier recours. Le seuil s'applique à chaque face, et une seule
  face trop petite disqualifie la source. Scryfall, dernier recours, est accepté sans condition.
- Seul un « introuvable » (404, page sans image, image trop petite) fait passer à la source
  suivante. Une erreur
  technique laisse la carte en attente, pour ne pas figer une source moins bonne à cause d'un
  incident passager.
- La source retenue est enregistrée sur la carte ; être en fallback s'en déduit (toute source
  autre que Gatherer dans la langue de la carte), sans colonne dédiée. Une image n'est jamais
  retéléchargée automatiquement : un endpoint de maintenance relance les cartes en fallback et
  celles en attente.
- L'image anglaise partagée ne se dégrade jamais. Sa qualité se lit sur les cartes qui l'utilisent
  déjà. Venue de Gatherer, elle est réutilisée sans appel réseau. Venue de Scryfall, on tente
  encore Gatherer en anglais, mais on ne la retélécharge jamais depuis Scryfall. Quand elle est
  remplacée par une meilleure, toutes les cartes qui la partagent sont alignées sur la nouvelle
  source : on a préféré garder la source sur chaque carte plutôt qu'une table d'images, au prix
  de cette synchronisation.
- Une image anglaise qui n'est plus utilisée (sa carte a trouvé depuis son image dans sa langue)
  reste sur le disque : elle servira si la carte anglaise est importée.
- Nitro sert les images en `immutable` sur un an, pour que Cloudflare les mette en cache. Comme
  un fichier peut être remplacé sous le même nom, l'URL exposée aux clients est versionnée par
  l'origine du fichier (Gatherer ou Scryfall), et non par la source de la carte : les cartes qui
  partagent une image partagent aussi son URL. Un remplacement change l'URL, sans purge de cache.
- Le backend devient dépositaire de fichiers : le volume des images doit être sauvegardé et monté
  en écriture sur le backend, en lecture sur le front.
