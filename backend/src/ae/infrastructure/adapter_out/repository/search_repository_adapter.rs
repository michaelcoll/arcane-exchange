use crate::application::error::AppError;
use crate::application::repository::SearchRepository;
use crate::domain::pagination::Paginated;
use crate::domain::rarity_code::RarityCode;
use crate::domain::search::{SearchCard, SearchQuery, SearchScope, SearchSortField};
use crate::domain::set_name::{SetCode, SetName};
use crate::infrastructure::adapter_out::repository::entities::{
    SearchCardEntity, invalid_db_value,
};
use crate::infrastructure::adapter_out::repository::{add, repository_error};
use async_trait::async_trait;
use sqlx::postgres::PgArguments;
use sqlx::{AssertSqlSafe, Pool, Postgres, query_as_with, query_scalar_with};

/// Restricts the search and its facets to the cards actually offered for trade — see
/// `.agents/database-schema.instructions.md`.
const TRADABLE_JOIN: &str = r#"JOIN v_tradable_entry vte ON (vte.user_id, vte.set_code, vte.collector_number, vte.language_code, vte.foil) =
                                             (cp.user_id, cp.set_code, cp.collector_number, cp.language_code, cp.foil)"#;

/// The filters of a search (text, rarity, sets, price range, player), as SQL over
/// `mv_card_prices cp`.
#[derive(Default)]
struct Filters<'a> {
    search_query: Option<&'a str>,
    rarity: &'a [RarityCode],
    sets: &'a [String],
    price_min: Option<u32>,
    price_max: Option<u32>,
    player_username: Option<&'a str>,
}

impl<'a> Filters<'a> {
    fn of(query: &'a SearchQuery) -> Self {
        Self {
            search_query: query.search_query.as_deref(),
            rarity: &query.rarity,
            sets: &query.sets,
            price_min: query.price_min,
            price_max: query.price_max,
            player_username: query.player_username.as_deref(),
        }
    }

    /// The scope alone: a facet must not shrink as the user filters.
    fn scope(scope: &'a SearchScope) -> Self {
        Self {
            search_query: scope.search_query.as_deref(),
            player_username: scope.player_username.as_deref(),
            ..Self::default()
        }
    }

    /// Appends the filters' values to `args` and returns the matching `AND ...` clause and
    /// `ORDER BY` prefix. Each placeholder is numbered after the value it was just bound to, so
    /// the clause and the binds can never drift apart, whatever was bound before.
    fn bind(&self, args: &mut PgArguments) -> Result<(String, String), AppError> {
        let mut conditions = Vec::new();
        let mut order_prefix = String::new();

        if let Some(q) = self.search_query {
            let idx = add(args, q)?;
            conditions.push(format!(
                "(cp.name ILIKE '%' || ${idx} || '%' OR ${idx} <% cp.name)"
            ));
            order_prefix = format!("word_similarity(${idx}, cp.name) DESC,");
        }
        if !self.rarity.is_empty() {
            let rarity: Vec<String> = self.rarity.iter().map(ToString::to_string).collect();
            let idx = add(args, rarity)?;
            conditions.push(format!("cp.rarity = ANY(${idx})"));
        }
        if !self.sets.is_empty() {
            let idx = add(args, self.sets)?;
            conditions.push(format!("cp.set_code = ANY(${idx})"));
        }
        if let Some(v) = self.price_min {
            let idx = add(args, i64::from(v))?;
            conditions.push(format!("cp.trend >= ${idx}"));
        }
        if let Some(v) = self.price_max {
            let idx = add(args, i64::from(v))?;
            conditions.push(format!("cp.trend <= ${idx}"));
        }
        if let Some(username) = self.player_username {
            let idx = add(args, username)?;
            conditions.push(format!(
                "cp.user_id IN (SELECT id FROM users WHERE LOWER(username) = LOWER(${idx}))"
            ));
        }

        let filter_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("AND {}", conditions.join(" AND "))
        };

        Ok((filter_clause, order_prefix))
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
        query: SearchQuery,
    ) -> Result<Paginated<SearchCard>, AppError> {
        let filters = Filters::of(&query);

        let mut args = PgArguments::default();
        let mut count_args = PgArguments::default();
        let limit_idx = add(&mut args, i64::from(query.pagination.limit()))?;
        let offset_idx = add(&mut args, i64::from(query.pagination.offset()))?;
        let (filter_clause, order_prefix) = filters.bind(&mut args)?;
        let (count_filter_clause, _) = filters.bind(&mut count_args)?;

        // `reserved` is only meaningful when the result set can't mix cards from several
        // owners: a search scoped to a single real owner via `player_username`. In the unscoped
        // multi-owner search, which owner a row's `reserved` would refer to is ambiguous, so it
        // stays `false`.
        let reserved_column = if query.player_username.is_some() {
            r#"bool_or(EXISTS (
                     SELECT 1 FROM trade_card tc
                     JOIN trade t ON t.id = tc.trade_id
                     WHERE tc.set_code = cp.set_code AND tc.collector_number = cp.collector_number
                       AND tc.language_code = cp.language_code AND tc.foil = cp.foil
                       AND tc.owner_user_id = cp.user_id
                       AND t.status IN ('ONE_ACCEPTED', 'FULLY_ACCEPTED')
                 )) AS reserved"#
        } else {
            "false AS reserved"
        };

        // `added_at` is never part of the `SELECT`/`GROUP BY`, so a bare `added_at` in `ORDER BY`
        // would not be the real per-row value. `MAX(cp.added_at)` is a valid aggregate regardless
        // of `GROUP BY` membership.
        //
        // Note that `MAX` here is purely a SQL-validity device, not a semantic choice: the
        // unique index on `mv_card_prices` guarantees at most one row per (card, user) once
        // `player_username` scopes the search, so the aggregate never actually picks among
        // several values. The value it sees is already `MIN(added_at)` across a player's
        // binders for that card, computed upstream by the view (see
        // `migrations/0019_add_binder_name_to_collection_entry.sql`) — so sorting by `added_at`
        // orders by each card's *earliest* addition among binders, not its most recent one.
        let sort_expr = if query.sort_by == SearchSortField::AddedAt {
            "MAX(cp.added_at)".to_string()
        } else {
            query.sort_by.to_string()
        };

        let sql = format!(
            r#"SELECT
                 cp.set_code,
                 sn.name AS set_name,
                 cp.collector_number,
                 cp.language_code,
                 cp.foil,
                 cp.name,
                 cp.rarity,
                 cp.scryfall_id,
                 c.image_source,
                 c.image_has_back,
                 COUNT(DISTINCT cp.user_id) AS owner_count,
                 {reserved_column},
                 cp.avg,
                 cp.low,
                 cp.trend
               FROM mv_card_prices cp
               JOIN set_name sn ON sn.set_code = cp.set_code
               -- Read from `card` rather than the view, which is only refreshed with the prices:
               -- an image shows up as soon as it is downloaded.
               JOIN card c ON (c.set_code, c.collector_number, c.language_code) =
                              (cp.set_code, cp.collector_number, cp.language_code)
               {TRADABLE_JOIN}
               WHERE 1 = 1
               {filter_clause}
               GROUP BY cp.set_code, sn.name, cp.collector_number, cp.language_code,
                        cp.foil, cp.name, cp.rarity, cp.scryfall_id, c.image_source,
                        c.image_has_back, cp.avg, cp.low, cp.trend
               ORDER BY {order_prefix} {sort_expr} {} NULLS LAST, cp.name
               LIMIT ${limit_idx} OFFSET ${offset_idx}"#,
            query.sort_dir,
        );

        let entities = query_as_with::<_, SearchCardEntity, _>(AssertSqlSafe(sql.as_str()), args)
            .fetch_all(&self.pool)
            .await
            .map_err(repository_error)?;

        let count_sql = format!(
            r#"SELECT COUNT(*) FROM (
                 SELECT 1 FROM mv_card_prices cp {TRADABLE_JOIN} WHERE 1 = 1 {count_filter_clause}
                 GROUP BY cp.set_code, cp.collector_number, cp.language_code, cp.foil
               ) sub"#
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
    async fn search_sets(&self, scope: &SearchScope) -> Result<Vec<SetName>, AppError> {
        let mut args = PgArguments::default();
        let (filter_clause, _) = Filters::scope(scope).bind(&mut args)?;

        // Same base as the search (`mv_card_prices` + `v_tradable_entry`), so a set is listed
        // exactly when the search can return one of its cards.
        let sql = format!(
            r#"SELECT DISTINCT sn.set_code, sn.name
               FROM mv_card_prices cp
               JOIN set_name sn ON sn.set_code = cp.set_code
               {TRADABLE_JOIN}
               WHERE 1 = 1 {filter_clause}
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
    use crate::application::service::search::search_cards::SEARCH_MAX_OFFSET;
    use crate::domain::card_image::{CardImage, CardImageSource};
    use crate::domain::collection::SortDirection;
    use crate::domain::pagination::Pagination;
    use crate::domain::rarity_code::RarityCode;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_card_with_rarity, insert_collection_entry,
        insert_collection_entry_with_binder, insert_price, insert_rarity_filter, insert_set,
        insert_trading_binder, insert_user, insert_user_with_visibility, refresh_view,
    };
    use crate::infrastructure::adapter_out::repository::entities::CardMarketPriceEntity;
    use chrono::{DateTime, Utc};
    use sqlx::PgPool;

    #[sqlx::test]
    async fn search_paginated_returns_cards_from_all_users(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", "Test Card", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", "Test Card", 2).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 200)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
    }

    async fn record_image_after_refresh(pool: &PgPool) {
        sqlx::query("UPDATE card SET image_source = 'gatherer_en', image_has_back = TRUE")
            .execute(pool)
            .await
            .unwrap();
    }

    const DOWNLOADED_IMAGE: Option<CardImage> = Some(CardImage {
        source: CardImageSource::GathererEn,
        has_back: true,
    });

    #[sqlx::test]
    async fn search_paginated_reads_an_image_downloaded_since_the_last_refresh(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "FR", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_collection_entry(&pool, "TST", "1", "FR", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "FR", false, "userB", 1, 100, Utc::now()).await;
        refresh_view(&pool).await;
        record_image_after_refresh(&pool).await;

        let result = SearchRepositoryAdapter::new(pool)
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].image, DOWNLOADED_IMAGE);
    }

    #[sqlx::test]
    async fn search_paginated_returns_empty_when_no_cards(pool: PgPool) {
        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn search_paginated_masks_purchase_price_and_added_at_for_every_user(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 3, 1500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].owner_count, 1);
        assert!(!result.items[0].reserved);
    }

    #[sqlx::test]
    async fn search_paginated_masks_purchase_price_even_for_the_searching_users_own_card(
        pool: PgPool,
    ) {
        // search_paginated has no notion of "the current user": it reports nothing but the
        // owner count, even for the card of the user running the search.
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 3, 1500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].owner_count, 1);
        assert!(!result.items[0].reserved);
    }

    #[sqlx::test]
    async fn search_paginated_same_card_owned_by_three_users_appears_once_with_owner_count_three(
        pool: PgPool,
    ) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_user_with_visibility(&pool, "userC", "Carol", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userC", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].owner_count, 3);
        assert!(!result.items[0].reserved);
    }

    #[sqlx::test]
    async fn search_paginated_filters_by_search_query_fuzzy(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Goblin Guide", 1).await;
        insert_card(&pool, "TST", "2", "EN", "Sol Ring", 2).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            search_query: Some("gob".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Goblin Guide");
    }

    #[sqlx::test]
    async fn search_paginated_filters_by_rarity(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_card_with_rarity;

        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "EN", "Common Card", 1, "C").await;
        insert_card_with_rarity(&pool, "TST", "2", "EN", "Mythic Card", 2, "M").await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            rarity: vec![RarityCode::M],
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Mythic Card");
    }

    #[sqlx::test]
    async fn search_paginated_respects_page_size(pool: PgPool) {
        for i in 1..=5i32 {
            let set = format!("TS{}", i);
            insert_set(&pool, &set).await;
            insert_card(&pool, &set, "1", "EN", "Test Card", i).await;
            insert_user_with_visibility(
                &pool,
                &format!("user{}", i),
                &format!("User{}", i),
                "public",
            )
            .await;
            insert_collection_entry(
                &pool,
                &set,
                "1",
                "EN",
                false,
                &format!("user{}", i),
                1,
                100,
                Utc::now(),
            )
            .await;
            insert_price(&pool, CardMarketPriceEntity::simple(i, i * 100)).await;
        }
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            pagination: Pagination::try_new(0, 2, SEARCH_MAX_OFFSET).unwrap(),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);
        assert_eq!(result.pagination.page_size(), 2);
    }

    #[sqlx::test]
    async fn search_paginated_filters_by_player_username_exact_match(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", "Card A", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", "Card B", 2).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].name, "Card A");
    }

    #[sqlx::test]
    async fn search_paginated_player_username_is_case_insensitive(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Card A", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            player_username: Some("ALICE".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Card A");
    }

    #[sqlx::test]
    async fn search_paginated_player_username_requires_exact_match_no_partial(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Card A", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            player_username: Some("ali".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn search_paginated_returns_empty_for_unknown_player_username(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Card A", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            player_username: Some("unknown-user".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn search_paginated_sorts_by_added_at_for_scoped_player_ascending(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", "Card A", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", "Card B", 2).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        let older = Utc::now() - chrono::Duration::days(2);
        let older = DateTime::from_timestamp_micros(older.timestamp_micros()).unwrap();
        let newer = Utc::now();
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "userA", 1, 100, newer).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "userA", 1, 100, older).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            sort_by: SearchSortField::AddedAt,
            sort_dir: SortDirection::Asc,
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result.items[0].name, "Card B",
            "the older entry sorts first"
        );
        assert_eq!(result.items[1].name, "Card A");
    }

    #[sqlx::test]
    async fn search_paginated_sorts_by_added_at_for_scoped_player_descending(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", "Card A", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", "Card B", 2).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        // Card B is the newer entry here (unlike the ascending test above), so the
        // alphabetical name tie-break would put Card A first if the `added_at` sort were a
        // no-op — this test only passes if the sort genuinely orders by date.
        let older = Utc::now() - chrono::Duration::days(2);
        let older = DateTime::from_timestamp_micros(older.timestamp_micros()).unwrap();
        let newer = Utc::now();
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "userA", 1, 100, older).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "userA", 1, 100, newer).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            sort_by: SearchSortField::AddedAt,
            sort_dir: SortDirection::Desc,
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result.items[0].name, "Card B",
            "the newer entry sorts first"
        );
        assert_eq!(result.items[1].name, "Card A");
    }

    #[sqlx::test]
    async fn search_paginated_text_relevance_outranks_added_at_sort(pool: PgPool) {
        // Both cards match the `q` filter (name contains "Foo"), but "Foo" is an exact match
        // (word_similarity 1.0) while "Foobar" is only a partial match. "Foo" is also the
        // *older* entry and `sort_dir=desc` is requested: if `added_at` took priority over
        // text relevance, "Foobar" (newer) would sort first. It must not.
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", "Foo", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", "Foobar", 2).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        let older = Utc::now() - chrono::Duration::days(2);
        let older = DateTime::from_timestamp_micros(older.timestamp_micros()).unwrap();
        let newer = Utc::now();
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "userA", 1, 100, older).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "userA", 1, 100, newer).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            search_query: Some("Foo".to_string()),
            sort_by: SearchSortField::AddedAt,
            sort_dir: SortDirection::Desc,
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result.items[0].name, "Foo",
            "the exact text match must outrank the more recent added_at"
        );
        assert_eq!(result.items[1].name, "Foobar");
    }

    #[sqlx::test]
    async fn search_paginated_added_at_stays_masked_when_sorted_by_added_at(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Card A", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            sort_by: SearchSortField::AddedAt,
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 1);
        // `SearchCard` structurally has no `added_at` field: the sort uses the real value
        // internally without ever exposing it.
        assert_eq!(result.items[0].owner_count, 1);
        assert!(!result.items[0].reserved);
    }

    #[sqlx::test]
    async fn search_paginated_returns_empty_for_unknown_player_username_with_added_at_sort(
        pool: PgPool,
    ) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Card A", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            sort_by: SearchSortField::AddedAt,
            player_username: Some("unknown-user".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn search_paginated_player_username_forces_owner_count_to_one_even_with_multiple_real_owners(
        pool: PgPool,
    ) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_user_with_visibility(&pool, "userC", "Carol", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userC", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].owner_count, 1);
        assert!(!result.items[0].reserved);
    }

    #[sqlx::test]
    async fn search_paginated_player_username_reflects_the_scoped_owners_reservation(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::{
            insert_trade, insert_trade_card,
        };

        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "userA", "userB", "FULLY_ACCEPTED").await;
        insert_trade_card(&pool, trade_id, "TST", "1", "EN", false, "userA", 1).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].owner_count, 1);
        assert!(result.items[0].reserved);
    }

    #[sqlx::test]
    async fn search_paginated_unscoped_multi_owner_never_reports_reserved(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::{
            insert_trade, insert_trade_card,
        };

        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "userA", "userB", "FULLY_ACCEPTED").await;
        insert_trade_card(&pool, trade_id, "TST", "1", "EN", false, "userA", 1).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].owner_count, 1);
        assert!(!result.items[0].reserved);
    }

    #[sqlx::test]
    async fn search_paginated_player_username_combines_with_other_filters(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_card_with_rarity;

        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "EN", "Common Card", 1, "C").await;
        insert_card_with_rarity(&pool, "TST", "2", "EN", "Mythic Card", 2, "M").await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            rarity: vec![RarityCode::M],
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Mythic Card");
    }

    #[sqlx::test]
    async fn search_paginated_excludes_card_owned_only_by_private_users(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "private").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn search_paginated_owner_count_ignores_owners_who_do_not_propose_the_card(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "private").await;
        insert_user_with_visibility(&pool, "userC", "Carol", "private").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userC", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].owner_count, 1);
        assert!(!result.items[0].reserved);
    }

    #[sqlx::test]
    async fn search_paginated_returns_empty_when_every_user_is_private(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_user(&pool, "userB", "Bob").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn search_paginated_player_username_only_returns_cards_the_player_proposes(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "EN", "Open Card", 1, "R").await;
        insert_card_with_rarity(&pool, "TST", "2", "EN", "Closed Card", 2, "R").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "trade").await;
        insert_trading_binder(&pool, "userB", "Trade Binder").await;
        insert_rarity_filter(&pool, "userB", "R", true, 0).await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "1",
            "EN",
            false,
            "userB",
            1,
            100,
            Utc::now(),
            Some("Trade Binder"),
        )
        .await;
        insert_collection_entry_with_binder(
            &pool,
            "TST",
            "2",
            "EN",
            false,
            "userB",
            1,
            100,
            Utc::now(),
            None,
        )
        .await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            player_username: Some("Bob".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].name, "Open Card");
    }

    #[sqlx::test]
    async fn search_paginated_player_username_on_private_user_returns_empty(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", "Test Card", 1).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "private").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let query = SearchQuery {
            player_username: Some("Alice".to_string()),
            ..SearchQuery::default()
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    fn set_codes(sets: &[SetName]) -> Vec<String> {
        sets.iter().map(|s| s.code.to_string()).collect()
    }

    #[sqlx::test]
    async fn search_sets_text_scope_lists_the_sets_of_matching_offered_cards_by_name(pool: PgPool) {
        sqlx::query("INSERT INTO set_name (set_code, name) VALUES ('ZZZ', 'Alpha'), ('AAA', 'Zendikar'), ('MMM', 'Mirage')")
            .execute(&pool)
            .await
            .unwrap();
        insert_card(&pool, "ZZZ", "1", "EN", "Goblin Guide", 1).await;
        insert_card(&pool, "AAA", "1", "EN", "Goblin King", 2).await;
        insert_card(&pool, "MMM", "1", "EN", "Sol Ring", 3).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        for set_code in ["ZZZ", "AAA", "MMM"] {
            insert_collection_entry(
                &pool,
                set_code,
                "1",
                "EN",
                false,
                "userA",
                1,
                100,
                Utc::now(),
            )
            .await;
        }
        for id in 1..=3 {
            insert_price(&pool, CardMarketPriceEntity::simple(id, 100)).await;
        }
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let sets = adapter
            .search_sets(&SearchScope {
                search_query: Some("gob".to_string()),
                player_username: None,
            })
            .await
            .unwrap();

        assert_eq!(set_codes(&sets), vec!["ZZZ", "AAA"]);
        assert_eq!(sets[0].name, "Alpha");
    }

    #[sqlx::test]
    async fn search_sets_player_scope_lists_only_the_sets_that_player_offers(pool: PgPool) {
        // Bob trades from "Trade Binder" with rarities R open (keeping no copy), U open but
        // keeping 1 copy, and M closed.
        for set_code in ["OPN", "BND", "KPT", "CLS", "OTH"] {
            insert_set(&pool, set_code).await;
        }
        insert_card_with_rarity(&pool, "OPN", "1", "EN", "Offered", 1, "R").await;
        insert_card_with_rarity(&pool, "BND", "1", "EN", "Unbinned", 2, "R").await;
        insert_card_with_rarity(&pool, "KPT", "1", "EN", "Kept", 3, "U").await;
        insert_card_with_rarity(&pool, "CLS", "1", "EN", "Closed", 4, "M").await;
        insert_card_with_rarity(&pool, "OTH", "1", "EN", "Alice's", 5, "R").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "trade").await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_trading_binder(&pool, "userB", "Trade Binder").await;
        insert_rarity_filter(&pool, "userB", "R", true, 0).await;
        insert_rarity_filter(&pool, "userB", "U", true, 1).await;
        insert_rarity_filter(&pool, "userB", "M", false, 0).await;
        for (set_code, binder) in [
            ("OPN", Some("Trade Binder")),
            ("BND", None),
            ("KPT", Some("Trade Binder")),
            ("CLS", Some("Trade Binder")),
        ] {
            insert_collection_entry_with_binder(
                &pool,
                set_code,
                "1",
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
        insert_collection_entry(&pool, "OTH", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        for id in 1..=5 {
            insert_price(&pool, CardMarketPriceEntity::simple(id, 100)).await;
        }
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let sets = adapter
            .search_sets(&SearchScope {
                search_query: None,
                player_username: Some("bob".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(set_codes(&sets), vec!["OPN"]);
    }

    #[sqlx::test]
    async fn search_sets_leaves_out_the_sets_of_private_players(pool: PgPool) {
        insert_set(&pool, "PUB").await;
        insert_set(&pool, "PRV").await;
        insert_card(&pool, "PUB", "1", "EN", "Goblin Guide", 1).await;
        insert_card(&pool, "PRV", "1", "EN", "Goblin King", 2).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "private").await;
        insert_collection_entry(&pool, "PUB", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "PRV", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let text = adapter
            .search_sets(&SearchScope {
                search_query: Some("goblin".to_string()),
                player_username: None,
            })
            .await
            .unwrap();
        let private_player = adapter
            .search_sets(&SearchScope {
                search_query: None,
                player_username: Some("Bob".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(set_codes(&text), vec!["PUB"]);
        assert!(private_player.is_empty());
    }

    #[sqlx::test]
    async fn search_sets_includes_the_sets_of_the_callers_own_offered_cards(pool: PgPool) {
        // Like the search itself, the facet never leaves out the caller: whoever asks, Alice's
        // offered cards count, including when Alice asks — for a text search or for her own
        // collection.
        insert_set(&pool, "OWN").await;
        insert_set(&pool, "OTH").await;
        insert_card(&pool, "OWN", "1", "EN", "Goblin Guide", 1).await;
        insert_card(&pool, "OTH", "1", "EN", "Goblin King", 2).await;
        insert_user_with_visibility(&pool, "userA", "Alice", "public").await;
        insert_user_with_visibility(&pool, "userB", "Bob", "public").await;
        insert_collection_entry(&pool, "OWN", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "OTH", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let text = adapter
            .search_sets(&SearchScope {
                search_query: Some("goblin".to_string()),
                player_username: None,
            })
            .await
            .unwrap();
        let own_collection = adapter
            .search_sets(&SearchScope {
                search_query: None,
                player_username: Some("alice".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(set_codes(&text), vec!["OTH", "OWN"]);
        assert_eq!(set_codes(&own_collection), vec!["OWN"]);
    }

    /// An owned card (`OWN/1`, trend 100) next to a card nobody owns (`ORP/1`, trend 99 999),
    /// both on the market, the view refreshed.
    async fn insert_owned_and_orphan_cards(pool: &PgPool) {
        insert_set(pool, "OWN").await;
        insert_card(pool, "OWN", "1", "EN", "Owned Card", 1).await;
        insert_set(pool, "ORP").await;
        insert_card(pool, "ORP", "1", "EN", "Orphan Card", 2).await;
        insert_user_with_visibility(pool, "userA", "Alice", "public").await;
        insert_collection_entry(pool, "OWN", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(pool, CardMarketPriceEntity::simple(2, 99_999)).await;
        refresh_view(pool).await;
    }

    #[sqlx::test]
    async fn a_card_nobody_owns_is_left_out_of_the_search(pool: PgPool) {
        insert_owned_and_orphan_cards(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(SearchQuery::default())
            .await
            .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
    }

    #[sqlx::test]
    async fn a_card_nobody_owns_is_left_out_of_the_set_search(pool: PgPool) {
        insert_owned_and_orphan_cards(&pool).await;

        let adapter = SearchRepositoryAdapter::new(pool);
        let sets = adapter.search_sets(&SearchScope::default()).await.unwrap();

        let codes: Vec<String> = sets.iter().map(|set| set.code.to_string()).collect();
        assert_eq!(codes, vec!["OWN"]);
    }
}
