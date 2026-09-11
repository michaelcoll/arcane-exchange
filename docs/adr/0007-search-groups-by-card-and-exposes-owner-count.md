---
date: 2026-07-31
---

# La recherche regroupe par carte et n'expose qu'un nombre de possesseurs

La recherche retournait une ligne par possesseur : une carte détenue par trois joueurs apparaissait
trois fois, et le total paginé comptait des possessions, pas des cartes. Les résultats sont désormais
regroupés par carte, et la seule information d'audience exposée est le nombre de joueurs distincts
qui la proposent.

Le choix n'est pas que cosmétique : il fixe ce qu'est un résultat de recherche. Une carte est un
résultat, un possesseur n'en est pas un — la liste nominative des détenteurs relève des offres d'une
carte précise, un écran délibérément distinct.

## Conséquences

- Le total paginé compte des cartes uniques, ce qui rend la pagination lisible.
- La recherche n'a plus besoin de résoudre l'identité des possesseurs, sauf lorsqu'elle est
  explicitement restreinte à un joueur donné — auquel cas le compteur vaut toujours un et n'est plus
  affiché.
- Aucune donnée par possesseur ne peut être restituée par la recherche, y compris une date d'ajout :
  elle serait ambiguë dès qu'une ligne regroupe plusieurs joueurs.
