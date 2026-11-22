# Passage de PostgreSQL 18 à 19 en production

Migration par réplication logique, en trois déploiements successifs. La production tourne sous Portainer CE
(stack GitOps) : chaque déploiement est **un commit sur `main` qui remplace `docker-compose.yml`**. Portainer le
applique au prochain poll, ou tout de suite avec « Pull and redeploy » sur la stack.

| Déploiement | Contenu du `docker-compose.yml`     | Effet                                                                          |
| ----------- | ----------------------------------- | ------------------------------------------------------------------------------ |
| 1           | `docker-compose.yml` (déjà à jour)  | Démarre PostgreSQL 19 à côté de la 18 et la réplique. Le backend ne bouge pas. |
| 2           | `docker-compose.deploy-2.yml` (ici) | Bascule : la 18 passe en lecture seule, le backend utilise la 19.              |
| 3           | `docker-compose.deploy-3.yml` (ici) | Supprime la 18 et renomme `postgres19` en `postgres`.                          |

Pour les déploiements 2 et 3, copier le fichier correspondant sur `docker-compose.yml` **en gardant les tags d'image
du backend et du frontend de la version en production**, et committer.

## Prérequis

- La stack Portainer pointe sur `docker-compose.yml` à la racine du dépôt.
- Compose ≥ 2.23 sur le serveur : les scripts des services one-shot sont inlinés dans les fichiers compose
  (`configs.content`), car Portainer clone le dépôt dans son propre conteneur et un bind mount relatif
  (`./deploy/...`) monterait un dossier vide.
- Le dossier `/mnt/ssd/containers/config/ccpt/db` peut être créé par Docker (la 19 y range ses données dans
  `19/docker`). Il est à côté de `pgdata/` (la 18) et de `card-images/` : ne jamais toucher à ce dernier.
- Faire une sauvegarde avant de commencer (`pg_dump`, ou copie à froid de `ccpt/pgdata`).
- **Aucune migration SQLx ne doit sortir entre les déploiements 1 et 2** : le DDL n'est pas répliqué, et le backend
  applique ses migrations au démarrage. Ne pas déployer de nouvelle version du backend dans cet intervalle.
- Toutes les tables ont une clé primaire et il n'y a aucune séquence (le script de bascule le revérifie).

## Déploiement 1 : réplication

Merger le commit qui met `docker-compose.yml` dans cet état (c'est le cas de `main` après cette PR).

La 18 redémarre une fois (`wal_level=logical`, le backend se reconnecte). `pg-replicate` copie le schéma sur la 19,
crée la publication et la souscription, puis s'arrête : la copie des données continue en arrière-plan dans Postgres.
Le script est idempotent (un redéploiement est sans danger) et refuse de continuer si la 19 contient déjà des
tables sans souscription (état à moitié fait : un humain décide).

Suivre l'avancement, dans Portainer (Containers, puis le conteneur, puis Logs / Console) ou en shell :

- les logs du conteneur one-shot `pg-replicate` (il reste visible une fois terminé, état « exited ») ;
- dans la console du conteneur `postgres19` : `psql -U postgres`, puis

```sql
SELECT srsubstate, count(*) FROM pg_subscription_rel GROUP BY 1;   -- r = table prête
```

- dans celle de `postgres` (la 18), le retard de la souscription :

```sql
SELECT slot_name, active, pg_size_pretty(pg_wal_lsn_diff(pg_current_wal_lsn(), confirmed_flush_lsn)) AS lag
FROM pg_replication_slots;
```

**Avant de merger le déploiement 2, vérifier que toutes les tables sont en `r`** : la bascule sera courte (elle
l'attend sinon, 30 minutes au plus, avec la 18 encore en écriture).

Retour arrière : dans la console de `postgres19`, `DROP SUBSCRIPTION pg19_upgrade;`, puis supprimer le conteneur
`postgres19` et vider `ccpt/db`. Rien n'est perdu : la 18 n'a jamais cessé d'être la base du backend. Tant que la
souscription existe, la 18 garde du WAL pour elle : ne pas laisser traîner des semaines.

## Déploiement 2 : bascule

Merger le commit qui remplace `docker-compose.yml` par `docker-compose.deploy-2.yml`. Il n'y a rien à arrêter à la
main : Portainer applique le nouveau fichier alors que l'ancien backend tourne encore. Compose recrée le backend
(il arrête l'ancien conteneur) avant de lancer `pg-cutover`, puis ne le démarre qu'une fois la bascule réussie : le
backend est donc arrêté pendant toute la bascule et ne rencontre jamais la base en lecture seule.

`pg-cutover` attend la fin de la copie, passe la base de la 18 en lecture seule et coupe ses connexions, attend que
la 19 ait rattrapé la position WAL de la 18 (`confirmed_flush_lsn` du slot), compare le `count(*)` de chaque table
publique, puis supprime la souscription et la publication. Le backend ne démarre que si tout cela a réussi, et se
connecte à `postgres19`. Le script est idempotent (« already cut over » à un redéploiement). Ses logs sont ceux du
conteneur `pg-cutover` dans Portainer.

En cas d'échec, la 18 reste en lecture seule et le backend ne démarre pas. Retour arrière : dans la console de la
18, `ALTER DATABASE postgres SET default_transaction_read_only = off;`, puis revenir au `docker-compose.yml` du
déploiement 1 (backend sur `postgres`).

**Après la bascule, les écritures sont uniquement sur la 19** : revenir à la 18 perd tout ce qui a été écrit depuis.

## Déploiement 3 : nettoyage et renommage

**Danger : deux PostgreSQL sur le même dossier de données = corruption.** Le conteneur `postgres19` orphelin
continuerait de tourner sur `ccpt/db` pendant que le nouveau `postgres` démarre dessus. Procédure obligatoire, dans
cet ordre :

1. Dans Portainer, **arrêter les conteneurs `postgres19` et `backend`** (stack encore au déploiement 2).
2. Merger le commit qui remplace `docker-compose.yml` par `docker-compose.deploy-3.yml`.
3. Cliquer **tout de suite** « Pull and redeploy » sur la stack, sans attendre le poll : un déploiement qui échoue
   n'est peut-être pas retenté sur le même commit.

Le service `pg-guard` est un filet de sécurité : il fait échouer le démarrage de `postgres` si un `postgres19` répond
encore sur le réseau compose (un orphelin garde son alias). Il ne remplace pas la procédure. S'il échoue : arrêter
`postgres19`, puis « Pull and redeploy ».

Ensuite, supprimer à la main l'ancien `/mnt/ssd/containers/config/ccpt/pgdata` (après vérification), jamais
`db/` ni `card-images/`. Retour arrière : impossible une fois la 18 supprimée sans restaurer la sauvegarde.

## Test

`mise run test-pg-upgrade` joue les trois déploiements dans Docker (projet compose `pg_upgrade_test`, aucun port
hôte, dossiers temporaires), avec un writer synthétique à la place du backend et la base de dev `ccpt_local`
comme jeu de données (lecture seule, `pg_dump`). Il utilise les vrais fichiers compose de ce dossier. Le writer
de l'ancien déploiement tourne pendant le déploiement 2, comme le backend en production.
