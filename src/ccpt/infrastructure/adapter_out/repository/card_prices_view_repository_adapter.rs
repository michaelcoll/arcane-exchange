use crate::application::error::{AppError, InfraError};
use crate::application::repository::CardPricesViewRepository;
use crate::domain::card::{Card, CardId, CollectionEntry};
use crate::domain::card_offer::{CardOfferSortField, PaginatedCardOffers};
use crate::domain::collection::{CollectionQuery, PaginatedCollection, SearchQuery};
use crate::domain::user::UserId;
use crate::infrastructure::adapter_out::repository::entities::{
    CardOfferEntity, CardWithPriceEntity,
};
use async_trait::async_trait;
use sqlx::{AssertSqlSafe, Pool, Postgres, query_as, query_scalar};

/// Builds the "AND ..." filter clause (search, rarity, sets, price range, player
/// username) for the collection query, starting bind placeholders at `start_idx`.
/// Returns (filter_clause, order_prefix, next_idx).
fn build_filter_clause(
    query: &CollectionQuery,
    player_username: Option<&str>,
    start_idx: u32,
) -> (String, String, u32) {
    let mut idx = start_idx;
    let mut conditions = Vec::new();
    let mut order_prefix = String::new();

    if query.search_query.is_some() {
        conditions.push(format!(
            "(cp.name ILIKE '%' || ${idx} || '%' OR ${idx} <% cp.name)"
        ));
        order_prefix = format!("word_similarity(${idx}, cp.name) DESC,");
        idx += 1;
    }
    if !query.rarity.is_empty() {
        conditions.push(format!("cp.rarity = ANY(${idx})"));
        idx += 1;
    }
    if !query.sets.is_empty() {
        conditions.push(format!("cp.set_code = ANY(${idx})"));
        idx += 1;
    }
    if query.price_min.is_some() {
        conditions.push(format!("cp.trend >= ${idx}"));
        idx += 1;
    }
    if query.price_max.is_some() {
        conditions.push(format!("cp.trend <= ${idx}"));
        idx += 1;
    }
    if player_username.is_some() {
        conditions.push(format!(
            "cp.user_id IN (SELECT id FROM users WHERE LOWER(username) = LOWER(${idx}))"
        ));
        idx += 1;
    }

    let filter_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("AND {}", conditions.join(" AND "))
    };

    (filter_clause, order_prefix, idx)
}

pub struct CardPricesViewRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CardPricesViewRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Shared implementation for [`CardPricesViewRepository::get_paginated`] and
    /// [`CardPricesViewRepository::search_paginated`].
    ///
    /// `user_id` is `Some` for the private "my collection" view (filtered to that user,
    /// real `purchase_price`/`added_at`, no `owner_count`) and `None` for the public
    /// search across every user's cards (no user filter, masked `purchase_price`/`added_at`,
    /// rows grouped by card with `owner_count` = number of distinct owners).
    async fn fetch_paginated(
        &self,
        user_id: Option<&UserId>,
        query: CollectionQuery,
        player_username: Option<&str>,
    ) -> Result<PaginatedCollection, AppError> {
        let limit_idx = if user_id.is_some() { 2 } else { 1 };
        let offset_idx = limit_idx + 1;
        let (filter_clause, order_prefix, _) =
            build_filter_clause(&query, player_username, offset_idx + 1);
        let (count_filter_clause, _, _) = build_filter_clause(
            &query,
            player_username,
            if user_id.is_some() { 2 } else { 1 },
        );

        let (where_clause, owned_columns, group_by_clause) = if user_id.is_some() {
            (
                "WHERE cp.user_id = $1",
                r#"cp.quantity,
                 cp.purchase_price,
                 cp.added_at,
                 0::bigint AS owner_count,
                 EXISTS (
                     SELECT 1 FROM trade_card tc
                     JOIN trade t ON t.id = tc.trade_id
                     WHERE tc.set_code = cp.set_code AND tc.collector_number = cp.collector_number
                       AND tc.language_code = cp.language_code AND tc.foil = cp.foil
                       AND tc.owner_user_id = cp.user_id
                       AND t.status IN ('ONE_ACCEPTED', 'FULLY_ACCEPTED')
                 ) AS reserved"#,
                "",
            )
        } else {
            (
                "WHERE 1 = 1",
                r#"0::integer AS quantity,
                 NULL::integer AS purchase_price,
                 NULL::timestamptz AS added_at,
                 COUNT(DISTINCT cp.user_id) AS owner_count,
                 false AS reserved"#,
                r#"GROUP BY cp.set_code, sn.name, cp.collector_number, cp.language_code,
                            cp.foil, cp.name, cp.rarity, cp.scryfall_id, cp.the_gatherer_id,
                            cp.avg, cp.low, cp.trend"#,
            )
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
                 cp.the_gatherer_id,
                 {owned_columns},
                 cp.avg,
                 cp.low,
                 cp.trend
               FROM mv_card_prices cp
               JOIN set_name sn ON sn.set_code = cp.set_code
               {where_clause}
               {filter_clause}
               {group_by_clause}
               ORDER BY {order_prefix} {} {} NULLS LAST, cp.name
               LIMIT ${limit_idx} OFFSET ${offset_idx}"#,
            query.sort_by, query.sort_dir,
        );

        let offset = (query.page * query.page_size) as i64;
        let limit = query.page_size as i64;

        let mut base_query = query_as::<_, CardWithPriceEntity>(AssertSqlSafe(sql.as_str()));
        if let Some(uid) = user_id {
            base_query = base_query.bind(uid.as_str());
        }
        base_query = base_query.bind(limit).bind(offset);
        if let Some(q) = &query.search_query {
            base_query = base_query.bind(q.clone());
        }
        if !query.rarity.is_empty() {
            base_query = base_query.bind(
                query
                    .rarity
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>(),
            );
        }
        if !query.sets.is_empty() {
            base_query = base_query.bind(query.sets.clone());
        }
        if let Some(v) = query.price_min {
            base_query = base_query.bind(v as i64);
        }
        if let Some(v) = query.price_max {
            base_query = base_query.bind(v as i64);
        }
        if let Some(username) = player_username {
            base_query = base_query.bind(username.to_string());
        }

        let entities = base_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Infra(InfraError::RepositoryError(e.to_string())))?;

        let count_sql = if user_id.is_some() {
            format!("SELECT COUNT(*) FROM mv_card_prices cp {where_clause} {count_filter_clause}")
        } else {
            format!(
                r#"SELECT COUNT(*) FROM (
                     SELECT 1 FROM mv_card_prices cp {where_clause} {count_filter_clause}
                     GROUP BY cp.set_code, cp.collector_number, cp.language_code, cp.foil
                   ) sub"#
            )
        };

        let mut base_count = query_scalar::<_, i64>(AssertSqlSafe(count_sql.as_str()));
        if let Some(uid) = user_id {
            base_count = base_count.bind(uid.as_str());
        }
        if let Some(q) = &query.search_query {
            base_count = base_count.bind(q.clone());
        }
        if !query.rarity.is_empty() {
            base_count = base_count.bind(
                query
                    .rarity
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>(),
            );
        }
        if !query.sets.is_empty() {
            base_count = base_count.bind(query.sets.clone());
        }
        if let Some(v) = query.price_min {
            base_count = base_count.bind(v as i64);
        }
        if let Some(v) = query.price_max {
            base_count = base_count.bind(v as i64);
        }
        if let Some(username) = player_username {
            base_count = base_count.bind(username.to_string());
        }

        let total: i64 = base_count
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Infra(InfraError::RepositoryError(e.to_string())))?;

        Ok(PaginatedCollection {
            items: entities.into_iter().map(Card::from).collect(),
            total: total as u64,
            page: query.page,
            page_size: query.page_size,
        })
    }
}

#[async_trait]
impl CardPricesViewRepository for CardPricesViewRepositoryAdapter {
    async fn refresh(&self) -> Result<(), AppError> {
        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY mv_card_prices")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_paginated(
        &self,
        user_id: &UserId,
        query: CollectionQuery,
    ) -> Result<PaginatedCollection, AppError> {
        self.fetch_paginated(Some(user_id), query, None).await
    }

    async fn search_paginated(&self, query: SearchQuery) -> Result<PaginatedCollection, AppError> {
        self.fetch_paginated(
            None,
            query.collection_query,
            query.player_username.as_deref(),
        )
        .await
    }

    async fn exists(&self, card_id: &CardId) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM mv_card_prices
                 WHERE set_code = $1 AND collector_number = $2 AND language_code = $3 AND foil = $4)"#,
            card_id.set_code.to_string(),
            card_id.collector_number,
            card_id.language_code.to_string(),
            card_id.foil
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Infra(InfraError::RepositoryError(e.to_string())))?;

        Ok(exists.unwrap_or(false))
    }

    async fn get_offers(
        &self,
        user_id: &UserId,
        card_id: &CardId,
        sort_by: CardOfferSortField,
        page: u32,
        page_size: u32,
    ) -> Result<PaginatedCardOffers, AppError> {
        let offset = (page * page_size) as i64;
        let limit = page_size as i64;

        let entities = match sort_by {
            CardOfferSortField::SellingPrice => sqlx::query_as!(
                CardOfferEntity,
                r#"SELECT u.username AS owner_username, cp.quantity AS "quantity!", cp.trend AS selling_price
                     FROM mv_card_prices cp
                     JOIN users u ON u.id = cp.user_id
                     WHERE cp.set_code = $1 AND cp.collector_number = $2 AND cp.language_code = $3
                       AND cp.foil = $4 AND cp.user_id != $5
                     ORDER BY cp.trend ASC NULLS LAST, u.username
                     LIMIT $6 OFFSET $7"#,
                card_id.set_code.to_string(),
                card_id.collector_number,
                card_id.language_code.to_string(),
                card_id.foil,
                user_id.as_str(),
                limit,
                offset
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Infra(InfraError::RepositoryError(e.to_string())))?,
        };

        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) FROM mv_card_prices cp
                 WHERE cp.set_code = $1 AND cp.collector_number = $2 AND cp.language_code = $3
                   AND cp.foil = $4 AND cp.user_id != $5"#,
            card_id.set_code.to_string(),
            card_id.collector_number,
            card_id.language_code.to_string(),
            card_id.foil,
            user_id.as_str()
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Infra(InfraError::RepositoryError(e.to_string())))?
        .unwrap_or(0);

        Ok(PaginatedCardOffers {
            items: entities.into_iter().map(CollectionEntry::from).collect(),
            total: total as u64,
            page,
            page_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::collection::{CollectionSortField, SortDirection};
    use crate::domain::rarity_code::RarityCode;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_collection_entry, insert_price, insert_set, insert_user, refresh_view,
    };
    use crate::infrastructure::adapter_out::repository::entities::{
        CardMarketPriceEntity, PriceGuideEntity,
    };
    use chrono::{NaiveDate, Utc};
    use sqlx::{PgPool, Pool, Postgres};

    impl CardMarketPriceEntity {
        pub fn simple(id_produit: i32, avg: i32) -> Self {
            Self::simple_at(id_produit, chrono::Local::now().date_naive(), avg)
        }

        pub fn simple_at(id_produit: i32, date: NaiveDate, avg: i32) -> Self {
            Self {
                id_produit,
                date,
                normal: PriceGuideEntity {
                    low: Some(avg / 2),
                    avg: Some(avg),
                    trend: Some(avg),
                },
                foil: PriceGuideEntity::empty(),
            }
        }

        pub fn with_foil(id_produit: i32, avg: i32, avg_foil: i32) -> Self {
            Self::with_foil_at(id_produit, chrono::Local::now().date_naive(), avg, avg_foil)
        }

        pub fn with_foil_at(id_produit: i32, date: NaiveDate, avg: i32, avg_foil: i32) -> Self {
            Self {
                id_produit,
                date,
                normal: PriceGuideEntity {
                    low: Some(avg / 2),
                    avg: Some(avg),
                    trend: Some(avg),
                },
                foil: PriceGuideEntity {
                    low: Some(avg_foil / 2),
                    avg: Some(avg_foil),
                    trend: Some(avg_foil),
                },
            }
        }
    }

    #[sqlx::test]
    async fn test_refresh_materialized_view(pool: Pool<Postgres>) {
        let adapter = CardPricesViewRepositoryAdapter::new(pool);

        let result = adapter.refresh().await;

        assert!(result.is_ok());
    }

    #[sqlx::test]
    async fn get_paginated_returns_empty_when_no_cards_in_collection(pool: PgPool) {
        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn get_paginated_returns_cards_for_the_given_user(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 2, 500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 200)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
    }

    #[sqlx::test]
    async fn get_paginated_marks_card_as_reserved_when_trade_is_one_accepted(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::{
            insert_trade, insert_trade_card, insert_user,
        };

        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 2, 500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 200)).await;
        refresh_view(&pool).await;

        insert_user(&pool, "user1", "alice").await;
        insert_user(&pool, "user2", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user2", "user1", "ONE_ACCEPTED").await;
        insert_trade_card(&pool, trade_id, "TST", "1", "EN", false, "user1", 1).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        match result.items[0].collection_entry {
            CollectionEntry::Mine { reserved, .. } => assert!(reserved),
            _ => panic!("expected CollectionEntry::Mine"),
        }
    }

    #[sqlx::test]
    async fn get_paginated_marks_card_as_reserved_when_trade_is_fully_accepted(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::{
            insert_trade, insert_trade_card, insert_user,
        };

        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 2, 500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 200)).await;
        refresh_view(&pool).await;

        insert_user(&pool, "user1", "alice").await;
        insert_user(&pool, "user2", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user2", "user1", "FULLY_ACCEPTED").await;
        insert_trade_card(&pool, trade_id, "TST", "1", "EN", false, "user1", 1).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        match result.items[0].collection_entry {
            CollectionEntry::Mine { reserved, .. } => assert!(reserved),
            _ => panic!("expected CollectionEntry::Mine"),
        }
    }

    #[sqlx::test]
    async fn get_paginated_does_not_mark_card_as_reserved_when_trade_is_pending(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::{
            insert_trade, insert_trade_card, insert_user,
        };

        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 2, 500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 200)).await;
        refresh_view(&pool).await;

        insert_user(&pool, "user1", "alice").await;
        insert_user(&pool, "user2", "bob").await;
        let trade_id = uuid::Uuid::new_v4();
        insert_trade(&pool, trade_id, "user2", "user1", "PENDING").await;
        insert_trade_card(&pool, trade_id, "TST", "1", "EN", false, "user1", 1).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        match result.items[0].collection_entry {
            CollectionEntry::Mine { reserved, .. } => assert!(!reserved),
            _ => panic!("expected CollectionEntry::Mine"),
        }
    }

    #[sqlx::test]
    async fn get_paginated_does_not_mark_card_as_reserved_when_no_active_trade(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 2, 500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 200)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        match result.items[0].collection_entry {
            CollectionEntry::Mine { reserved, .. } => assert!(!reserved),
            _ => panic!("expected CollectionEntry::Mine"),
        }
    }

    #[sqlx::test]
    async fn get_paginated_respects_page_size(pool: PgPool) {
        for i in 1..=5i32 {
            let set = format!("TS{}", i);
            insert_set(&pool, &set).await;
            insert_card(&pool, &set, "1", "EN", false, "Test Card", i).await;
            insert_collection_entry(&pool, &set, "1", "EN", false, "user1", 1, 100, Utc::now())
                .await;
            insert_price(&pool, CardMarketPriceEntity::simple(i, i * 100)).await;
        }
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            page: 0,
            page_size: 2,
            ..CollectionQuery::default()
        };
        let result = adapter
            .get_paginated(&UserId::new("user1"), query)
            .await
            .unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);
        assert_eq!(result.page_size, 2);
    }

    #[sqlx::test]
    async fn get_paginated_returns_correct_page(pool: PgPool) {
        for i in 1..=4i32 {
            let set = format!("TS{}", i);
            insert_set(&pool, &set).await;
            insert_card(&pool, &set, "1", "EN", false, "Test Card", i).await;
            insert_collection_entry(&pool, &set, "1", "EN", false, "user1", 1, 100, Utc::now())
                .await;
            insert_price(&pool, CardMarketPriceEntity::simple(i, i * 100)).await;
        }
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query_page0 = CollectionQuery {
            page: 0,
            page_size: 2,
            ..CollectionQuery::default()
        };
        let query_page1 = CollectionQuery {
            page: 1,
            page_size: 2,
            ..CollectionQuery::default()
        };

        let page0 = adapter
            .get_paginated(&UserId::new("user1"), query_page0)
            .await
            .unwrap();
        let page1 = adapter
            .get_paginated(&UserId::new("user1"), query_page1)
            .await
            .unwrap();

        assert_eq!(page0.items.len(), 2);
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page0.page, 0);
        assert_eq!(page1.page, 1);

        let ids_page0: Vec<_> = page0
            .items
            .iter()
            .map(|c| c.id.set_code.to_string())
            .collect();
        let ids_page1: Vec<_> = page1
            .items
            .iter()
            .map(|c| c.id.set_code.to_string())
            .collect();
        assert!(ids_page0.iter().all(|id| !ids_page1.contains(id)));
    }

    #[sqlx::test]
    async fn get_paginated_sorts_by_avg_descending_by_default(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", false, "Test Card", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", false, "Test Card", 2).await;
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 300)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 2);
        let first_avg = result.items[0]
            .price_guide
            .as_ref()
            .and_then(|p| p.avg.value);
        let second_avg = result.items[1]
            .price_guide
            .as_ref()
            .and_then(|p| p.avg.value);
        assert!(first_avg >= second_avg);
    }

    #[sqlx::test]
    async fn get_paginated_sorts_by_set_code_ascending(pool: PgPool) {
        insert_set(&pool, "ZZZ").await;
        insert_card(&pool, "ZZZ", "1", "EN", false, "Test Card", 1).await;
        insert_set(&pool, "AAA").await;
        insert_card(&pool, "AAA", "1", "EN", false, "Test Card", 2).await;
        insert_collection_entry(&pool, "ZZZ", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "AAA", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            sort_by: CollectionSortField::SetCode,
            sort_dir: SortDirection::Asc,
            ..CollectionQuery::default()
        };
        let result = adapter
            .get_paginated(&UserId::new("user1"), query)
            .await
            .unwrap();

        assert_eq!(result.items[0].id.set_code.to_string(), "AAA");
        assert_eq!(result.items[1].id.set_code.to_string(), "ZZZ");
    }

    #[sqlx::test]
    async fn get_paginated_sorts_by_language_code_ascending(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "FR", false, "Test Card", 1).await;
        insert_card(&pool, "TST", "2", "EN", false, "Test Card", 2).await;
        insert_collection_entry(&pool, "TST", "1", "FR", false, "user1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            sort_by: CollectionSortField::LanguageCode,
            sort_dir: SortDirection::Asc,
            ..CollectionQuery::default()
        };
        let result = adapter
            .get_paginated(&UserId::new("user1"), query)
            .await
            .unwrap();

        assert_eq!(result.items.len(), 2);
        let first_lang = result.items[0].id.language_code.to_string();
        let second_lang = result.items[1].id.language_code.to_string();
        assert!(first_lang <= second_lang);
    }

    #[sqlx::test]
    async fn get_paginated_returns_empty_page_when_offset_exceeds_total(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            page: 5,
            page_size: 10,
            ..CollectionQuery::default()
        };
        let result = adapter
            .get_paginated(&UserId::new("user1"), query)
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 1);
        assert_eq!(result.page, 5);
    }

    #[sqlx::test]
    async fn get_paginated_isolates_cards_between_multiple_users(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", false, "Test Card", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", false, "Test Card", 2).await;
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 200)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);

        let result_a = adapter
            .get_paginated(&UserId::new("userA"), CollectionQuery::default())
            .await
            .unwrap();
        let result_b = adapter
            .get_paginated(&UserId::new("userB"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result_a.total, 1);
        assert_eq!(result_b.total, 1);
        assert_eq!(result_a.items[0].id.set_code.to_string(), "TS1");
        assert_eq!(result_b.items[0].id.set_code.to_string(), "TS2");
    }

    #[sqlx::test]
    async fn search_paginated_returns_cards_from_all_users(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", false, "Test Card", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", false, "Test Card", 2).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_user(&pool, "userB", "Bob").await;
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 200)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(CollectionQuery::default().into())
            .await
            .unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
    }

    #[sqlx::test]
    async fn search_paginated_returns_empty_when_no_cards(pool: PgPool) {
        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(CollectionQuery::default().into())
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn search_paginated_masks_purchase_price_and_added_at_for_every_user(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "userB", "Bob").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 3, 1500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(CollectionQuery::default().into())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(
            result.items[0].collection_entry,
            CollectionEntry::Public { owner_count: 1 }
        );
    }

    #[sqlx::test]
    async fn search_paginated_masks_purchase_price_even_for_the_searching_users_own_card(
        pool: PgPool,
    ) {
        // search_paginated has no notion of "the current user" — it always returns
        // CollectionEntry::Public, even for the card of the user running the search.
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 3, 1500, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(CollectionQuery::default().into())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(
            result.items[0].collection_entry,
            CollectionEntry::Public { owner_count: 1 }
        );
    }

    #[sqlx::test]
    async fn search_paginated_same_card_owned_by_three_users_appears_once_with_owner_count_three(
        pool: PgPool,
    ) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_user(&pool, "userB", "Bob").await;
        insert_user(&pool, "userC", "Carol").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userC", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .search_paginated(CollectionQuery::default().into())
            .await
            .unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(
            result.items[0].collection_entry,
            CollectionEntry::Public { owner_count: 3 }
        );
    }

    #[sqlx::test]
    async fn search_paginated_filters_by_search_query_fuzzy(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Goblin Guide", 1).await;
        insert_card(&pool, "TST", "2", "EN", false, "Sol Ring", 2).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            search_query: Some("gob".to_string()),
            ..CollectionQuery::default()
        };
        let result = adapter.search_paginated(query.into()).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Goblin Guide");
    }

    #[sqlx::test]
    async fn search_paginated_filters_by_rarity(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_card_with_rarity;

        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "EN", false, "Common Card", 1, "C").await;
        insert_card_with_rarity(&pool, "TST", "2", "EN", false, "Mythic Card", 2, "M").await;
        insert_user(&pool, "userA", "Alice").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            rarity: vec![RarityCode::M],
            ..CollectionQuery::default()
        };
        let result = adapter.search_paginated(query.into()).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Mythic Card");
    }

    #[sqlx::test]
    async fn search_paginated_respects_page_size(pool: PgPool) {
        for i in 1..=5i32 {
            let set = format!("TS{}", i);
            insert_set(&pool, &set).await;
            insert_card(&pool, &set, "1", "EN", false, "Test Card", i).await;
            insert_user(&pool, &format!("user{}", i), &format!("User{}", i)).await;
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

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            page: 0,
            page_size: 2,
            ..CollectionQuery::default()
        };
        let result = adapter.search_paginated(query.into()).await.unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 5);
        assert_eq!(result.page_size, 2);
    }

    #[sqlx::test]
    async fn search_paginated_filters_by_player_username_exact_match(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_card(&pool, "TS1", "1", "EN", false, "Card A", 1).await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS2", "1", "EN", false, "Card B", 2).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_user(&pool, "userB", "Bob").await;
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = SearchQuery {
            collection_query: CollectionQuery::default(),
            player_username: Some("Alice".to_string()),
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(result.items[0].name, "Card A");
    }

    #[sqlx::test]
    async fn search_paginated_player_username_is_case_insensitive(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Card A", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = SearchQuery {
            collection_query: CollectionQuery::default(),
            player_username: Some("ALICE".to_string()),
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Card A");
    }

    #[sqlx::test]
    async fn search_paginated_player_username_requires_exact_match_no_partial(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Card A", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = SearchQuery {
            collection_query: CollectionQuery::default(),
            player_username: Some("ali".to_string()),
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn search_paginated_returns_empty_for_unknown_player_username(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Card A", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = SearchQuery {
            collection_query: CollectionQuery::default(),
            player_username: Some("unknown-user".to_string()),
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
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_user(&pool, "userB", "Bob").await;
        insert_user(&pool, "userC", "Carol").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userC", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = SearchQuery {
            collection_query: CollectionQuery::default(),
            player_username: Some("Alice".to_string()),
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.items.len(), 1);
        assert_eq!(
            result.items[0].collection_entry,
            CollectionEntry::Public { owner_count: 1 }
        );
    }

    #[sqlx::test]
    async fn search_paginated_player_username_combines_with_other_filters(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_card_with_rarity;

        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "EN", false, "Common Card", 1, "C").await;
        insert_card_with_rarity(&pool, "TST", "2", "EN", false, "Mythic Card", 2, "M").await;
        insert_user(&pool, "userA", "Alice").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = SearchQuery {
            collection_query: CollectionQuery {
                rarity: vec![RarityCode::M],
                ..CollectionQuery::default()
            },
            player_username: Some("Alice".to_string()),
        };
        let result = adapter.search_paginated(query).await.unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Mythic Card");
    }

    #[sqlx::test]
    async fn get_paginated_uses_latest_price_when_multiple_dates_exist(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;

        insert_price(
            &pool,
            CardMarketPriceEntity::simple_at(1, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 100),
        )
        .await;
        insert_price(
            &pool,
            CardMarketPriceEntity::simple_at(1, NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(), 999),
        )
        .await;

        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        let avg = result.items[0]
            .price_guide
            .as_ref()
            .and_then(|p| p.avg.value);
        assert_eq!(avg, Some(999));
    }

    #[sqlx::test]
    async fn get_paginated_returns_foil_prices_for_foil_cards(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", true, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", true, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::with_foil(1, 50, 777)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert!(result.items[0].id.foil);
        let avg = result.items[0]
            .price_guide
            .as_ref()
            .and_then(|p| p.avg.value);
        assert_eq!(avg, Some(777));
    }

    #[sqlx::test]
    async fn get_paginated_returns_correct_quantity_and_purchase_price(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 7, 1234, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 500)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        let CollectionEntry::Mine {
            quantity,
            purchase_price,
            ..
        } = &result.items[0].collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(*quantity, 7);
        assert_eq!(*purchase_price, 1234);
    }

    #[sqlx::test]
    async fn get_paginated_price_guide_is_none_when_no_cardmarket_data(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert!(result.items[0].price_guide.is_none());
    }

    #[sqlx::test]
    async fn get_paginated_non_foil_card_does_not_use_foil_prices(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::with_foil(1, 123, 999)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert!(!result.items[0].id.foil);
        let avg = result.items[0]
            .price_guide
            .as_ref()
            .and_then(|p| p.avg.value);
        assert_eq!(avg, Some(123));
    }

    #[sqlx::test]
    async fn get_paginated_card_not_in_collection_is_not_returned(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_paginated(&UserId::new("user1"), CollectionQuery::default())
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn get_paginated_filters_by_search_query_fuzzy(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Goblin Guide", 1).await;
        insert_card(&pool, "TST", "2", "EN", false, "Sol Ring", 2).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            search_query: Some("gob".to_string()),
            ..CollectionQuery::default()
        };
        let result = adapter
            .get_paginated(&UserId::new("user1"), query)
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Goblin Guide");
    }

    #[sqlx::test]
    async fn get_paginated_filters_by_rarity(pool: PgPool) {
        use crate::infrastructure::adapter_out::repository::common_repository_tests::insert_card_with_rarity;

        insert_set(&pool, "TST").await;
        insert_card_with_rarity(&pool, "TST", "1", "EN", false, "Common Card", 1, "C").await;
        insert_card_with_rarity(&pool, "TST", "2", "EN", false, "Mythic Card", 2, "M").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            rarity: vec![RarityCode::M],
            ..CollectionQuery::default()
        };
        let result = adapter
            .get_paginated(&UserId::new("user1"), query)
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Mythic Card");
    }

    #[sqlx::test]
    async fn get_paginated_filters_by_sets(pool: PgPool) {
        insert_set(&pool, "TS1").await;
        insert_set(&pool, "TS2").await;
        insert_card(&pool, "TS1", "1", "EN", false, "Card A", 1).await;
        insert_card(&pool, "TS2", "1", "EN", false, "Card B", 2).await;
        insert_collection_entry(&pool, "TS1", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TS2", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            sets: vec!["TS2".to_string()],
            ..CollectionQuery::default()
        };
        let result = adapter
            .get_paginated(&UserId::new("user1"), query)
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Card B");
    }

    #[sqlx::test]
    async fn get_paginated_filters_by_price_range(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Cheap Card", 1).await;
        insert_card(&pool, "TST", "2", "EN", false, "Expensive Card", 2).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "2", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        insert_price(&pool, CardMarketPriceEntity::simple(2, 5000)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let query = CollectionQuery {
            price_min: Some(1000),
            price_max: Some(10000),
            ..CollectionQuery::default()
        };
        let result = adapter
            .get_paginated(&UserId::new("user1"), query)
            .await
            .unwrap();

        assert_eq!(result.items.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.items[0].name, "Expensive Card");
    }

    fn card_id(set_code: &str, collector_number: &str, language_code: &str, foil: bool) -> CardId {
        use crate::domain::language_code::LanguageCode;
        CardId::new(
            set_code,
            collector_number,
            LanguageCode::new(language_code),
            foil,
        )
    }

    #[sqlx::test]
    async fn exists_returns_true_when_card_is_owned_by_someone(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .exists(&card_id("TST", "1", "EN", false))
            .await
            .unwrap();

        assert!(result);
    }

    #[sqlx::test]
    async fn exists_returns_true_even_when_only_the_requesting_user_owns_it(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "user1", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .exists(&card_id("TST", "1", "EN", false))
            .await
            .unwrap();

        assert!(result);
    }

    #[sqlx::test]
    async fn exists_returns_false_when_no_one_owns_the_card(pool: PgPool) {
        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .exists(&card_id("TST", "1", "EN", false))
            .await
            .unwrap();

        assert!(!result);
    }

    #[sqlx::test]
    async fn get_offers_returns_other_owners_with_quantity_and_selling_price(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_user(&pool, "userB", "Bob").await;
        insert_user(&pool, "userC", "Carol").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 2, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userC", 3, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_offers(
                &UserId::new("userA"),
                &card_id("TST", "1", "EN", false),
                CardOfferSortField::SellingPrice,
                0,
                20,
            )
            .await
            .unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);
        for item in &result.items {
            match item {
                CollectionEntry::Owned {
                    owner_username,
                    selling_price,
                    ..
                } => {
                    assert_ne!(owner_username, "Alice");
                    // All offers of the same card share the same trend price today (no
                    // per-seller pricing yet), so `selling_price` is identical for every row.
                    assert_eq!(*selling_price, Some(100));
                }
                CollectionEntry::Mine { .. } | CollectionEntry::Public { .. } => {
                    panic!("expected CollectionEntry::Owned")
                }
            }
        }
    }

    #[sqlx::test]
    async fn get_offers_breaks_ties_by_owner_username_ascending(pool: PgPool) {
        // All offers of a given card share the same selling_price today (derived from the
        // same trend), so the secondary sort key (owner_username) is what's actually
        // observable in the returned order.
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_user(&pool, "userB", "Zoe").await;
        insert_user(&pool, "userC", "Bob").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userB", 1, 100, Utc::now()).await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userC", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_offers(
                &UserId::new("userA"),
                &card_id("TST", "1", "EN", false),
                CardOfferSortField::SellingPrice,
                0,
                20,
            )
            .await
            .unwrap();

        let usernames: Vec<&str> = result
            .items
            .iter()
            .map(|item| match item {
                CollectionEntry::Owned { owner_username, .. } => owner_username.as_str(),
                CollectionEntry::Mine { .. } | CollectionEntry::Public { .. } => {
                    panic!("expected CollectionEntry::Owned")
                }
            })
            .collect();
        assert_eq!(usernames, vec!["Bob", "Zoe"]);
    }

    #[sqlx::test]
    async fn get_offers_excludes_current_user(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_user(&pool, "userA", "Alice").await;
        insert_collection_entry(&pool, "TST", "1", "EN", false, "userA", 1, 100, Utc::now()).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_offers(
                &UserId::new("userA"),
                &card_id("TST", "1", "EN", false),
                CardOfferSortField::SellingPrice,
                0,
                20,
            )
            .await
            .unwrap();

        assert!(result.items.is_empty());
        assert_eq!(result.total, 0);
    }

    #[sqlx::test]
    async fn get_offers_paginates_results(pool: PgPool) {
        insert_set(&pool, "TST").await;
        insert_card(&pool, "TST", "1", "EN", false, "Test Card", 1).await;
        insert_price(&pool, CardMarketPriceEntity::simple(1, 100)).await;
        for (i, name) in ["userB", "userC", "userD"].iter().enumerate() {
            insert_user(&pool, name, &format!("User{}", i)).await;
            insert_collection_entry(&pool, "TST", "1", "EN", false, name, 1, 100, Utc::now()).await;
        }
        refresh_view(&pool).await;

        let adapter = CardPricesViewRepositoryAdapter::new(pool);
        let result = adapter
            .get_offers(
                &UserId::new("userA"),
                &card_id("TST", "1", "EN", false),
                CardOfferSortField::SellingPrice,
                0,
                2,
            )
            .await
            .unwrap();

        assert_eq!(result.items.len(), 2);
        assert_eq!(result.total, 3);
        assert_eq!(result.page, 0);
        assert_eq!(result.page_size, 2);
    }
}
