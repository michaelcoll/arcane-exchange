---
status: proposed
date: 2026-11-05
---

# L'index de scan est clé par set et numéro, la langue est lue sur la carte

Pour scanner vite, et même sans réseau, l'app iOS reconnaît une carte contre un index embarqué
plutôt qu'en interrogeant le backend à chaque carte. L'OCR lit la ligne de collection, qui porte
déjà le set, le numéro et la langue. L'index n'a donc pas besoin d'une ligne par langue : il est
clé par set et numéro, et la langue retenue est celle lue sur la carte. Une impression absente de
Scryfall dans la langue lue, cas fréquent pour les tokens français, reste ainsi scannable.

L'index porte, par set et numéro, la rareté, le nombre de faces et le nom au moins en anglais et en
français, avec repli sur l'anglais. Les images restent servies par le backend.

## Conséquences

- L'identité d'une Card ne change pas (ADR-0001) : la langue lue complète le set et le numéro.
- Seules les cartes portant une ligne de collection lisible (environ 2014 et après) sont
  scannables. Les autres s'ajoutent par recherche dans le catalogue.
