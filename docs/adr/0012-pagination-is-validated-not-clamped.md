---
date: 2026-08-17
---

# La pagination est validée par le domaine et refusée, jamais corrigée en silence

Les règles de pagination vivent dans un type unique du domaine, seul capable de produire une
pagination valide. Une demande hors bornes est rejetée en erreur fonctionnelle au lieu d'être
silencieusement ramenée dans les clous : un client qui demandait dix mille éléments en recevait cent
avec un succès, sans jamais apprendre que sa requête était fausse.

La profondeur maximale est bornée par l'offset et non par le numéro de page — une petite page permet
donc d'aller plus loin —, et chaque endpoint fixe sa propre borne en dur au plus près de son service :
parcourir l'intégralité d'une collection et consulter les premières offres d'une carte n'appellent pas
la même profondeur, et ce n'est pas un réglage d'exploitation.

## Conséquences

- Le contrat change pour les clients : des requêtes jusque-là tolérées reçoivent une erreur 400
  nommant le paramètre en cause et la borne attendue.
- Un endpoint paginé ne manipule plus directement le numéro de page ni la taille de page, et le calcul
  de l'offset disparaît des adaptateurs.
- Une valeur par défaut unique s'applique partout : un client qui a besoin d'une autre taille de page
  la demande explicitement.
- Une page valide au-delà du nombre de résultats reste un succès à liste vide — seule la demande est
  jugée, pas le résultat.
