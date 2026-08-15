# Spec : Endpoints de visibilité de collection

## Contexte

La page profil (`frontend-vue/app/pages/profile/index.vue`) affiche déjà un sélecteur "Visibilité de la
collection" (`SegToggle`) avec 3 valeurs (`public` / `trade` / `private`) et leurs explications, mais l'état est
purement local (`ref('trade')`, non persisté). Il n'existe aujourd'hui aucune notion de visibilité au niveau du
domaine `User` ni de la table `users` (colonnes actuelles : `id`, `username`).

## Objectif

Permettre à un utilisateur connecté de consulter et modifier la visibilité de sa collection, avec persistance en
base, et câbler le sélecteur existant de la page profil sur ces nouveaux endpoints.

Les 3 valeurs possibles reprennent celles déjà définies côté front :

- `public` : toute la collection est visible.
- `trade` : seules les cartes retenues par les règles de mise à l'échange sont visibles (le filtrage effectif
  n'est pas dans le périmètre de cette spec — voir Hors scope).
- `private` : la collection n'est visible par personne.

## Solution

### Modèle de données

- Ajouter une colonne `visibility` à la table `users`, contrainte aux 3 valeurs (`public`, `trade`, `private`) via
  un `CHECK`, `NOT NULL`, valeur par défaut `'private'`.
- La valeur par défaut s'applique aussi bien aux lignes déjà existantes (migration) qu'aux nouveaux utilisateurs
  créés via `POST /user/register` (qui ne renseigne pas ce champ).

### Domaine

- Nouveau type `CollectionVisibility` (enum à 3 variants : `Public`, `Trade`, `Private`) dans le domaine, avec
  conversion string ↔ enum pour la persistance et la sérialisation JSON.
- Ajouter le champ `visibility: CollectionVisibility` à `User`.

### Endpoints

Montés sous le même groupe de routes que `/user/register` (tag `auth`), authentifiés (`AuthenticatedUser`,
401 si token absent/invalide) :

- `GET /user/visibility` : retourne la visibilité de l'utilisateur courant.
- `PUT /user/visibility` : met à jour la visibilité de l'utilisateur courant. Body contenant la nouvelle valeur.

Ces endpoints n'exposent et ne modifient que la visibilité de l'utilisateur authentifié — pas d'accès à la
visibilité d'un tiers dans cette spec.

### Architecture

Hexagonale, même pattern que `register` : handler (`adapter_in/user/controller.rs`) → use case → service →
`UserRepository` (port existant, à étendre) → `UserRepositoryAdapter`.

### Frontend

- `profile/index.vue` : au montage de la page, appeler `GET /user/visibility` pour initialiser `vis` (au lieu du
  défaut local `'trade'`) ; à chaque changement via `SegToggle`, appeler `PUT /user/visibility` avec la nouvelle
  valeur.
- Gérer un état de chargement/erreur cohérent avec les autres écrans déjà câblés sur l'API (cf. pattern utilisé
  pour le câblage de l'écran trade).

### Documentation

Documenter les deux endpoints dans `doc/openapi.yml`, sous le tag `auth`, à côté de `/user/register`.

## Cas d'erreurs

- **Token absent ou invalide** : 401 Unauthorized (comportement standard, inchangé) sur les deux endpoints.
- **`PUT /user/visibility` avec une valeur hors des 3 valeurs autorisées** : 400 Bad Request, aucune écriture en
  base.
- **`GET /user/visibility` pour un utilisateur authentifié mais jamais enregistré dans `users`** (pas encore
  passé par `POST /user/register`) : 404 Not Found (`FunctionalError::UserNotFound`, déjà existant).
- **Erreur base de données** : 500 Internal Server Error (pattern d'erreur existant).

## Critères d'acceptance

- [ ] Une migration ajoute la colonne `users.visibility` (`NOT NULL`, `CHECK` sur `public`/`trade`/`private`,
      défaut `'private'`) et migre les lignes existantes à `'private'`.
- [ ] `POST /user/register` sur un nouvel utilisateur crée une ligne avec `visibility = 'private'`.
- [ ] Given un utilisateur authentifié avec `visibility = 'trade'` en base, When j'appelle
      `GET /user/visibility`, Then la réponse est `200 OK` avec `{"visibility": "trade"}`.
- [ ] Given un utilisateur authentifié non enregistré dans `users`, When j'appelle `GET /user/visibility`, Then la
      réponse est `404 Not Found`.
- [ ] Given aucune en-tête `Authorization`, When j'appelle `GET /user/visibility` ou `PUT /user/visibility`, Then
      la réponse est `401 Unauthorized`.
- [ ] Given un utilisateur authentifié, When j'appelle `PUT /user/visibility` avec `{"visibility": "public"}`,
      Then la réponse est `200 OK` (ou `204 No Content`) et la colonne `visibility` de l'utilisateur passe à
      `'public'` en base.
- [ ] Given un utilisateur authentifié, When j'appelle `PUT /user/visibility` avec une valeur invalide (ex:
      `"hidden"`), Then la réponse est `400 Bad Request` et la valeur en base n'est pas modifiée.
- [ ] `profile/index.vue` initialise le `SegToggle` avec la valeur retournée par `GET /user/visibility` au
      chargement de la page.
- [ ] Changer la sélection du `SegToggle` sur `profile/index.vue` déclenche `PUT /user/visibility` et persiste le
      choix (vérifié par rechargement de la page).
- [ ] Les deux endpoints sont documentés dans `doc/openapi.yml`.
- [ ] `mise run lint-backend` passe sans erreur.
- [ ] `mise run lint-frontend` passe sans erreur.

## Hors scope

- Application effective de la règle de visibilité dans les endpoints existants (recherche, autocomplete, accès à
  la collection d'un tiers, etc.) — cette spec ne fait que stocker et exposer la valeur.
- Persistance des "règles de mise à l'échange" (`ProfileTradeRules.vue` / rareté, binders) — reste un état local
  non persisté, non concerné par cette spec.
- Consultation de la visibilité d'un utilisateur tiers.
