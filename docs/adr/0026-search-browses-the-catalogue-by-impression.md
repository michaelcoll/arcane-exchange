---
date: 2026-11-15
status: supersedes ADR-0007, amends ADR-0006
---

# La recherche parcourt le Catalogue, impression par impression

Trois recherches coexistaient : celle des cartes proposées par les autres, celle du Catalogue pour
ajouter une carte, et le filtrage de la collection. Chacune avait sa règle de correspondance du nom
et son unité de résultat. Pour qu'une même saisie réponde à « l'ai-je ? », « qui la propose ? » et
« combien vaut-elle ? », y compris depuis les App Intents d'iOS, les deux premières fusionnent en
une seule recherche. Elle part du Catalogue, son résultat est une impression (set et numéro, toutes
langues confondues), et chaque résultat dit combien d'exemplaires le joueur connecté en possède et
combien de joueurs la proposent, lui compris.

La collection reste à part : la lister, la trier par date d'ajout ou par prix d'achat n'est pas
une recherche. La recherche ne porte qu'une seule donnée privée, la quantité possédée par le joueur
connecté ; rien de ce qui concerne un autre joueur n'en sort, hormis le nombre de joueurs qui
proposent l'impression.

## Conséquences

- La recherche ne parle plus seulement des cartes des autres (ADR-0006) : le choix de la route ne
  tranche plus à lui seul ce qui est privé. C'est le contenu du résultat qui l'encadre.
- Une impression que personne ne possède est un résultat. Possession et offre deviennent deux filtres
  indépendants, ce qui permet de chercher « ce qui me manque et que quelqu'un propose ».
- La langue et la finition ne sont plus visibles dans la liste : elles se détaillent sur la fiche.
- L'image d'un résultat reste hétérogène tant que les impressions jamais possédées n'ont pas
  d'image locale.
