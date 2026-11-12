---
status: accepted
date: 2026-11-12
---

# Sur iOS, tout ce qui n'est pas un écran vit dans un framework partagé

Les tests iOS dépendaient de l'app hôte (~70 s de lancement en CI pour moins d'une seconde de
tests) et l'extension des widgets recompilait des sources de l'app. Tout ce qui n'est pas un écran
vit donc dans le framework `ArcaneExchangeCore`, que l'app, l'extension et les tests (sans hôte)
lient. Règle : **écran → app, le reste → framework**. Le code du framework n'importe ni `ClerkKitUI`
ni `NukeUI`.

Clerk et Nuke passent par des produits dynamiques du paquet `APIClient` (`ClerkKitShared`,
`NukeShared`) : liés automatiquement, `ClerkKitUI` et `NukeUI` partaient en statique dans l'app
avec leur propre copie de `ClerkKit` et `Nuke`, dont le framework ne configurait jamais les
singletons. L'extension charge donc aussi le code UI de ces bibliothèques, sans l'embarquer.
`scripts/check-app-linking.sh` vérifie en CI qu'aucun module n'est lié deux fois et que l'app
embarque toutes ses bibliothèques.

Les types partagés sont en `package`, pas `public` : le framework n'a de clients que dans ce dépôt,
et `public` imposerait d'exposer initialiseurs et conformances. Cela demande
`SWIFT_PACKAGE_NAME`, posé dans `Config/Shared.xcconfig`.

Écartés : un paquet Swift local (asset catalog, shader et UIKit le lient au simulateur de toute
façon) et un découpage en plusieurs modules par domaine.
