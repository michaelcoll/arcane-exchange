---
date: 2026-09-29
---

# Les erreurs de l'API portent un code stable, traduit par les clients

Les erreurs de l'API ne portaient qu'un message technique en anglais, que les clients affichaient
tel quel, ou pas du tout quand le client généré ne décodait pas le corps de la réponse. Toute
erreur de l'API porte désormais, en plus de ce message, un code stable dérivé de l'erreur
fonctionnelle. Chaque client traduit ce code en message pour l'utilisateur, et le message technique
ne sert plus qu'au diagnostic.

## Options écartées

- **Messages en français côté serveur** : plus simple, mais fige la langue et la formulation dans
  le backend, alors que le web et iOS présentent l'erreur chacun à sa manière.
- **Code réservé à l'import** : plus petit, mais installe deux contrats d'erreur dans la même API.

## Conséquences

- Le code fait partie du contrat OpenAPI : le renommer casse les clients déjà déployés, en
  particulier l'app iOS, qui ne se met pas à jour en même temps que le backend.
- Un client qui reçoit un code qu'il ne connaît pas affiche un message générique, jamais le message
  technique.
- Une erreur survenue en tâche de fond, comme l'échec d'un import, est persistée avec son code pour
  que les clients la traduisent de la même façon.
