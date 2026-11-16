use crate::application::error::AppError;
use crate::application::repository::SearchRepository;
use crate::domain::pagination::Paginated;
use crate::domain::rarity_code::RarityCode;
use crate::domain::search::{Offer, Possession, SearchCard, SearchQuery, SearchScope};
use crate::domain::set_name::{SetCode, SetName};
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::entities::{
    SearchCardEntity, invalid_db_value,
};
use crate::infrastructure::adapter_out::repository::{add, repository_error};
use async_trait::async_trait;
use sqlx::postgres::PgArguments;
use sqlx::{AssertSqlSafe, Pool, Postgres, query_as_with, query_scalar_with};

/// The latest Cardmarket prices of the impression (`cc`), normal and foil. The latest price of
/// the Catalogue's Cardmarket id, not `mv_last_cardmarket_prices`: that view is built from `card`,
/// which an impression nobody owns is not in.
const PRICE_JOIN: &str = r#"LEFT JOIN LATERAL (SELECT low, avg, trend, low_foil, avg_foil, trend_foil
                                  FROM cardmarket_price
                                  WHERE id_produit = cc.cardmarket_id
                                  ORDER BY date DESC
                                  LIMIT 1) p ON TRUE"#;

/// The reference price of an impression: its normal trend price, or its foil one when it has
/// none. The sort and the price filters use this same price.
const REFERENCE_PRICE: &str = "COALESCE(p.trend, p.trend_foil)";

/// One image of the Cards of the impression (`cc`): the French one first, then the English one.
/// Read from `card` rather than from a view: an image shows up as soon as it is downloaded.
const IMAGE_JOIN: &str = r#"LEFT JOIN LATERAL (SELECT c.language_code, c.image_source, c.image_has_back
                                  FROM card c
                                  WHERE (c.set_code, c.collector_number) = (cc.set_code, cc.collector_number)
                                    AND c.language_code IN ('FR', 'EN')
                                    AND c.image_source IS NOT NULL
                                  ORDER BY c.language_code = 'FR' DESC
                                  LIMIT 1) img ON TRUE"#;

/// What the players offer of each impression: from `v_tradable_entry`, the only source of what a
/// player actually offers (see `.claude/instructions/database-schema.instructions.md`). With a
/// player (`player_idx`, the placeholder of their username), also whether they offer the
/// impression and whether one of their Cards of it is reserved.
fn offers_join(player_idx: Option<usize>) -> String {
    let player_columns = player_idx.map_or_else(String::new, |idx| {
        let player =
            format!("vte.user_id IN (SELECT id FROM users WHERE LOWER(username) = LOWER(${idx}))");
        format!(
            r#", bool_or({player}) AS by_player,
                 array_agg(DISTINCT vte.language_code::TEXT) FILTER (WHERE {player}) AS player_languages,
                 bool_or({player} AND EXISTS (
                     SELECT 1 FROM trade_card tc
                     JOIN trade t ON t.id = tc.trade_id
                     WHERE tc.set_code = vte.set_code AND tc.collector_number = vte.collector_number
                       AND tc.language_code = vte.language_code AND tc.foil = vte.foil
                       AND tc.owner_user_id = vte.user_id
                       AND t.status IN ('ONE_ACCEPTED', 'FULLY_ACCEPTED')
                 )) AS reserved"#
        )
    });
    format!(
        r#"LEFT JOIN (SELECT vte.set_code, vte.collector_number,
                            COUNT(DISTINCT vte.user_id) AS owner_count,
                            array_agg(DISTINCT vte.language_code::TEXT) AS languages{player_columns}
                     FROM v_tradable_entry vte
                     GROUP BY vte.set_code, vte.collector_number) o
                    ON (o.set_code, o.collector_number) = (cc.set_code, cc.collector_number)"#
    )
}

/// What `user_idx` (the placeholder of the user id) owns of each impression, every language and
/// finish together: the quantity of the searching player's own collection, whatever its binders.
fn owned_join(user_idx: usize) -> String {
    format!(
        r#"LEFT JOIN (SELECT ce.set_code, ce.collector_number, SUM(ce.quantity) AS owned_quantity
                     FROM collection_entry ce
                     WHERE ce.user_id = ${user_idx}
                     GROUP BY ce.set_code, ce.collector_number) own
                    ON (own.set_code, own.collector_number) = (cc.set_code, cc.collector_number)"#
    )
}

/// The filters of a search (text, Possession and Offre axes, rarity, sets, price range), as SQL
/// over `catalog_card cc`.
struct Filters<'a> {
    search_query: Option<&'a str>,
    /// The searching player, for the Possession axis and the owned quantity.
    user_id: &'a UserId,
    possession: Possession,
    offer: &'a Offer,
    rarity: &'a [RarityCode],
    sets: &'a [String],
    price_min: Option<u32>,
    price_max: Option<u32>,
}

/// The SQL of [`Filters::bind`].
struct Clauses {
    /// The joins the filters (and, for the rows, the result) need.
    joins: String,
    /// The `AND ...` conditions, empty without a filter.
    conditions: String,
    /// The `ORDER BY` prefix: the best name match first, with a text.
    order_prefix: String,
}

impl<'a> Filters<'a> {
    fn of(user_id: &'a UserId, query: &'a SearchQuery) -> Self {
        Self {
            search_query: query.search_query.as_deref(),
            user_id,
            possession: query.possession,
            offer: &query.offer,
            rarity: &query.rarity,
            sets: &query.sets,
            price_min: query.price_min,
            price_max: query.price_max,
        }
    }

    /// The scope alone: a facet must not shrink as the user filters.
    fn scope(user_id: &'a UserId, scope: &'a SearchScope) -> Self {
        Self {
            search_query: scope.search_query.as_deref(),
            user_id,
            possession: scope.possession,
            offer: &scope.offer,
            rarity: &[],
            sets: &[],
            price_min: None,
            price_max: None,
        }
    }

    /// Appends the filters' values to `args` and returns the matching SQL. Each placeholder is
    /// numbered after the value it was just bound to, so the clause and the binds can never
    /// drift apart, whatever was bound before. `with_result` also joins what the rows of the
    /// result carry (the offers and the prices), whatever the filters.
    fn bind(&self, args: &mut PgArguments, with_result: bool) -> Result<Clauses, AppError> {
        let mut conditions = Vec::new();
        let mut joins = Vec::new();
        let mut order_prefix = String::new();

        if let Some(q) = self.search_query {
            let idx = add(args, q)?;
            // The English or the French name, by substring or by trigram similarity.
            conditions.push(format!(
                "(cc.name ILIKE '%' || ${idx} || '%' OR ${idx} <% cc.name \
                  OR cc.french_name ILIKE '%' || ${idx} || '%' OR ${idx} <% cc.french_name)"
            ));
            order_prefix = format!(
                "GREATEST(COALESCE(word_similarity(${idx}, cc.name), 0), \
                          COALESCE(word_similarity(${idx}, cc.french_name), 0)) DESC,"
            );
        }

        if with_result || self.possession != Possession::Any {
            let idx = add(args, self.user_id.as_str())?;
            joins.push(owned_join(idx));
            match self.possession {
                Possession::Any => {}
                Possession::Owned => {
                    conditions.push("COALESCE(own.owned_quantity, 0) > 0".to_string())
                }
                Possession::NotOwned => {
                    conditions.push("COALESCE(own.owned_quantity, 0) = 0".to_string());
                }
            }
        }

        match self.offer {
            Offer::Any => {
                if with_result {
                    joins.push(offers_join(None));
                }
            }
            Offer::Offered => {
                joins.push(offers_join(None));
                conditions.push("o.owner_count > 0".to_string());
            }
            Offer::OfferedBy(username) => {
                let idx = add(args, username.as_str())?;
                joins.push(offers_join(Some(idx)));
                conditions.push("o.by_player".to_string());
            }
        }

        if !self.rarity.is_empty() {
            let rarity: Vec<String> = self.rarity.iter().map(ToString::to_string).collect();
            let idx = add(args, rarity)?;
            conditions.push(format!("cc.rarity = ANY(${idx})"));
        }
        if !self.sets.is_empty() {
            let idx = add(args, self.sets)?;
            conditions.push(format!("cc.set_code = ANY(${idx})"));
        }
        if let Some(v) = self.price_min {
            let idx = add(args, i64::from(v))?;
            conditions.push(format!("{REFERENCE_PRICE} >= ${idx}"));
        }
        if let Some(v) = self.price_max {
            let idx = add(args, i64::from(v))?;
            conditions.push(format!("{REFERENCE_PRICE} <= ${idx}"));
        }
        if with_result || self.price_min.is_some() || self.price_max.is_some() {
            joins.push(PRICE_JOIN.to_string());
        }

        Ok(Clauses {
            joins: joins.join("\n"),
            conditions: conditions
                .iter()
                .map(|c| format!("AND {c}"))
                .collect::<Vec<_>>()
                .join(" "),
            order_prefix,
        })
    }
}

pub struct SearchRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl SearchRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SearchRepository for SearchRepositoryAdapter {
    #[tracing::instrument(name = "search_repo.search_paginated", skip_all, fields(sentry.op = "db"))]
    async fn search_paginated(
        &self,
        user_id: &UserId,
        query: SearchQuery,
    ) -> Result<Paginated<SearchCard>, AppError> {
        let filters = Filters::of(user_id, &query);

        let mut args = PgArguments::default();
        let mut count_args = PgArguments::default();
        let limit_idx = add(&mut args, i64::from(query.pagination.limit()))?;
        let offset_idx = add(&mut args, i64::from(query.pagination.offset()))?;
        let rows = filters.bind(&mut args, true)?;
        let count = filters.bind(&mut count_args, false)?;

        // `reserved` only makes sense for one player: with the others, it stays absent.
        let reserved_column = match query.offer {
            Offer::OfferedBy(_) => "o.reserved",
            _ => "NULL::boolean",
        };
        // The languages offered within the Offre scope: the searched player's, or anybody's.
        let languages_column = match query.offer {
            Offer::OfferedBy(_) => "o.player_languages",
            _ => "o.languages",
        };

        let Clauses {
            joins,
            conditions,
            order_prefix,
        } = rows;
        let sql = format!(
            r#"SELECT
                 cc.set_code,
                 sn.name AS set_name,
                 cc.collector_number,
                 cc.name,
                 cc.french_name,
                 cc.rarity,
                 cc.scryfall_id,
                 cc.image_url,
                 img.language_code AS image_language,
                 img.image_source,
                 img.image_has_back,
                 COALESCE(o.owner_count, 0) AS owner_count,
                 COALESCE(own.owned_quantity, 0) AS owned_quantity,
                 {reserved_column} AS reserved,
                 COALESCE({languages_column}, ARRAY[]::TEXT[]) AS offered_languages,
                 p.low, p.avg, p.trend, p.low_foil, p.avg_foil, p.trend_foil
               FROM catalog_card cc
               JOIN set_name sn ON sn.set_code = cc.set_code
               {joins}
               {IMAGE_JOIN}
               WHERE 1 = 1
               {conditions}
               ORDER BY {order_prefix} {REFERENCE_PRICE} {} NULLS LAST,
                        cc.name, cc.set_code,
                        SUBSTRING(cc.collector_number FROM '^[0-9]+')::BIGINT NULLS LAST,
                        cc.collector_number
               LIMIT ${limit_idx} OFFSET ${offset_idx}"#,
            query.sort_dir,
        );

        let entities = query_as_with::<_, SearchCardEntity, _>(AssertSqlSafe(sql.as_str()), args)
            .fetch_all(&self.pool)
            .await
            .map_err(repository_error)?;

        let count_sql = format!(
            "SELECT COUNT(*) FROM catalog_card cc {} WHERE 1 = 1 {}",
            count.joins, count.conditions
        );

        let total: i64 = query_scalar_with(AssertSqlSafe(count_sql.as_str()), count_args)
            .fetch_one(&self.pool)
            .await
            .map_err(repository_error)?;

        Ok(Paginated {
            items: entities
                .into_iter()
                .map(SearchCard::try_from)
                .collect::<Result<_, _>>()?,
            total: total as u64,
            pagination: query.pagination,
        })
    }

    #[tracing::instrument(name = "search_repo.search_sets", skip_all, fields(sentry.op = "db"))]
    async fn search_sets(
        &self,
        user_id: &UserId,
        scope: &SearchScope,
    ) -> Result<Vec<SetName>, AppError> {
        let mut args = PgArguments::default();
        let Clauses {
            joins, conditions, ..
        } = Filters::scope(user_id, scope).bind(&mut args, false)?;

        // Same base as the search (the Catalogue, the same text and both axes), so a set is
        // listed exactly when the search can return one of its impressions.
        let sql = format!(
            r#"SELECT DISTINCT sn.set_code, sn.name
               FROM catalog_card cc
               JOIN set_name sn ON sn.set_code = cc.set_code
               {joins}
               WHERE 1 = 1 {conditions}
               ORDER BY sn.name, sn.set_code"#
        );

        let rows = query_as_with::<_, (String, String), _>(AssertSqlSafe(sql.as_str()), args)
            .fetch_all(&self.pool)
            .await
            .map_err(repository_error)?;

        rows.into_iter()
            .map(|(code, name)| {
                let set_code =
                    SetCode::try_new(&code).map_err(|_| invalid_db_value("set code", &code))?;
                Ok(SetName::new(set_code, name))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::card::CardId;
    use crate::domain::card_image::{CardImage, CardImageSource};
    use crate::domain::collection::SortDirection;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::pagination::Pagination;
    use crate::domain::search::{Possession, SearchImage};
    use crate::domain::user::UserId;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_catalog_card, insert_collection_entry,
        insert_collection_entry_with_binder, insert_price, insert_rarity_filter, insert_set,
        insert_trade, insert_trade_card, insert_trading_binder, insert_user_with_visibility,
    };
    use crate::infrastructure::adapter_out::repository::entities::{
        CardMarketPriceEntity, PriceGuideEntity,
    };
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

    /// A search by a player who owns nothing.
    async fn search(pool: &PgPool, query: SearchQuery) -> Paginated<SearchCard> {
        search_as(pool, "nobody", query).await
    }

    async fn search_as(pool: &PgPool, user_id: &str, query: SearchQuery) -> Paginated<SearchCard> {
        SearchRepositoryAdapter::new(pool.clone())
            .search_paginated(&UserId::new(user_id), query)
            .await
            .unwrap()
    }

    fn possession(possession: Possession) -> SearchQuery {
        SearchQuery {
            possession,
            ..SearchQuery::default()
        }
    }

    /// `user_id` owns `quantity` non-foil `language` copies of the impression, in the Catalogue
    /// already, without offering them (no public collection).
    async fn owns(
        pool: &PgPool,
        user_id: &str,
        number: &str,
        language: &str,
        foil: bool,
        quantity: i32,
    ) {
        let known: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM card WHERE (set_code, collector_number, language_code) = ('TST', $1, $2))",
        )
        .bind(number)
        .bind(language)
        .fetch_one(pool)
        .await
        .unwrap();
        if !known {
            insert_card(pool, "TST", number, language, "Card", 9_000).await;
        }
        insert_collection_entry(
            pool,
            "TST",
            number,
            language,
            foil,
            user_id,
            quantity,
            100,
            Utc::now(),
        )
        .await;
    }

    async fn private_user(pool: &PgPool, id: &str, username: &str) {
        insert_user_with_visibility(pool, id, username, "private").await;
    }

    fn names(result: &Paginated<SearchCard>) -> Vec<&str> {
        result.items.iter().map(|c| c.name.as_str()).collect()
    }

    fn text(q: &str) -> SearchQuery {
        SearchQuery {
            search_query: Some(q.to_string()),
            ..SearchQuery::default()
        }
    }

    fn offer(offer: Offer) -> SearchQuery {
        SearchQuery {
            offer,
            ..SearchQuery::default()
        }
    }

    /// A catalogue impression of `set_code` with no price, `C`ommon.
    async fn catalog(pool: &PgPool, set_code: &str, number: &str, name: &str) {
        insert_set_once(pool, set_code).await;
        insert_catalog_card(pool, set_code, number, name, None, None, "C").await;
    }

    async fn insert_set_once(pool: &PgPool, set_code: &str) {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM set_name WHERE set_code = $1)")
                .bind(set_code)
                .fetch_one(pool)
                .await
                .unwrap();
        if !exists {
            insert_set(pool, set_code).await;
        }
    }

    /// An impression of the Catalogue priced `trend` (normal) by Cardmarket product `product`.
    async fn priced(pool: &PgPool, number: &str, name: &str, product: i32, trend: i32) {
        insert_set_once(pool, "TST").await;
        insert_catalog_card(pool, "TST", number, name, None, Some(product), "C").await;
        insert_price(pool, CardMarketPriceEntity::simple(product, trend)).await;
    }

    async fn public_user(pool: &PgPool, id: &str, username: &str) {
        insert_user_with_visibility(pool, id, username, "public").await;
    }

    /// `user_id` owns, and so offers, one non-foil `language` Card of the impression, which is
    /// also in the Catalogue.
    async fn offers(pool: &PgPool, user_id: &str, set_code: &str, number: &str, language: &str) {
        let known: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM card WHERE (set_code, collector_number, language_code) = ($1, $2, $3))",
        )
        .bind(set_code)
        .bind(number)
        .bind(language)
        .fetch_one(pool)
        .await
        .unwrap();
        if !known {
            insert_card(pool, set_code, number, language, "Card", 9_000).await;
        }
        insert_collection_entry(
            pool,
            set_code,
            number,
            language,
            false,
            user_id,
            1,
            100,
            Utc::now(),
        )
        .await;
    }

    // --- Catalogue domain ---

    #[sqlx::test]
    async fn an_impression_nobody_owns_is_a_result(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_catalog_card(
            &pool,
            "TST",
            "7",
            "Sol Ring",
            Some("Anneau solaire"),
            None,
            "U",
        )
        .await;

        let result = search(&pool, SearchQuery::default()).await;

        assert_eq!(result.total, 1);
        let card = &result.items[0];
        assert_eq!(card.name, "Sol Ring");
        assert_eq!(card.french_name, Some("Anneau solaire".to_string()));
        assert_eq!(card.collector_number, "7");
        assert_eq!(card.set_name.code, SetCode::new("TST"));
        assert_eq!(card.set_name.name, "Set TST");
        assert_eq!(card.rarity_code, RarityCode::U);
        assert_eq!(card.owner_count, 0);
        assert_eq!(card.reserved, None);
        assert_eq!(card.price_guide, None);
        assert_eq!(card.foil_price_guide, None);
    }

    #[sqlx::test]
    async fn the_catalogue_is_empty_when_nothing_is_loaded(pool: PgPool) {
        let result = search(&pool, SearchQuery::default()).await;

        assert_eq!(result.total, 0);
        assert!(result.items.is_empty());
    }

    // --- Text ---

    #[sqlx::test]
    async fn a_french_name_finds_the_impression(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_catalog_card(
            &pool,
            "TST",
            "1",
            "Sol Ring",
            Some("Anneau solaire"),
            None,
            "U",
        )
        .await;
        insert_catalog_card(
            &pool,
            "TST",
            "2",
            "Counterspell",
            Some("Contresort"),
            None,
            "U",
        )
        .await;

        let result = search(&pool, text("Anneau solaire")).await;

        assert_eq!(names(&result), vec!["Sol Ring"]);
        assert_eq!(result.total, 1);
    }

    #[sqlx::test]
    async fn an_english_substring_finds_the_impression(pool: PgPool) {
        catalog(&pool, "TST", "1", "Lightning Bolt").await;
        catalog(&pool, "TST", "2", "Counterspell").await;

        let result = search(&pool, text("ning bo")).await;

        assert_eq!(names(&result), vec!["Lightning Bolt"]);
    }

    #[sqlx::test]
    async fn a_light_typo_still_finds_the_impression(pool: PgPool) {
        catalog(&pool, "TST", "1", "Lightning Bolt").await;
        catalog(&pool, "TST", "2", "Counterspell").await;

        let result = search(&pool, text("Lightnig Bolt")).await;

        assert_eq!(names(&result), vec!["Lightning Bolt"]);
    }

    #[sqlx::test]
    async fn the_text_does_not_target_the_set(pool: PgPool) {
        catalog(&pool, "ZZZ", "1", "Lightning Bolt").await;

        let result = search(&pool, text("Set ZZZ")).await;

        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn a_text_ranks_the_best_name_match_before_the_price(pool: PgPool) {
        // The longer name is the most expensive one, yet the exact match comes first.
        priced(&pool, "1", "Sol Ringleader", 1, 900).await;
        priced(&pool, "2", "Sol Ring", 2, 100).await;

        let result = search(&pool, text("Sol Ring")).await;

        assert_eq!(names(&result), vec!["Sol Ring", "Sol Ringleader"]);
    }

    // --- Offre axis ---

    #[sqlx::test]
    async fn the_offered_axis_keeps_what_players_offer(pool: PgPool) {
        catalog(&pool, "TST", "1", "Offered").await;
        catalog(&pool, "TST", "2", "Not offered").await;
        public_user(&pool, "userA", "Alice").await;
        offers(&pool, "userA", "TST", "1", "EN").await;

        let any = search(&pool, offer(Offer::Any)).await;
        let offered = search(&pool, offer(Offer::Offered)).await;

        assert_eq!(any.total, 2);
        assert_eq!(names(&offered), vec!["Offered"]);
        assert_eq!(offered.total, 1);
    }

    #[sqlx::test]
    async fn an_impression_offered_in_two_languages_by_two_players_is_one_result_with_two_owners(
        pool: PgPool,
    ) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userB", "Bob").await;
        offers(&pool, "userA", "TST", "1", "FR").await;
        offers(&pool, "userB", "TST", "1", "EN").await;

        let result = search(&pool, offer(Offer::Offered)).await;

        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].owner_count, 2);
    }

    #[sqlx::test]
    async fn a_player_offering_several_cards_of_an_impression_counts_once(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userA", "Alice").await;
        offers(&pool, "userA", "TST", "1", "FR").await;
        offers(&pool, "userA", "TST", "1", "EN").await;

        let result = search(&pool, offer(Offer::Offered)).await;

        assert_eq!(result.items[0].owner_count, 1);
    }

    #[sqlx::test]
    async fn the_owner_count_is_there_without_an_offer_filter(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userA", "Alice").await;
        offers(&pool, "userA", "TST", "1", "EN").await;

        let result = search(&pool, SearchQuery::default()).await;

        assert_eq!(result.items[0].owner_count, 1);
    }

    #[sqlx::test]
    async fn the_offered_languages_are_those_of_every_player_without_a_player_axis(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userB", "Bob").await;
        offers(&pool, "userA", "TST", "1", "FR").await;
        offers(&pool, "userB", "TST", "1", "EN").await;
        offers(&pool, "userB", "TST", "1", "FR").await;

        let result = search(&pool, SearchQuery::default()).await;

        assert_eq!(
            result.items[0].offered_languages,
            vec![LanguageCode::EN, LanguageCode::FR]
        );
    }

    #[sqlx::test]
    async fn the_offered_languages_are_those_of_the_searched_player_only(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userB", "Bob").await;
        offers(&pool, "userA", "TST", "1", "FR").await;
        offers(&pool, "userB", "TST", "1", "EN").await;

        let result = search(&pool, offer(Offer::OfferedBy("Bob".to_string()))).await;

        assert_eq!(result.items[0].offered_languages, vec![LanguageCode::EN]);
    }

    #[sqlx::test]
    async fn an_impression_nobody_offers_has_no_offered_language(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;

        let result = search(&pool, SearchQuery::default()).await;

        assert!(result.items[0].offered_languages.is_empty());
    }

    #[sqlx::test]
    async fn a_private_collection_offers_nothing(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        insert_user_with_visibility(&pool, "userA", "Alice", "private").await;
        offers(&pool, "userA", "TST", "1", "EN").await;

        let offered = search(&pool, offer(Offer::Offered)).await;
        let any = search(&pool, offer(Offer::Any)).await;

        assert_eq!(offered.total, 0);
        assert_eq!(any.items[0].owner_count, 0);
    }

    #[sqlx::test]
    async fn only_the_cards_of_trade_binders_and_open_rarities_are_offered(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_catalog_card(&pool, "TST", "1", "Open", None, None, "R").await;
        insert_catalog_card(&pool, "TST", "2", "Not in a binder", None, None, "R").await;
        insert_catalog_card(&pool, "TST", "3", "Closed rarity", None, None, "M").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "trade").await;
        insert_trading_binder(&pool, "userB", "Trade Binder").await;
        insert_rarity_filter(&pool, "userB", "R", true, 0).await;
        insert_rarity_filter(&pool, "userB", "M", false, 0).await;
        for (number, rarity, binder) in [
            ("1", "R", Some("Trade Binder")),
            ("2", "R", None),
            ("3", "M", Some("Trade Binder")),
        ] {
            sqlx::query(
                "INSERT INTO card (set_code, collector_number, language_code, name, rarity, scryfall_id, cardmarket_id) VALUES ('TST', $1, 'EN', 'Card', $2, $3, 1)",
            )
            .bind(number)
            .bind(rarity)
            .bind(Uuid::new_v4())
            .execute(&pool)
            .await
            .unwrap();
            insert_collection_entry_with_binder(
                &pool,
                "TST",
                number,
                "EN",
                false,
                "userB",
                1,
                100,
                Utc::now(),
                binder,
            )
            .await;
        }

        let result = search(&pool, offer(Offer::Offered)).await;

        assert_eq!(names(&result), vec!["Open"]);
    }

    #[sqlx::test]
    async fn offered_by_a_player_keeps_only_what_that_player_offers(pool: PgPool) {
        catalog(&pool, "TST", "1", "Alice only").await;
        catalog(&pool, "TST", "2", "Bob only").await;
        catalog(&pool, "TST", "3", "Both").await;
        catalog(&pool, "TST", "4", "Nobody").await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userB", "Bob").await;
        offers(&pool, "userA", "TST", "1", "EN").await;
        offers(&pool, "userB", "TST", "2", "EN").await;
        offers(&pool, "userA", "TST", "3", "EN").await;
        offers(&pool, "userB", "TST", "3", "EN").await;

        let result = search(&pool, offer(Offer::OfferedBy("Alice".to_string()))).await;

        assert_eq!(names(&result), vec!["Alice only", "Both"]);
        assert_eq!(result.total, 2);
        // The Owner count stays the number of players offering the impression.
        assert_eq!(result.items[1].owner_count, 2);
    }

    #[sqlx::test]
    async fn the_player_is_matched_by_exact_username_ignoring_case(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userA", "Alice").await;
        offers(&pool, "userA", "TST", "1", "EN").await;

        let exact = search(&pool, offer(Offer::OfferedBy("aLiCe".to_string()))).await;
        let partial = search(&pool, offer(Offer::OfferedBy("Ali".to_string()))).await;
        let unknown = search(&pool, offer(Offer::OfferedBy("Zed".to_string()))).await;

        assert_eq!(exact.total, 1);
        assert_eq!(partial.total, 0);
        assert_eq!(unknown.total, 0);
    }

    #[sqlx::test]
    async fn a_private_player_offers_nothing(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        insert_user_with_visibility(&pool, "userA", "Alice", "private").await;
        offers(&pool, "userA", "TST", "1", "EN").await;

        let result = search(&pool, offer(Offer::OfferedBy("Alice".to_string()))).await;

        assert_eq!(result.total, 0);
    }

    // --- Possession axis and owned quantity ---

    #[sqlx::test]
    async fn the_owned_quantity_sums_every_language_and_finish(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        private_user(&pool, "userA", "Alice").await;
        owns(&pool, "userA", "1", "FR", false, 2).await;
        owns(&pool, "userA", "1", "EN", false, 1).await;
        owns(&pool, "userA", "1", "EN", true, 3).await;

        let result = search_as(&pool, "userA", SearchQuery::default()).await;

        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].owned_quantity, 6);
    }

    #[sqlx::test]
    async fn an_impression_the_player_does_not_own_has_a_zero_quantity(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;

        let result = search_as(&pool, "userA", SearchQuery::default()).await;

        assert_eq!(result.items[0].owned_quantity, 0);
    }

    #[sqlx::test]
    async fn another_players_copies_are_never_in_the_owned_quantity(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userB", "Bob").await;
        owns(&pool, "userB", "1", "EN", false, 4).await;

        let any = search_as(&pool, "userA", SearchQuery::default()).await;
        let by_bob = search_as(&pool, "userA", offer(Offer::OfferedBy("Bob".to_string()))).await;

        assert_eq!(any.items[0].owned_quantity, 0);
        assert_eq!(by_bob.total, 1);
        assert_eq!(by_bob.items[0].owned_quantity, 0);
        assert_eq!(by_bob.items[0].owner_count, 1);
    }

    #[sqlx::test]
    async fn the_possession_axis_splits_the_catalogue_into_owned_and_not_owned(pool: PgPool) {
        catalog(&pool, "TST", "1", "Mine").await;
        catalog(&pool, "TST", "2", "Missing").await;
        private_user(&pool, "userA", "Alice").await;
        owns(&pool, "userA", "1", "EN", false, 1).await;

        let any = search_as(&pool, "userA", possession(Possession::Any)).await;
        let owned = search_as(&pool, "userA", possession(Possession::Owned)).await;
        let missing = search_as(&pool, "userA", possession(Possession::NotOwned)).await;

        assert_eq!(any.total, 2);
        assert_eq!(names(&owned), vec!["Mine"]);
        assert_eq!(owned.total, 1);
        assert_eq!(names(&missing), vec!["Missing"]);
        assert_eq!(missing.total, 1);
    }

    #[sqlx::test]
    async fn not_owned_and_offered_keep_what_the_player_lacks_and_somebody_offers(pool: PgPool) {
        catalog(&pool, "TST", "1", "Mine and offered").await;
        catalog(&pool, "TST", "2", "Missing and offered").await;
        catalog(&pool, "TST", "3", "Missing and not offered").await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userB", "Bob").await;
        owns(&pool, "userA", "1", "EN", false, 1).await;
        offers(&pool, "userB", "TST", "1", "EN").await;
        offers(&pool, "userB", "TST", "2", "EN").await;

        let result = search_as(
            &pool,
            "userA",
            SearchQuery {
                possession: Possession::NotOwned,
                offer: Offer::Offered,
                ..SearchQuery::default()
            },
        )
        .await;

        assert_eq!(names(&result), vec!["Missing and offered"]);
        assert_eq!(result.total, 1);
    }

    #[sqlx::test]
    async fn an_impression_owned_only_by_somebody_else_is_not_owned(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userB", "Bob").await;
        owns(&pool, "userB", "1", "EN", false, 1).await;

        let owned = search_as(&pool, "userA", possession(Possession::Owned)).await;
        let missing = search_as(&pool, "userA", possession(Possession::NotOwned)).await;

        assert_eq!(owned.total, 0);
        assert_eq!(missing.total, 1);
    }

    // --- reserved ---

    async fn reserve(pool: &PgPool, owner: &str, number: &str, language: &str) {
        let trade_id = Uuid::new_v4();
        insert_trade(pool, trade_id, owner, "userZ", "FULLY_ACCEPTED").await;
        insert_trade_card(pool, trade_id, "TST", number, language, false, owner, 1).await;
    }

    #[sqlx::test]
    async fn reserved_is_only_there_with_the_player_axis(pool: PgPool) {
        catalog(&pool, "TST", "1", "Reserved").await;
        catalog(&pool, "TST", "2", "Free").await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userZ", "Zed").await;
        offers(&pool, "userA", "TST", "1", "EN").await;
        offers(&pool, "userA", "TST", "2", "EN").await;
        reserve(&pool, "userA", "1", "EN").await;

        let by_player = search(&pool, offer(Offer::OfferedBy("Alice".to_string()))).await;
        let offered = search(&pool, offer(Offer::Offered)).await;
        let any = search(&pool, offer(Offer::Any)).await;

        assert_eq!(names(&by_player), vec!["Free", "Reserved"]);
        assert_eq!(by_player.items[0].reserved, Some(false));
        assert_eq!(by_player.items[1].reserved, Some(true));
        assert!(offered.items.iter().all(|c| c.reserved.is_none()));
        assert!(any.items.iter().all(|c| c.reserved.is_none()));
    }

    #[sqlx::test]
    async fn another_players_reservation_does_not_reserve_the_impression_of_the_searched_player(
        pool: PgPool,
    ) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userB", "Bob").await;
        public_user(&pool, "userZ", "Zed").await;
        offers(&pool, "userA", "TST", "1", "EN").await;
        offers(&pool, "userB", "TST", "1", "EN").await;
        reserve(&pool, "userB", "1", "EN").await;

        let result = search(&pool, offer(Offer::OfferedBy("Alice".to_string()))).await;

        assert_eq!(result.items[0].reserved, Some(false));
    }

    // --- Reference price: sort and filter ---

    async fn foil_only(pool: &PgPool, number: &str, name: &str, product: i32, trend_foil: i32) {
        insert_set_once(pool, "TST").await;
        insert_catalog_card(pool, "TST", number, name, None, Some(product), "C").await;
        insert_price(
            pool,
            CardMarketPriceEntity {
                normal: PriceGuideEntity::empty(),
                foil: PriceGuideEntity {
                    low: Some(trend_foil / 2),
                    avg: Some(trend_foil),
                    trend: Some(trend_foil),
                },
                ..CardMarketPriceEntity::simple(product, 0)
            },
        )
        .await;
    }

    #[sqlx::test]
    async fn without_a_text_the_most_expensive_come_first_and_the_unpriced_last(pool: PgPool) {
        priced(&pool, "1", "Cheap", 1, 100).await;
        priced(&pool, "2", "Dear", 2, 900).await;
        catalog(&pool, "TST", "3", "Unpriced").await;

        let result = search(&pool, SearchQuery::default()).await;

        assert_eq!(names(&result), vec!["Dear", "Cheap", "Unpriced"]);
    }

    #[sqlx::test]
    async fn the_sort_direction_reverses_the_price_order(pool: PgPool) {
        priced(&pool, "1", "Cheap", 1, 100).await;
        priced(&pool, "2", "Dear", 2, 900).await;
        catalog(&pool, "TST", "3", "Unpriced").await;

        let result = search(
            &pool,
            SearchQuery {
                sort_dir: SortDirection::Asc,
                ..SearchQuery::default()
            },
        )
        .await;

        assert_eq!(names(&result), vec!["Cheap", "Dear", "Unpriced"]);
    }

    #[sqlx::test]
    async fn a_foil_only_impression_is_sorted_on_its_foil_trend(pool: PgPool) {
        priced(&pool, "1", "Normal 500", 1, 500).await;
        foil_only(&pool, "2", "Foil 800", 2, 800).await;
        priced(&pool, "3", "Normal 100", 3, 100).await;

        let result = search(&pool, SearchQuery::default()).await;

        assert_eq!(names(&result), vec!["Foil 800", "Normal 500", "Normal 100"]);
    }

    #[sqlx::test]
    async fn a_foil_only_impression_is_filtered_on_its_foil_trend(pool: PgPool) {
        priced(&pool, "1", "Normal 500", 1, 500).await;
        foil_only(&pool, "2", "Foil 800", 2, 800).await;
        priced(&pool, "3", "Normal 100", 3, 100).await;

        let dear = search(
            &pool,
            SearchQuery {
                price_min: Some(600),
                ..SearchQuery::default()
            },
        )
        .await;
        let cheap = search(
            &pool,
            SearchQuery {
                price_max: Some(500),
                ..SearchQuery::default()
            },
        )
        .await;

        assert_eq!(names(&dear), vec!["Foil 800"]);
        assert_eq!(dear.total, 1);
        assert_eq!(names(&cheap), vec!["Normal 500", "Normal 100"]);
    }

    #[sqlx::test]
    async fn the_normal_trend_is_the_reference_price_when_there_is_one(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_catalog_card(&pool, "TST", "1", "Both", None, Some(1), "C").await;
        insert_price(&pool, CardMarketPriceEntity::with_foil(1, 100, 9_000)).await;

        let normal_range = search(
            &pool,
            SearchQuery {
                price_max: Some(200),
                ..SearchQuery::default()
            },
        )
        .await;

        assert_eq!(normal_range.total, 1);
        let card = &normal_range.items[0];
        assert_eq!(card.price_guide.as_ref().unwrap().trend.value, Some(100));
        assert_eq!(
            card.foil_price_guide.as_ref().unwrap().trend.value,
            Some(9_000)
        );
    }

    #[sqlx::test]
    async fn the_latest_price_of_the_product_is_used(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_catalog_card(&pool, "TST", "1", "Card", None, Some(1), "C").await;
        let yesterday = chrono::Local::now().date_naive().pred_opt().unwrap();
        insert_price(&pool, CardMarketPriceEntity::simple_at(1, yesterday, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 300)).await;

        let result = search(&pool, SearchQuery::default()).await;

        assert_eq!(
            result.items[0].price_guide.as_ref().unwrap().trend.value,
            Some(300)
        );
    }

    // --- Other filters and pagination ---

    #[sqlx::test]
    async fn the_rarity_and_set_filters_narrow_the_result(pool: PgPool) {
        insert_set(&pool, "AAA").await;
        insert_set(&pool, "BBB").await;
        insert_catalog_card(&pool, "AAA", "1", "AAA common", None, None, "C").await;
        insert_catalog_card(&pool, "AAA", "2", "AAA rare", None, None, "R").await;
        insert_catalog_card(&pool, "BBB", "1", "BBB rare", None, None, "R").await;

        let rare = search(
            &pool,
            SearchQuery {
                rarity: vec![RarityCode::R],
                ..SearchQuery::default()
            },
        )
        .await;
        let rare_in_bbb = search(
            &pool,
            SearchQuery {
                rarity: vec![RarityCode::R],
                sets: vec!["BBB".to_string()],
                ..SearchQuery::default()
            },
        )
        .await;

        assert_eq!(rare.total, 2);
        assert_eq!(names(&rare_in_bbb), vec!["BBB rare"]);
    }

    #[sqlx::test]
    async fn the_page_and_the_total_are_independent(pool: PgPool) {
        for i in 1..=5 {
            priced(&pool, &i.to_string(), &format!("Card {i}"), i, i * 100).await;
        }

        let result = search(
            &pool,
            SearchQuery {
                pagination: Pagination::try_new(1, 2, 100).unwrap(),
                ..SearchQuery::default()
            },
        )
        .await;

        assert_eq!(result.total, 5);
        assert_eq!(names(&result), vec!["Card 3", "Card 2"]);
    }

    fn numbers(result: &Paginated<SearchCard>) -> Vec<&str> {
        result
            .items
            .iter()
            .map(|c| c.collector_number.as_str())
            .collect()
    }

    #[sqlx::test]
    async fn impressions_tied_on_everything_else_come_in_collector_number_order(pool: PgPool) {
        // Compared as text, "10" would come before "2".
        for number in ["10", "2", "1b", "1"] {
            catalog(&pool, "TST", number, "Bolt").await;
        }

        let result = search(&pool, SearchQuery::default()).await;

        assert_eq!(numbers(&result), vec!["1", "1b", "2", "10"]);
    }

    #[sqlx::test]
    async fn the_page_is_a_window_of_the_results_of_a_text(pool: PgPool) {
        for number in 1..=5 {
            catalog(&pool, "TST", &number.to_string(), "Bolt").await;
        }
        catalog(&pool, "TST", "6", "Island").await;

        let result = search(
            &pool,
            SearchQuery {
                pagination: Pagination::try_new(1, 2, 100).unwrap(),
                ..text("Bolt")
            },
        )
        .await;

        assert_eq!(result.total, 5);
        assert_eq!(numbers(&result), vec!["3", "4"]);
    }

    // --- Image ---

    async fn downloaded(pool: &PgPool, language: &str, source: &str, has_back: bool) {
        sqlx::query(
            "UPDATE card SET image_source = $1, image_has_back = $2 WHERE language_code = $3",
        )
        .bind(source)
        .bind(has_back)
        .bind(language)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn image_after(pool: &PgPool, languages: &[&str]) -> Option<SearchImage> {
        catalog(pool, "TST", "1", "Sol Ring").await;
        for language in languages {
            insert_card(pool, "TST", "1", language, "Sol Ring", 1).await;
        }
        search(pool, SearchQuery::default())
            .await
            .items
            .remove(0)
            .image
    }

    fn local(
        language: LanguageCode,
        source: CardImageSource,
        has_back: bool,
    ) -> Option<SearchImage> {
        Some(SearchImage::Local {
            card_id: CardId {
                set_code: SetCode::new("TST"),
                collector_number: "1".to_string(),
                language_code: language,
            },
            image: CardImage { source, has_back },
        })
    }

    #[sqlx::test]
    async fn the_image_is_the_scryfall_one_of_the_catalogue_without_a_downloaded_image(
        pool: PgPool,
    ) {
        let image = image_after(&pool, &["FR", "EN"]).await;

        assert_eq!(
            image,
            Some(SearchImage::Scryfall(
                "https://img.example/TST/1.jpg".to_string()
            ))
        );
    }

    #[sqlx::test]
    async fn the_image_is_the_english_one_when_no_french_one_is_downloaded(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        insert_card(&pool, "TST", "1", "FR", "Sol Ring", 1).await;
        insert_card(&pool, "TST", "1", "EN", "Sol Ring", 1).await;
        downloaded(&pool, "EN", "gatherer_localized", true).await;

        let image = search(&pool, SearchQuery::default())
            .await
            .items
            .remove(0)
            .image;

        assert_eq!(
            image,
            local(LanguageCode::EN, CardImageSource::GathererLocalized, true)
        );
    }

    #[sqlx::test]
    async fn the_french_image_wins_over_the_english_one(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        insert_card(&pool, "TST", "1", "FR", "Sol Ring", 1).await;
        insert_card(&pool, "TST", "1", "EN", "Sol Ring", 1).await;
        downloaded(&pool, "EN", "gatherer_localized", false).await;
        downloaded(&pool, "FR", "gatherer_localized", false).await;

        let image = search(&pool, SearchQuery::default())
            .await
            .items
            .remove(0)
            .image;

        assert_eq!(
            image,
            local(LanguageCode::FR, CardImageSource::GathererLocalized, false)
        );
    }

    #[sqlx::test]
    async fn an_image_of_another_language_than_french_or_english_is_not_used(pool: PgPool) {
        catalog(&pool, "TST", "1", "Sol Ring").await;
        insert_card(&pool, "TST", "1", "DE", "Sol Ring", 1).await;
        downloaded(&pool, "DE", "gatherer_localized", false).await;

        let image = search(&pool, SearchQuery::default())
            .await
            .items
            .remove(0)
            .image;

        assert_eq!(
            image,
            Some(SearchImage::Scryfall(
                "https://img.example/TST/1.jpg".to_string()
            ))
        );
    }

    // --- Sets facet ---

    async fn sets(pool: &PgPool, scope: SearchScope) -> Vec<String> {
        sets_as(pool, "nobody", scope).await
    }

    async fn sets_as(pool: &PgPool, user_id: &str, scope: SearchScope) -> Vec<String> {
        SearchRepositoryAdapter::new(pool.clone())
            .search_sets(&UserId::new(user_id), &scope)
            .await
            .unwrap()
            .into_iter()
            .map(|s| s.code.to_string())
            .collect()
    }

    async fn two_sets(pool: &PgPool) {
        insert_set(pool, "ZZZ").await;
        insert_set(pool, "AAA").await;
        // `Set AAA` sorts before `Set ZZZ`: the facet is sorted by name.
        insert_catalog_card(
            pool,
            "ZZZ",
            "1",
            "Sol Ring",
            Some("Anneau solaire"),
            None,
            "C",
        )
        .await;
        insert_catalog_card(pool, "AAA", "1", "Counterspell", None, None, "C").await;
    }

    #[sqlx::test]
    async fn the_facet_lists_every_set_of_the_catalogue_by_name(pool: PgPool) {
        two_sets(&pool).await;

        assert_eq!(
            sets(&pool, SearchScope::default()).await,
            vec!["AAA", "ZZZ"]
        );
    }

    #[sqlx::test]
    async fn the_facet_follows_the_text_in_french(pool: PgPool) {
        two_sets(&pool).await;

        let scope = SearchScope {
            search_query: Some("Anneau solaire".to_string()),
            ..SearchScope::default()
        };

        assert_eq!(sets(&pool, scope).await, vec!["ZZZ"]);
    }

    #[sqlx::test]
    async fn the_facet_follows_the_offre_axis(pool: PgPool) {
        two_sets(&pool).await;
        public_user(&pool, "userA", "Alice").await;
        public_user(&pool, "userB", "Bob").await;
        offers(&pool, "userA", "ZZZ", "1", "EN").await;
        offers(&pool, "userB", "AAA", "1", "EN").await;

        let offered = sets(
            &pool,
            SearchScope {
                offer: Offer::Offered,
                ..SearchScope::default()
            },
        )
        .await;
        let by_alice = sets(
            &pool,
            SearchScope {
                offer: Offer::OfferedBy("alice".to_string()),
                ..SearchScope::default()
            },
        )
        .await;

        assert_eq!(offered, vec!["AAA", "ZZZ"]);
        assert_eq!(by_alice, vec!["ZZZ"]);
    }

    #[sqlx::test]
    async fn the_facet_follows_the_possession_axis_combined_with_the_offre_axis(pool: PgPool) {
        // TST: owned by me and offered by Bob; AAA: offered by Bob; MMM: neither.
        catalog(&pool, "TST", "1", "Sol Ring").await;
        catalog(&pool, "AAA", "1", "Counterspell").await;
        catalog(&pool, "MMM", "1", "Island").await;
        public_user(&pool, "userB", "Bob").await;
        offers(&pool, "userB", "TST", "1", "EN").await;
        offers(&pool, "userB", "AAA", "1", "EN").await;
        private_user(&pool, "me", "Me").await;
        owns(&pool, "me", "1", "EN", false, 1).await;

        let scope = |possession, offer| SearchScope {
            possession,
            offer,
            ..SearchScope::default()
        };

        assert_eq!(
            sets_as(&pool, "me", scope(Possession::Owned, Offer::Any)).await,
            vec!["TST"]
        );
        assert_eq!(
            sets_as(&pool, "me", scope(Possession::NotOwned, Offer::Any)).await,
            vec!["AAA", "MMM"]
        );
        assert_eq!(
            sets_as(&pool, "me", scope(Possession::NotOwned, Offer::Offered)).await,
            vec!["AAA"]
        );
    }
}
