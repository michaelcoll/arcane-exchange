# Spec : Avatar utilisateur (image_url) et endpoint profil par username

## Contexte

- Le JWT Clerk émis à la connexion contient un claim `image_url` (avatar de l'utilisateur), mais le backend ne le décode pas : seuls `sub`, `username`, `exp`, `azp` sont parsés (`ClerkClaims` dans `auth_service.rs`)
- La table `users` ne stocke que `id` et `username` (+ `visibility`) — l'avatar n'est pas persisté
- Le composant front `PlayerAvatar.vue` affiche un placeholder d'initiales : la vraie image de l'avatar n'est jamais montrée
- `PlayerAvatar` est instancié partout avec le **username** d'un joueur (recherche de joueurs, offres de cartes, écrans d'échange) — le front ne manipule jamais l'id utilisateur (`sub`), seulement le username
- Le backend n'expose aucune route de consultation d'un utilisateur par username (seul l'autocomplete existe)

## Objectif

Persister l'URL de l'avatar utilisateur (claim `image_url` du JWT) et exposer un endpoint `GET /user/{username}` retournant le profil public, pour que `PlayerAvatar` devienne autonome (il reçoit un username, récupère lui-même l'avatar et l'affiche, avec repli sur les initiales).

## Solution

### Stockage de l'avatar

- Migration SQLx : ajouter une colonne `image_url` nullable (TEXT) à la table `users`
- Décoder le claim `image_url` (optionnel) en plus des claims existants lors de la validation du JWT Clerk
- Écrire l'image_url en base à chaque login : le front appelle déjà `POST /user/register` à la connexion (upsert) — ce point d'entrée existant alimente/rafraîchit `image_url` à partir du claim du token courant. Un utilisateur existant voit donc son avatar mis à jour à sa connexion suivante
- Un login sans claim `image_url` ne doit pas écraser une valeur existante

### Endpoint

- `GET /user/{username}` — authentifié (extracteur `AuthenticatedUser` existant), accessible pour **n'importe quel** username connu : les avatars des autres joueurs sont affichés par `PlayerAvatar` sur les écrans de recherche/échange, l'endpoint ne peut donc pas être réservé à l'utilisateur courant
- Retourne le profil public tel que stocké en base : `id`, `username`, `avatar_url` (null si jamais fourni)
- La correspondance username est insensible à la casse (comportement existant de `find_by_username`, `LOWER()`)
- L'avatar suit la visibilité du username (déjà public via l'autocomplete), pas celle de la collection (`visibility`) : un joueur en collection privée reste consultable par username
- Même pattern que les autres routes user : handler → use case trait → service → repository port → adapter (hexagonal), DTO sérialisé snake_case et exporté en binding TypeScript comme les DTO existants (`dto.rs` + `ts-rs`/`utoipa`)
- La route `/:username` coexiste avec les routes statiques existantes (`register`, `visibility`, `trade-binders`) qui restent prioritaires — un username identique à l'un de ces segments n'est pas résolvable par cette route (limite acceptée)

### Frontend

- `PlayerAvatar.vue` devient **autonome** : prop `username` (obligatoire) au lieu de `initials` (calculées en interne pour le fallback), récupère lui-même le profil via `GET /user/{username}`, affiche l'image si `avatar_url` présent, sinon les initiales
- Cache par username en session (Map partagé) pour éviter un appel par instance dans les listes (offres, résultats de recherche, échanges) ; durée de vie = session : un avatar changé chez Clerk n'est rafraîchi qu'au rechargement complet de la page
- `useUserService` : ajouter la fonction de récupération du profil
- Tous les sites d'instanciation existants sont migrés (PlayerPicker, DetailModal, pages index/search/trade)

## Cas d'erreurs

- Token absent ou invalide → 401 (comportement existant de l'extracteur)
- Username inconnu en base → 404 (`FunctionalError::UserNotFound`, déjà mappé NOT_FOUND)
- Aucun `image_url` stocké → `avatar_url: null` dans la réponse, le front garde le fallback initiales
- Erreur réseau / réponse non-200 lors de l'appel du composant → fallback initiales, aucun crash
- Échec de chargement de l'image elle-même (URL expirée, invalide, image supprimée) → fallback initiales (gestion de l'événement d'erreur de l'`<img>`), pas d'image cassée
- Utilisateur sans username (claim `username` absent du token) : `register` échoue déjà avec un 400 et l'utilisateur n'est jamais en base → il n'est pas consultable via cette route (404). Comportement hérité de l'existant, non modifié
- Claim absent du token → pas d'écriture en base, valeur existante conservée

## Critères d'acceptance

- [ ] La migration ajoute une colonne `image_url` nullable à `users` sans casser les migrations existantes
- [ ] Le parsing du JWT Clerk accepte le claim `image_url` optionnel
- [ ] Après connexion avec un claim `image_url` présent, la base contient l'URL pour cet utilisateur
- [ ] Après connexion sans claim `image_url`, une valeur précédemment stockée est conservée (non écrasée)
- [ ] `GET /user/{username}` retourne 200 avec `id`, `username` et `avatar_url` pour un username connu
- [ ] `GET /user/{username}` retourne 401 sans token valide
- [ ] `GET /user/{username}` retourne 404 pour un username inconnu en base
- [ ] `GET /user/{username}` est insensible à la casse : même réponse pour `myname` et `MyName`
- [ ] `avatar_url` est null dans la réponse quand aucune image n'est stockée
- [ ] Le binding TypeScript du DTO de réponse est généré et consommable côté front
- [ ] `PlayerAvatar` reçoit un username, charge lui-même le profil, affiche l'image quand `avatar_url` est fourni et les initiales sinon — y compris en cas d'échec réseau de l'appel et en cas d'échec de chargement de l'image (pas d'image cassée)
- [ ] Tous les sites d'instanciation de `PlayerAvatar` sont migrés vers la prop `username`
- [ ] Tests unitaires couvrant : upsert avec/sans claim, handler (200, 404, avatar null), composant front (image, fallback, échec réseau, échec de chargement de l'image)
