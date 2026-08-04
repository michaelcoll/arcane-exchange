# Spec : Passage à des images du site The Gatherer

## Contexte

Je souhaiterais faire étudier une nouvelle fonctionnalité et que tu crées un plan pour lister toutes les étapes.
Actuellement, j'utilise l'id Scryfall pour que coté front, on appelle un endpoint du site Scryfall pour afficher la
carte.

## Objectif

Je souhaiterais utiliser des images différentes pour afficher les cartes. Je voudrais passer sur les images du site the
gatherer, qui sont en WebP, d'une taille un peu
plus grande, d'une qualité supérieure et en plus existe en plusieurs langues.

## Solution

### Récupération des données

Pour cela, il va falloir faire du web scrapping pour rechercher le nom de l'image.

L'URL pour obtenir la carte est de la forme :

https://gatherer.wizards.com/ECL/en-us/41/wanderbrine-preacher

Qui est donc composée :

- du code du set en majuscule
- de la locale
- du collector_number
- du nom en minuscule séparé de tiret

Une fois le flux html trouvé, il faut rechercher la balise <meta />

Par exemple dans le cas du lien donné plus haut, la balise vaut :

```html
<meta
  property="og:image"
  content="https://gatherer-static.wizards.com/Cards/medium/530325A982E8ADA7B336093036D69C306198A8A1B1E36D11DE2F9FAEA7186FE5.webp"
/>
```

La seule donnée importante dans l'url de l'image est l'id. dans l'exemple plus haut la valeur à conserver est
`530325A982E8ADA7B336093036D69C306198A8A1B1E36D11DE2F9FAEA7186FE5`

### Stockage

Il faudrait rajouter une nouvelle colonne `id_gatherer` dans la table `card`

### Méthode de récupération

Pour récupérer, il faut ajouter un traitement asynchrone qui se lancerait après un import de collection Manabox et qui
regarderait si des cartes dans `card` n'ont pas d'`id_gatherer` et si c'est le cas, utiliser le scrapping web pour
obtenir la valeur.

### Affichage

Coté front, il faut utiliser l'id gatherer à la place de celui de scryfall pour construire l'url de l'image.

## Cas d'erreurs

Dans le cas où, l'url construite à partir des informations de la carte n'existe pas, il faut logger un warn et laisser
la colonne en bdd vide.
