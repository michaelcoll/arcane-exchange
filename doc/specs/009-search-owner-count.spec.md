# Spec : Remplacer owner_username par owner_count dans le endpoint de recherche

## Contexte

L'endpoint `GET /search/card` retourne actuellement une ligne par possesseur de carte.
Si trois utilisateurs possèdent la même carte, il y a trois lignes dans les résultats,
chacune avec `owner_username` pointant vers le possesseur concerné.

Cette approche crée :
- des doublons visuels côté frontend (la même carte apparaît plusieurs fois)
- une pagination trompeuse (`total` compte les possesseurs, pas les cartes)
- une jointure `users` inutile puisqu'on ne fait qu'afficher un nom

## Objectif

Dans les réponses de `GET /search/card`, remplacer le champ `owner_username` par
`owner_count`, indiquant le nombre de joueurs distincts qui possèdent la carte.

Les résultats sont regroupés par carte : chaque combinaison unique de
`set_code / collector_number / language_code / foil` n'apparaît qu'une seule fois,
peu importe le nombre de possesseurs.

## Solution

### Comportement de recherche

- Regroupement par carte unique dans les résultats de `GET /search/card`
- `total` de la pagination reflète le nombre de cartes uniques correspondant aux
  filtres, pas le nombre de possesseurs

### Payload API

- `CollectionCardResponse.owner_username` (`string | null`) est supprimé
- `CollectionCardResponse.owner_count` (`integer`) est ajouté : nombre de joueurs
  distincts possédant la carte
- Le reste du payload (`collection_entry`, `price_guide`, etc.) est inchangé

### Repository

- Réutiliser le mode de recherche publique existant (`search_paginated`, sans filtre
  par utilisateur) en le faisant agréger par carte plutôt que retourner une ligne par
  possesseur
- La jointure sur la table des utilisateurs n'est plus nécessaire pour ce mode ; seul
  le nombre de possesseurs distincts est utile

### Frontend

- Le binding généré (`CollectionCard`) expose `owner_count` à la place de
  `owner_username`
- La clé d'itération sur les résultats de recherche (page "Trouver une carte") doit
  être `scryfall_id` uniquement, plus besoin de la combiner avec le nom du possesseur
- Les composants qui affichaient le nom du possesseur dans le contexte de la recherche
  (carte, modale de détail) doivent être adaptés pour afficher un nombre de possesseurs
  à la place

### Hors scope

- `GET /collection` (collection privée de l'utilisateur) : non concerné, reste filtré
  par utilisateur et continue de retourner les vraies infos de collection (`quantity`,
  `purchase_price`, `added_at`)
- `GET /card/offers` : non concerné, continue de lister les offres individuelles par
  vendeur avec `owner_username`

## Cas d'erreurs

- Une carte retournée par la recherche a toujours au moins un possesseur (la source de
  données ne référence que des cartes réellement possédées) — pas de cas "0 possesseur"
  à gérer
- Le comptage doit porter sur des utilisateurs distincts, pas sur le nombre de lignes
  de collection (un même utilisateur ne doit jamais être compté plusieurs fois pour une
  même carte)

## Critères d'acceptance

- [ ] `GET /search/card` retourne une seule ligne par carte, même si plusieurs joueurs
      la possèdent
- [ ] Le champ `owner_count` correspond au nombre de joueurs distincts possédant la
      carte
- [ ] Si 3 joueurs possèdent la même carte, elle apparaît une fois avec
      `owner_count = 3`, et compte pour 1 dans `total`
- [ ] Le champ `owner_username` n'apparaît plus dans la réponse de `/search/card`
- [ ] `GET /collection` (collection privée) garde son comportement actuel : pas de
      champ `owner_count`, `collection_entry` réel retourné
- [ ] `GET /card/offers` garde son comportement actuel : `owner_username` par offre
      individuelle
- [ ] Le frontend utilise `scryfall_id` comme clé unique d'itération sur les résultats
      de recherche
- [ ] `mise run lint-backend` et `mise run lint-frontend` passent sans erreur
