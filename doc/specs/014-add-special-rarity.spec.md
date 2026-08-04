# Spec : Ajouter la rareté "Special"

## Contexte

Le système gère aujourd'hui 4 niveaux de rareté pour les cartes : Common (C), Uncommon (U), Rare (R) et Mythic (M). La plupart de ces valeurs correspondent aux codes de rareté de Scryfall.

Scryfall utilise un code **S** pour les cartes **Special** (ex : nourriture, sponsorship, marketing et autres cartes promotionnelles hors cadre de set standard). Cette rareté n'est pas actuellement reconnue par le tracker.

Les CSV d'import de ManaList contiennent déjà cette valeur dans la colonne `Rarity`. Les cartes avec `rarity = special` rejetées silencieusement ou causent une erreur de parsing.

## Objectif

Rendre le tracker capable de gérer un cinquième niveau de rareté **Special** (code `S`), depuis l'import CSV jusqu'à l'affichage et le filtrage côté frontend.

## Solution

### Domaine backend (`RarityCode`)

- Étendre l'enum `RarityCode` avec une variante `S`
- Ajouter `"special" | "s"` comme alias dans `RarityCode::try_new()`
- Ajouter le format affiché `"S"` dans `Display`
- Ajouter les tests correspondants (try_new sur `"special"`/`"S"`/`"Special"`, case insensitive, display)

### DTO API (`RarityCodeParam`)

- Étendre l'enum `RarityCodeParam` (module `collection/dto.rs`) avec la variante `S` et l'attribut utoipa `#[enum_value] = "S"`
- Ajouter la conversion `RarityCodeParam::S => RarityCode::S` dans le `From`
- Ajouter les assertions correspondantes dans les tests

### Frontend (`RarityCode` TypeScript)

- Ajouter `'S'` au type union dans `frontend-vue/app/bindings/RarityCode.ts`
- Ajouter `S: 'Special'` dans `RARITY_LABELS` et `'S'` en **dernier** de `RARITIES` (`['M', 'R', 'U', 'C', 'S']`) dans `CollectionFilters.vue`

### BDD — Aucune migration

La colonne `rarity` est `VARCHAR(1)` sans contrainte CHECK — les valeurs `"special"` et `"S"` sont acceptées telles quelles.

### Cas d'erreurs

- Import CSV avec `Rarity = special` → le parseur retourne un `FunctionalError::InvalidRarityCode` aujourd'hui. Après l'ajout, la carte est correctement parsée avec `RarityCode::S`.
- Frontend demandant `?rarity=S` → l'API accepte le paramètre (la validation en amont via `RarityCodeParam` inclut `S`), et le filtrage passe la valeur au repository.

### Tests

- Test unitaire : `RarityCode::try_new("special")` → `Ok(RarityCode::S)`
- Test unitaire : `RarityCode::try_new("Special")` → `Ok(RarityCode::S)` (case insensitive)
- Test unitaire : `RarityCode::S.to_string()` → `"S"`
- Test unitaire : `RarityCodeParam::S` → `RarityCode::S`
- Test intégration (collection/import) : CSV avec `Rarity=.special.` se parse correctement

## Critères d'acceptance

- [ ] `RarityCode::try_new("special")` retourne `Ok(RarityCode::S)`
- [ ] `RarityCode::try_new("Special")` retourne `Ok(RarityCode::S)` (case insensitive)
- [ ] `RarityCode::try_new("S")` retourne `Ok(RarityCode::S)`
- [ ] `RarityCode::S.to_string()` retourne `"S"`
- [ ] `RarityCodeParam::S` existe et convertit vers `RarityCode::S`
- [ ] L'endpoint `GET /collection?rarity=S` filtre correctement sur les cartes spéciales
- [ ] L'endpoint `GET /search/card?rarity=S` filtre correctement sur les cartes spéciales
- [ ] L'OpenAPI spec inclut `S` dans l'enum des codes de rareté
- [ ] Le frontend affiche "Special" pour le code `S`
- [ ] Le frontend permet de cocher/décocher le filtre "Special" dans `CollectionFilters` (positionné après Commune, en dernier)
- [ ] `mise run lint-backend` passe sans erreur
- [ ] `mise run lint-frontend` passe sans erreur
- [ ] `mise run format` passe sans erreur
