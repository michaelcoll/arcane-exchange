---
status: amends ADR-0001
date: 2026-09-11
---

# Le foil est un attribut de l'exemplaire, pas de la définition de carte

La finition faisait partie de l'identité de carte. C'était une erreur de modélisation : une même
impression existe en normal et en foil, et les deux partagent nom, rareté et identifiants externes.
Le catalogue contenait donc jusqu'à deux lignes strictement redondantes par impression, chaque flux
d'enrichissement traitait deux fois la même carte, et une résolution par identifiant Scryfall
retournait une ligne arbitraire — dont la finition décidait, silencieusement, quelle série de prix
était servie.

La finition est désormais portée par l'exemplaire : par la possession et par la carte engagée dans un
trade, où elle reste une composante de l'unicité — posséder les deux finitions d'une même carte, et
les échanger séparément, reste parfaitement légitime.

## Conséquences

- Le prix dépendant de la finition, chaque lecture de prix doit la tirer de l'exemplaire. Un oubli ne
  se signale pas toujours à la compilation et produirait des prix faux — dont un historique de valeur
  de collection persisté.
- La déduplication à l'import se dissocie : sur l'identité seule côté catalogue, sur l'identité plus
  la finition côté collection. Les confondre ferait disparaître une version foil en la fusionnant avec
  sa version normale.
- Rupture de contrat assumée : l'historique de prix d'une carte exige désormais une finition
  explicite, faute de quoi il la devinerait — et la devinait mal.
- Le compteur de cartes du catalogue baisse sans qu'aucune carte n'ait disparu : il comptait des
  variantes. Aucune compensation n'est introduite.
- La migration vérifie elle-même son hypothèse de fusion et s'arrête sur une divergence réelle, en
  distinguant une valeur face à une absence — cas normal pour les identifiants résolus
  asynchroniquement — d'un véritable conflit.
- Le catalogue ne connaît pas les finitions réellement parues : demander le prix foil d'une impression
  qui n'existe pas en foil donne un prix absent, pas une erreur.
