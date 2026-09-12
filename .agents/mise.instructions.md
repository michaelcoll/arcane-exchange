# Mise & Development Workflow

**mise est l'unique point d'entrée des commandes locales.** Ne lance jamais `cargo`, `pnpm`, `xcodebuild` ou `sqlx`
directement : `mise.toml` exporte l'environnement dont ils dépendent (`DATABASE_URL`, `BACKEND_PORT`, les URLs des
APIs externes) et fixe les versions des toolchains.

## Découvrir les commandes

Le catalogue des tâches n'est pas documenté ici, il se lit depuis `mise.toml` :

```sh
mise tasks -x                 # toutes les tâches : nom, alias, description
mise tasks --hidden           # + les sous-tâches internes (format-backend, generate-ios, …)
mise tasks info <task>        # la commande réellement exécutée, le dossier, les dépendances
mise tasks deps <task>        # l'arbre des dépendances (utile sur `checks`, `format`, `setup`)
```

Si la réponse de `mise tasks` ne suffit pas à comprendre ce qu'une tâche fait ou quand l'utiliser, **c'est
`mise.toml` qu'il faut corriger** — en complétant la `description` de la tâche, ou en ajoutant un commentaire pour
un « pourquoi » — pas ce fichier.

## Premier checkout

```sh
mise install                  # toolchains ; le hook postinstall installe lefthook
mise run setup                # clean + dépendances frontend
```

Pour l'iOS, copier en plus `ios-app/Config/Local.xcconfig.example` vers `Local.xcconfig` (voir
[ios.instructions.md](ios.instructions.md)).

## Enchaînements qui ne se devinent pas

- **`mise run test-backend` régénère les bindings TypeScript** du front (`frontend-vue/app/bindings/`) : le derive
  `ts-rs` s'exécute à la compilation des tests. Après toute modification d'un DTO ou d'un enum exposé, relancer la
  tâche est le seul moyen de remettre le front à jour.
- **`mise run sqlx-prepare` exige une base qui tourne** et repart de zéro (`rm -fr .sqlx`). Les requêtes SQLx sont
  vérifiées à la compilation, donc une requête modifiée sans `sqlx-prepare` casse le build offline et la CI.
- **`mise run checks`** est le filet complet avant de pousser : docs régénérées, métadonnées SQLx, tests et lints
  (lint iOS compris).
