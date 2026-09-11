---
date: 2026-07-08
---

# Les images de cartes viennent de Gatherer, par scraping

Les visuels de cartes étaient servis par l'API Scryfall à partir de l'identifiant Scryfall. Gatherer
propose des images de meilleure qualité, en WebP, plus grandes et déclinées par langue — mais aucune
API : l'identifiant d'image doit être extrait de la balise `og:image` de la page HTML de la carte,
dont l'URL se reconstruit à partir du set, de la langue, du numéro de collection et du nom.

Le scraping est assumé : la qualité et le multilingue priment, et l'identifiant extrait est stocké
sur la carte, donc récupéré une seule fois par carte plutôt qu'à chaque affichage.

## Conséquences

- Une dépendance à la structure HTML d'un site tiers, qui peut changer sans préavis.
- La résolution est un flux asynchrone déclenché après l'import de collection, pas une étape
  bloquante : une carte dont l'URL ne résout pas reste sans identifiant Gatherer, avec un
  avertissement journalisé, et l'import réussit quand même.
