---
status: accepted
date: 2026-11-12
---

# Sur iOS, tout ce qui n'est pas un écran vit dans un framework partagé

Les tests iOS dépendaient de l'app hôte (~70 s de lancement en CI pour moins d'une seconde de
tests) et l'extension des widgets recompilait des sources de l'app. Tout ce qui n'est pas un écran
vit donc dans le framework `ArcaneExchangeCore`, que l'app, l'extension et les tests (sans hôte)
lient. Règle : **écran → app, le reste → framework**. Le framework ne dépend ni de `ClerkKitUI` ni
de `NukeUI`, pour que l'extension ne les embarque pas.

Les types partagés sont en `package`, pas `public` : le framework n'a de clients que dans ce dépôt,
et `public` imposerait d'exposer initialiseurs et conformances. Cela demande
`SWIFT_PACKAGE_NAME`, posé dans `Config/Shared.xcconfig`.

Écartés : un paquet Swift local (asset catalog, shader et UIKit le lient au simulateur de toute
façon) et un découpage en plusieurs modules par domaine.
