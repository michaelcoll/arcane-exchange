---
status: accepted
date: 2026-11-12
---

# Sur iOS, tout ce qui n'est pas un écran vit dans un framework partagé

Les tests iOS dépendaient de l'app hôte (~70 s de lancement en CI pour moins d'une seconde de
tests) et l'extension des widgets recompilait des sources de l'app. Tout ce qui n'est pas un écran
vit donc dans le framework `ArcaneExchangeCore`, que l'app, l'extension et les tests (sans hôte)
lient. Règle : **écran → app, le reste → framework**. `HomeWidgetReload` y va aussi : WidgetKit est
un framework système que l'extension lie de toute façon.

Les types partagés sont en `package`, pas `public` : le framework n'a de clients que dans ce dépôt,
et `public` imposerait d'exposer initialiseurs et conformances (cela demande `SWIFT_PACKAGE_NAME`).

Conséquence : une bibliothèque que le framework et l'app lient tous deux doit l'être en dynamique,
une seule fois. Liée en statique d'un côté, chacun en a sa copie, et un singleton configuré par le
framework reste vide dans l'app (Clerk plantait au lancement). Un contrôle en CI le vérifie sur
l'app construite. Le prix : Clerk et Nuke sont liés avec leur partie UI, que l'extension charge donc
aussi, sans l'embarquer.

Écartés : un paquet Swift local (asset catalog, shader et UIKit le lient au simulateur de toute
façon) et un découpage en plusieurs modules par domaine.
