---
status: accepted
date: 2026-11-07
---

# L'index de scan est clé par set et numéro, la langue est lue sur la carte

Pour scanner vite, et même sans réseau, l'app iOS reconnaît une carte contre un index embarqué
plutôt qu'en interrogeant le backend à chaque carte. L'OCR lit la ligne d'impression, qui porte
déjà le set, le numéro et la langue. L'index n'a donc pas besoin d'une ligne par langue : il est
clé par set et numéro, et la langue retenue est celle lue sur la carte. Une impression absente de
Scryfall dans la langue lue reste ainsi scannable.

L'index porte, par set et numéro, la rareté et le nom au moins en anglais et en français ; à
défaut de nom français, l'app montre le nom anglais. Les images restent servies par le backend.

Le prototype (#646) l'a confirmé sur de vraies cartes : plus de 90 % de lectures correctes en
moins de 0,5 s, en français comme en anglais, foils et cartes double face compris. Le verso d'une
transform ou d'une modale porte la même ligne d'impression que son recto : une carte double face
est un seul objet Scryfall, avec un seul numéro, et ses deux faces mènent à la même entrée.
L'index ne porte donc pas de nombre de faces, prévu avant le prototype pour réunir les faces d'une
même Card.

## Conséquences

- L'identité d'une Card ne change pas (ADR-0001) : la langue lue complète le set et le numéro.
- Seules les cartes portant une ligne d'impression lisible (2014 et après) sont scannables. Les
  autres s'ajoutent par recherche dans le catalogue, comme les numéros non numériques (`H13`, `★`).
- Les tokens ne sont pas scannés : Scryfall et MTGJSON ne donnent pas de `productId` Cardmarket
  fiable pour un token double face. Un token n'est donc pas un exemplaire : il ne figure ni dans
  l'index ni dans le catalogue, et ne s'ajoute pas par recherche.
