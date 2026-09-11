# Arcane Exchange

Plateforme d'échange de cartes Magic entre joueurs : chacun importe sa collection, choisit ce qu'il
accepte de mettre à l'échange, cherche les cartes des autres, et négocie des échanges carte contre
carte. L'échange physique et le règlement d'un éventuel écart de valeur se font hors plateforme.

## Language

### Cartes et catalogue

**Card** :
La définition d'une impression de carte — un set, un numéro de collection, une langue. Une donnée de
jeu, sans notion de possession ni de finition.
_Avoid_ : printing, édition, « carte foil » comme entité distincte

**CardId** :
L'identité d'une Card : le triplet set / numéro de collection / langue. C'est la seule clé métier
d'une carte ; les identifiants des catalogues externes n'en font jamais partie.
_Avoid_ : scryfall_id, cardmarket_id comme identité

**Finition** :
Le fait qu'un exemplaire soit foil ou normal. Propriété de l'exemplaire possédé ou échangé, jamais de
la Card.
_Avoid_ : variante, version foil

**Set** :
L'édition dont une carte est issue, identifiée par son code et portant un nom complet.
_Avoid_ : extension, bloc

**Rareté** :
Le rang d'une carte parmi cinq valeurs : Common, Uncommon, Rare, Mythic, Special.
_Avoid_ : niveau, tier

**Price guide** :
Les prix de marché d'une carte pour une finition donnée — plancher, tendance, moyenne — toujours
exprimés en centimes et distincts selon la finition. Un prix inconnu est absent, jamais nul.
_Avoid_ : cote, valeur marchande, prix à zéro

**Historique de prix** :
La série des prix de marché d'une carte, jour par jour, sur une période. À distinguer de l'historique
de valeur d'une collection, qui agrège prix et quantités possédées pour un joueur.

### Collection

**Collection** :
L'ensemble des cartes qu'un joueur possède. Toujours privée : seul son propriétaire la consulte en
tant que telle.
_Avoid_ : inventaire, stock

**Collection entry** :
La possession, par un joueur, d'un certain nombre d'exemplaires d'une carte dans une finition donnée
et un binder donné. Porte la quantité, le prix d'achat et la date d'ajout.
_Avoid_ : ligne de collection, possession, card quantity

**Binder** :
Le classeur physique dans lequel un joueur range des exemplaires, tel que déclaré à l'import. Donnée
importée en lecture seule — la plateforme ne crée, ne renomme et ne supprime jamais de binder.
_Avoid_ : classeur, dossier, deck

**Import** :
Le remplacement intégral de la collection d'un joueur à partir d'un export ManaBox. Asynchrone, suivi
par son état, sa progression et ses erreurs de ligne.
_Avoid_ : synchronisation, upload

### Mise à l'échange

**Visibilité de collection** :
Le réglage par lequel un joueur décide de ce qu'un tiers peut voir de sa collection : `public` (tout),
`trade` (ce que ses règles de mise à l'échange retiennent) ou `private` (rien). `private` par défaut.
_Avoid_ : confidentialité, mode privé

**Binder ouvert à l'échange** :
Un binder que le joueur a coché comme éligible à l'échange. Aucun binder n'est coché par défaut.
_Avoid_ : binder actif, binder public

**Filtre de rareté** :
Le réglage, par rareté, indiquant si elle est ouverte à l'échange et combien d'exemplaires le joueur
garde systématiquement pour lui. Toute rareté est fermée par défaut.
_Avoid_ : règle de trade, quota

**Quantité proposée** :
Le nombre d'exemplaires d'une carte qu'un joueur offre réellement à un tiers, une fois appliqués sa
visibilité, ses binders ouverts et ses filtres de rareté. Seule mesure qui gouverne l'exposition d'une
carte à un tiers. Une quantité proposée nulle est, pour un tiers, indiscernable de l'absence de la
carte.
_Avoid_ : quantité disponible, quantité échangeable

**Offre** :
La proposition implicite d'un joueur pour une carte précise : sa quantité proposée et un prix de
vente. Il n'existe pas d'acte de « mise en vente » — une offre se déduit de la collection et des
réglages.
_Avoid_ : annonce, listing, mise en vente

**Owner count** :
Le nombre de joueurs distincts qui proposent une carte donnée. C'est la seule information d'audience
exposée par la recherche — jamais la liste nominative des possesseurs.
_Avoid_ : nombre de possesseurs, popularité

### Échange

**Trade** :
Une négociation entre exactement deux joueurs, portant sur des cartes des deux côtés. L'échange
physique et le règlement d'un écart de valeur ont lieu hors plateforme.
_Avoid_ : transaction, deal, offre

**Initiateur** / **Répondant** :
Les deux parties d'un Trade : celui qui l'a ouvert et l'autre. Le rôle sert à identifier qui a agi, il
ne confère aucun privilège — les deux parties ont exactement les mêmes droits sur le Trade.
_Avoid_ : acheteur/vendeur, demandeur/propriétaire

**Trade actif** :
Un Trade encore en cours entre deux joueurs, par opposition aux états terminaux. Il n'en existe jamais
plus d'un par paire de joueurs, quel que soit le sens dans lequel il a été ouvert.
_Avoid_ : trade ouvert, négociation en cours

**Carte réservée** :
Une carte engagée dans un Trade qu'au moins une partie a accepté. La réservation se lit dans la
collection de son propriétaire ; elle n'est pas une action, elle découle de l'état du Trade.
_Avoid_ : carte bloquée, carte verrouillée

**Delta cash** :
L'écart de valeur entre les deux côtés d'un Trade. Purement informatif : il est affiché, jamais réglé
par la plateforme.
_Avoid_ : soulte, paiement, balance à payer

**Note** :
L'appréciation, de zéro à cinq, que chaque partie porte sur l'autre à l'issue d'un Trade. Non
modifiable une fois donnée.
_Avoid_ : avis, review, score

### Joueurs

**Joueur** :
Un utilisateur de la plateforme, désigné publiquement par son **username**. Le username est le seul
identifiant de joueur que l'API expose et que les clients manipulent.
_Avoid_ : compte, membre, identifiant utilisateur en surface publique
