---
date: 2026-10-15
---

# iOS : les erreurs de l'API sont levées par un middleware, jamais lues dans les Output générés

Chaque appel de l'app iOS faisait un switch sur l'`Output` généré de son opération. On en compte 36,
chacun avec son enum distinct et sans protocole commun. Le résultat : le traitement des 401 était
dupliqué et incohérent, et les messages étaient déduits du statut HTTP au lieu du code stable
d'ADR-0018. Désormais, un middleware du client transforme toute réponse ≥ 400, ainsi que tout échec
réseau, en une seule erreur `APIError`. Cette erreur porte le code, le statut et le message en
français. Les appelants ne lisent que le cas de succès, par les accesseurs générés, et les cas
d'erreur typés des `Output` ne sont jamais lus.

## Options écartées

- **Fonction générique `call`** : elle ne peut pas abstraire 36 enums sans protocole commun sans un
  shim écrit à la main par opération.
- **Méthodes par ressource écrites à la main** : l'interface est la plus riche, mais elle double
  chacune des 36 opérations, qu'il faut ensuite maintenir quand le contrat change.

## Conséquences

- Les cas `.conflict`, `.unauthorized` et les autres cas d'erreur générés restent dans le code
  généré, mais aucun appelant ne doit y recourir. Un switch sur l'un d'eux est du code mort.
- Un 401 du client authentifié déconnecte le joueur, une seule fois même si plusieurs requêtes
  échouent en même temps. Cela ne vaut que dans l'app : le widget et le client anonyme reçoivent la
  même erreur sans jamais déconnecter, car le widget partage la session Clerk de l'app par le
  Keychain.
