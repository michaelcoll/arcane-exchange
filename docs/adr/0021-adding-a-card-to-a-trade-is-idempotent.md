---
status: amends ADR-0004, ADR-0016
date: 2026-11-02
---

# L'ajout d'une carte à un trade est idempotent et porte sur un exemplaire

Le bouton « Échanger » d'une Offre doit être sûr à répéter : il signifie « ouvre le trade actif avec
ce joueur et assure-toi que cette carte y figure de son côté ». Tant que chaque appel augmentait la
quantité, chaque client devait se protéger lui-même d'un second appui, et aucun n'y parvenait de
façon fiable.

`POST /trades/{trade_id}/cards` garantit donc la présence de la carte, à un exemplaire. Si la carte
est déjà du côté de ce propriétaire, rien n'est écrit — ni quantité, ni statut, ni acceptations, ni
date de modification — et l'appel réussit. La requête n'a plus de champ de quantité ; une ancienne
app qui l'envoie encore est ignorée, le corps n'étant pas strict.

## Conséquences

- ADR-0004 reste valable pour tout ajout réel : une carte absente ajoutée à un trade accepté par
  une seule partie le ramène en négociation, et un trade pleinement accepté la refuse.
- Un ajout sans effet ne ramène pas le trade en négociation : un second appui ne défait aucune
  acceptation. Il réussit même sur un trade pleinement accepté qui contient déjà la carte, et
  reste refusé (`TradeNotModifiable`) sur un trade terminé.
- La présence est décidée sous le verrou du trade (ADR-0016), avant d'écrire la transition : deux
  ajouts concurrents de la même carte donnent un seul exemplaire.
- La disponibilité et la réservation ne sont contrôlées que pour un ajout réel, à un exemplaire.
- Une quantité supérieure à un déjà présente dans un trade reste telle quelle, sans migration.
- Demander plusieurs exemplaires d'une même carte n'est plus possible par cet appel.
