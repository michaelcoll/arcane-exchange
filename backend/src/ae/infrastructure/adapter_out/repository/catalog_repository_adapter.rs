use crate::application::error::AppError;
use crate::application::repository::CatalogRepository;
use crate::domain::catalog::{CatalogCard, CatalogSearchItem, ScanIndexEntry, TrendPrices};
use crate::domain::pagination::{Paginated, Pagination};
use crate::domain::set_name::{SetCode, SetName};
use crate::infrastructure::adapter_out::repository::entities::{
    cardmarket_id_from_db, cardmarket_id_to_db, from_db_rarity, invalid_db_value,
};
use async_trait::async_trait;
use sqlx::{Pool, Postgres, QueryBuilder};

/// Rows per `INSERT`: 8 binds each stay far under Postgres's 65535 parameters.
const INSERT_CHUNK_SIZE: usize = 5_000;

pub struct CatalogRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CatalogRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CatalogRepository for CatalogRepositoryAdapter {
    #[tracing::instrument(name = "catalog_repo.replace_all", skip_all, fields(sentry.op = "db"))]
    async fn replace_all(&self, sets: &[SetName], cards: &[CatalogCard]) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        for chunk in sets.chunks(INSERT_CHUNK_SIZE) {
            let mut qb: QueryBuilder<Postgres> =
                QueryBuilder::new("INSERT INTO set_name (set_code, name) ");
            qb.push_values(chunk, |mut b, set| {
                b.push_bind(set.code.to_string())
                    .push_bind(set.name.clone());
            });
            // DO NOTHING, like `SetNameRepository::save_all`: a known set's name is never overwritten.
            qb.push(" ON CONFLICT (set_code) DO NOTHING");
            qb.build().execute(&mut *tx).await?;
        }

        sqlx::query!("DELETE FROM catalog_card")
            .execute(&mut *tx)
            .await?;

        for chunk in cards.chunks(INSERT_CHUNK_SIZE) {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                "INSERT INTO catalog_card (set_code, collector_number, rarity, name, french_name, \
                 scryfall_id, cardmarket_id, image_url) ",
            );
            let cardmarket_ids = chunk
                .iter()
                .map(|card| card.cardmarket_id.map(cardmarket_id_to_db).transpose())
                .collect::<Result<Vec<_>, _>>()?;
            qb.push_values(
                chunk.iter().zip(cardmarket_ids),
                |mut b, (card, cardmarket_id)| {
                    b.push_bind(card.set_code.to_string())
                        .push_bind(card.collector_number.clone())
                        .push_bind(card.rarity.to_string())
                        .push_bind(card.name.clone())
                        .push_bind(card.french_name.clone())
                        .push_bind(card.scryfall_id)
                        .push_bind(cardmarket_id)
                        .push_bind(card.image_url.clone());
                },
            );
            qb.build().execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    #[tracing::instrument(name = "catalog_repo.find", skip_all, fields(sentry.op = "db"))]
    async fn find(
        &self,
        set_code: &SetCode,
        collector_number: &str,
    ) -> Result<Option<CatalogCard>, AppError> {
        let row = sqlx::query!(
            "SELECT rarity, name, french_name, scryfall_id, cardmarket_id, image_url
             FROM catalog_card
             WHERE set_code = $1 AND collector_number = $2",
            set_code.to_string(),
            collector_number,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|row| {
                Ok::<_, AppError>(CatalogCard {
                    set_code: set_code.clone(),
                    collector_number: collector_number.to_string(),
                    rarity: from_db_rarity(&row.rarity)?,
                    name: row.name,
                    french_name: row.french_name,
                    scryfall_id: row.scryfall_id,
                    cardmarket_id: row.cardmarket_id.map(cardmarket_id_from_db).transpose()?,
                    image_url: row.image_url,
                })
            })
            .transpose()?)
    }

    #[tracing::instrument(name = "catalog_repo.trend_prices", skip_all, fields(sentry.op = "db"))]
    async fn trend_prices(
        &self,
        cards: &[(SetCode, String)],
    ) -> Result<Vec<TrendPrices>, AppError> {
        let set_codes: Vec<String> = cards.iter().map(|(code, _)| code.to_string()).collect();
        let numbers: Vec<String> = cards.iter().map(|(_, number)| number.clone()).collect();

        // The latest price of the catalog's Cardmarket id, not `mv_last_cardmarket_prices`: that
        // view is built from `card`, which a card nobody owns is not in. The primary key
        // (id_produit, date) makes the lookup an index seek per card.
        let rows = sqlx::query!(
            r#"SELECT p.trend AS "trend?", p.trend_foil AS "trend_foil?"
               FROM unnest($1::text[], $2::text[]) WITH ORDINALITY AS k (set_code, collector_number, ord)
                        LEFT JOIN catalog_card cc
                                  ON (cc.set_code, cc.collector_number) = (k.set_code, k.collector_number)
                        LEFT JOIN LATERAL (SELECT trend, trend_foil
                                           FROM cardmarket_price
                                           WHERE id_produit = cc.cardmarket_id
                                           ORDER BY date DESC
                                           LIMIT 1) p ON TRUE
               ORDER BY k.ord"#,
            &set_codes,
            &numbers,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(TrendPrices {
                    trend_cents: row.trend.map(trend_from_db).transpose()?,
                    foil_trend_cents: row.trend_foil.map(trend_from_db).transpose()?,
                })
            })
            .collect()
    }

    #[tracing::instrument(name = "catalog_repo.search", skip_all, fields(sentry.op = "db"))]
    async fn search(
        &self,
        name: Option<&str>,
        set_code: Option<&SetCode>,
        pagination: Pagination,
    ) -> Result<Paginated<CatalogSearchItem>, AppError> {
        let set_code = set_code.map(ToString::to_string);

        // A name matches by substring or by trigram similarity, in English or in French: the
        // same rule as the search of the collection.
        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) AS "total!"
               FROM catalog_card cc
               WHERE ($2::TEXT IS NULL OR cc.set_code = $2)
                 AND ($1::TEXT IS NULL
                      OR cc.name ILIKE '%' || $1 || '%' OR $1 <% cc.name
                      OR cc.french_name ILIKE '%' || $1 || '%' OR $1 <% cc.french_name)"#,
            name,
            set_code,
        )
        .fetch_one(&self.pool)
        .await?;

        let rows = sqlx::query!(
            r#"SELECT cc.set_code, sn.name AS set_name, cc.collector_number, cc.rarity, cc.name,
                      cc.french_name, cc.image_url
               FROM catalog_card cc
                        JOIN set_name sn ON sn.set_code = cc.set_code
               WHERE ($2::TEXT IS NULL OR cc.set_code = $2)
                 AND ($1::TEXT IS NULL
                      OR cc.name ILIKE '%' || $1 || '%' OR $1 <% cc.name
                      OR cc.french_name ILIKE '%' || $1 || '%' OR $1 <% cc.french_name)
               ORDER BY GREATEST(COALESCE(word_similarity($1, cc.name), 0),
                                 COALESCE(word_similarity($1, cc.french_name), 0)) DESC,
                        cc.set_code,
                        SUBSTRING(cc.collector_number FROM '^[0-9]+')::BIGINT NULLS LAST,
                        cc.collector_number
               LIMIT $3 OFFSET $4"#,
            name,
            set_code,
            i64::from(pagination.limit()),
            i64::from(pagination.offset()),
        )
        .fetch_all(&self.pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|row| {
                Ok(CatalogSearchItem {
                    set: SetName::new(SetCode::try_new(row.set_code)?, row.set_name),
                    collector_number: row.collector_number,
                    rarity: from_db_rarity(&row.rarity)?,
                    name: row.name,
                    french_name: row.french_name,
                    image_url: row.image_url,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(Paginated {
            items,
            total: u64::try_from(total)
                .map_err(|_| invalid_db_value("catalog total", &total.to_string()))?,
            pagination,
        })
    }

    #[tracing::instrument(
        name = "catalog_repo.scan_index_entries",
        skip_all,
        fields(sentry.op = "db")
    )]
    async fn scan_index_entries(&self) -> Result<Vec<ScanIndexEntry>, AppError> {
        // The numeric order of the numbers: the cast is safe, the filter keeps up to 10 digits.
        let rows = sqlx::query!(
            r#"SELECT set_code, collector_number, rarity, name, french_name
               FROM catalog_card
               WHERE collector_number ~ '^[0-9]+$'
               ORDER BY set_code, collector_number::BIGINT"#
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(ScanIndexEntry {
                    set_code: SetCode::try_new(row.set_code)?,
                    collector_number: row.collector_number,
                    rarity: from_db_rarity(&row.rarity)?,
                    name: row.name,
                    french_name: row.french_name,
                })
            })
            .collect()
    }
}

fn trend_from_db(trend: i32) -> Result<u32, AppError> {
    u32::try_from(trend).map_err(|_| invalid_db_value("trend price", &trend.to_string()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::rarity_code::RarityCode;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        insert_card, insert_collection_entry, insert_set, insert_user,
    };
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

    fn set(code: &str) -> SetName {
        SetName::new(code, format!("Catalog {code}"))
    }

    fn catalog_card(set_code: &str, number: &str) -> CatalogCard {
        CatalogCard {
            set_code: SetCode::new(set_code),
            collector_number: number.to_string(),
            rarity: RarityCode::R,
            name: format!("Card {number}"),
            french_name: Some(format!("Carte {number}")),
            scryfall_id: Uuid::new_v4(),
            cardmarket_id: Some(42),
            image_url: Some(format!("https://img.example/{set_code}/{number}.jpg")),
        }
    }

    /// A card with the names given, to search by.
    fn named(set_code: &str, number: &str, name: &str, french_name: Option<&str>) -> CatalogCard {
        CatalogCard {
            name: name.to_string(),
            french_name: french_name.map(str::to_string),
            ..catalog_card(set_code, number)
        }
    }

    fn keys(page: &Paginated<CatalogSearchItem>) -> Vec<String> {
        page.items
            .iter()
            .map(|item| format!("{}/{}", item.set.code, item.collector_number))
            .collect()
    }

    fn first_page() -> Pagination {
        Pagination::try_new(0, 20, 1_000).unwrap()
    }

    async fn catalog_keys(pool: &PgPool) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT set_code || '/' || collector_number FROM catalog_card ORDER BY 1",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn count(pool: &PgPool, table: Table) -> i64 {
        let sql = match table {
            Table::CatalogCard => "SELECT COUNT(*) FROM catalog_card",
            Table::Card => "SELECT COUNT(*) FROM card",
            Table::CollectionEntry => "SELECT COUNT(*) FROM collection_entry",
            Table::SetName => "SELECT COUNT(*) FROM set_name",
        };
        sqlx::query_scalar::<_, i64>(sql)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    enum Table {
        CatalogCard,
        Card,
        CollectionEntry,
        SetName,
    }

    #[sqlx::test]
    async fn replace_all_writes_the_catalog_cards_and_their_sets(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        let card = catalog_card("MKM", "12");

        adapter
            .replace_all(&[set("MKM")], std::slice::from_ref(&card))
            .await
            .unwrap();

        let row: (
            String,
            String,
            Option<String>,
            Uuid,
            Option<i32>,
            Option<String>,
        ) = sqlx::query_as(
            "SELECT rarity, name, french_name, scryfall_id, cardmarket_id, image_url
                 FROM catalog_card WHERE set_code = 'MKM' AND collector_number = '12'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            row,
            (
                "R".to_string(),
                "Card 12".to_string(),
                Some("Carte 12".to_string()),
                card.scryfall_id,
                Some(42),
                Some("https://img.example/MKM/12.jpg".to_string())
            )
        );
        let set_name: String =
            sqlx::query_scalar("SELECT name FROM set_name WHERE set_code = 'MKM'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(set_name, "Catalog MKM");
    }

    #[sqlx::test]
    async fn a_six_character_set_code_fits_the_catalog_and_an_owned_card(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());

        adapter
            .replace_all(&[set("PMPS06")], &[catalog_card("PMPS06", "1")])
            .await
            .unwrap();
        insert_user(&pool, "user-1", "player").await;
        insert_card(&pool, "PMPS06", "1", "JA", "Card 1", 1).await;
        insert_collection_entry(
            &pool,
            "PMPS06",
            "1",
            "JA",
            false,
            "user-1",
            1,
            100,
            Utc::now(),
        )
        .await;

        assert_eq!(catalog_keys(&pool).await, vec!["PMPS06/1"]);
        assert_eq!(count(&pool, Table::CollectionEntry).await, 1);
    }

    #[sqlx::test]
    async fn a_catalog_card_gone_from_the_new_catalog_leaves_it_without_touching_its_card(
        pool: PgPool,
    ) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        adapter
            .replace_all(
                &[set("MKM")],
                &[catalog_card("MKM", "1"), catalog_card("MKM", "2")],
            )
            .await
            .unwrap();
        insert_card(&pool, "MKM", "2", "fr", "Owned Card", 7).await;
        insert_user(&pool, "user-1", "User1").await;
        insert_collection_entry(&pool, "MKM", "2", "fr", false, "user-1", 1, 100, Utc::now()).await;

        adapter
            .replace_all(&[set("MKM")], &[catalog_card("MKM", "1")])
            .await
            .unwrap();

        assert_eq!(catalog_keys(&pool).await, vec!["MKM/1"]);
        assert_eq!(count(&pool, Table::Card).await, 1);
        assert_eq!(count(&pool, Table::CollectionEntry).await, 1);
    }

    #[sqlx::test]
    async fn replace_all_never_creates_a_card(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());

        adapter
            .replace_all(&[set("MKM")], &[catalog_card("MKM", "1")])
            .await
            .unwrap();

        assert_eq!(count(&pool, Table::Card).await, 0);
    }

    #[sqlx::test]
    async fn a_failing_replacement_keeps_the_previous_catalog_and_the_sets(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        adapter
            .replace_all(&[set("MKM")], &[catalog_card("MKM", "1")])
            .await
            .unwrap();

        // The same set and number twice breaks the primary key, after the old catalog was cleared.
        let result = adapter
            .replace_all(
                &[set("MKM"), set("ECL")],
                &[catalog_card("ECL", "5"), catalog_card("ECL", "5")],
            )
            .await;

        assert!(result.is_err());
        assert_eq!(catalog_keys(&pool).await, vec!["MKM/1"]);
        assert_eq!(
            count(&pool, Table::SetName).await,
            3,
            "ECL and FDN from the first migration, and MKM"
        );
    }

    #[sqlx::test]
    async fn the_name_of_a_known_set_is_not_overwritten(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        insert_set(&pool, "MKM").await;

        adapter
            .replace_all(&[set("MKM")], &[catalog_card("MKM", "1")])
            .await
            .unwrap();

        let name: String = sqlx::query_scalar("SELECT name FROM set_name WHERE set_code = 'MKM'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name, "Set MKM");
    }

    #[sqlx::test]
    async fn find_returns_the_catalog_card_of_a_set_and_number(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        let mut without_cardmarket_id = catalog_card("MKM", "13");
        without_cardmarket_id.french_name = None;
        without_cardmarket_id.cardmarket_id = None;
        without_cardmarket_id.image_url = None;
        let card = catalog_card("MKM", "12");
        adapter
            .replace_all(
                &[set("MKM")],
                &[card.clone(), without_cardmarket_id.clone()],
            )
            .await
            .unwrap();

        let found = adapter.find(&SetCode::new("MKM"), "12").await.unwrap();
        let found_bare = adapter.find(&SetCode::new("MKM"), "13").await.unwrap();

        assert_eq!(found, Some(card));
        assert_eq!(found_bare, Some(without_cardmarket_id));
    }

    #[sqlx::test]
    async fn find_returns_nothing_for_an_unknown_set_or_number(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        adapter
            .replace_all(&[set("MKM")], &[catalog_card("MKM", "12")])
            .await
            .unwrap();

        assert_eq!(
            adapter.find(&SetCode::new("MKM"), "13").await.unwrap(),
            None
        );
        assert_eq!(
            adapter.find(&SetCode::new("TMKM"), "12").await.unwrap(),
            None
        );
    }

    #[sqlx::test]
    async fn the_scan_index_keeps_only_the_purely_numeric_collector_numbers(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        let mut french_less = catalog_card("MKM", "7");
        french_less.french_name = None;
        french_less.rarity = RarityCode::C;
        adapter
            .replace_all(
                &[set("MKM"), set("PLST")],
                &[
                    catalog_card("MKM", "12"),
                    french_less,
                    catalog_card("MKM", "H13"),
                    catalog_card("MKM", "★"),
                    catalog_card("MKM", "12a"),
                    catalog_card("PLST", "100"),
                ],
            )
            .await
            .unwrap();

        let entries = adapter.scan_index_entries().await.unwrap();

        let keys: Vec<String> = entries
            .iter()
            .map(|e| format!("{}/{}", e.set_code, e.collector_number))
            .collect();
        assert_eq!(keys, vec!["MKM/7", "MKM/12", "PLST/100"]);
        assert_eq!(
            entries[1],
            ScanIndexEntry {
                set_code: SetCode::new("MKM"),
                collector_number: "12".to_string(),
                rarity: RarityCode::R,
                name: "Card 12".to_string(),
                french_name: Some("Carte 12".to_string()),
            }
        );
        assert_eq!(entries[0].french_name, None);
        assert_eq!(entries[0].rarity, RarityCode::C);
    }

    #[sqlx::test]
    async fn the_scan_index_is_empty_without_a_catalog(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool);

        assert!(adapter.scan_index_entries().await.unwrap().is_empty());
    }

    // ---- search ----

    async fn catalog_with(pool: &PgPool, cards: &[CatalogCard]) -> CatalogRepositoryAdapter {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        let mut sets: Vec<SetName> = cards.iter().map(|c| set(&c.set_code.to_string())).collect();
        sets.sort_by_key(|s| s.code.to_string());
        sets.dedup_by_key(|s| s.code.to_string());
        adapter.replace_all(&sets, cards).await.unwrap();
        adapter
    }

    #[sqlx::test]
    async fn search_finds_a_card_by_its_english_name(pool: PgPool) {
        let adapter = catalog_with(
            &pool,
            &[
                named("LEA", "161", "Lightning Bolt", Some("Foudre")),
                named("LEA", "162", "Serra Angel", Some("Ange de Serra")),
            ],
        )
        .await;

        let page = adapter
            .search(Some("lightning"), None, first_page())
            .await
            .unwrap();

        assert_eq!(keys(&page), vec!["LEA/161"]);
        assert_eq!(page.total, 1);
        let item = &page.items[0];
        assert_eq!(item.set, SetName::new("LEA", "Catalog LEA"));
        assert_eq!(item.rarity, RarityCode::R);
        assert_eq!(item.name, "Lightning Bolt");
        assert_eq!(item.french_name.as_deref(), Some("Foudre"));
        assert_eq!(
            item.image_url.as_deref(),
            Some("https://img.example/LEA/161.jpg")
        );
    }

    #[sqlx::test]
    async fn search_finds_a_card_by_its_french_name(pool: PgPool) {
        let adapter = catalog_with(
            &pool,
            &[
                named("LEA", "161", "Lightning Bolt", Some("Foudre")),
                named("LEA", "162", "Serra Angel", Some("Ange de Serra")),
                named("LEA", "163", "No French Card", None),
            ],
        )
        .await;

        let page = adapter
            .search(Some("ange de"), None, first_page())
            .await
            .unwrap();

        assert_eq!(keys(&page), vec!["LEA/162"]);
    }

    #[sqlx::test]
    async fn search_tolerates_a_typo_through_trigram_similarity(pool: PgPool) {
        let adapter = catalog_with(
            &pool,
            &[
                named("LEA", "161", "Lightning Bolt", Some("Foudre")),
                named("LEA", "162", "Serra Angel", Some("Ange de Serra")),
            ],
        )
        .await;

        let page = adapter
            .search(Some("lightnig bolt"), None, first_page())
            .await
            .unwrap();

        assert_eq!(keys(&page), vec!["LEA/161"]);
    }

    #[sqlx::test]
    async fn search_lists_the_impressions_of_a_name_by_set_then_number(pool: PgPool) {
        let adapter = catalog_with(
            &pool,
            &[
                named("MKM", "9", "Lightning Bolt", None),
                named("LEA", "161", "Lightning Bolt", None),
                named("LEA", "20", "Lightning Bolt", None),
                named("LEA", "H3", "Lightning Bolt", None),
            ],
        )
        .await;

        let page = adapter
            .search(Some("Lightning Bolt"), None, first_page())
            .await
            .unwrap();

        // The numeric order of the numbers, not the alphabetical one; a non-numeric one last.
        assert_eq!(keys(&page), vec!["LEA/20", "LEA/161", "LEA/H3", "MKM/9"]);
    }

    #[sqlx::test]
    async fn search_ranks_the_closest_name_before_the_set_and_the_number(pool: PgPool) {
        let adapter = catalog_with(
            &pool,
            &[
                named("LEA", "1", "Mangeur de rêves", None),
                named("MKM", "9", "Serra Angel", Some("Ange de Serra")),
            ],
        )
        .await;

        let page = adapter
            .search(Some("ange"), None, first_page())
            .await
            .unwrap();

        // Both match ("ange" is inside "Mangeur" and is a word of "Ange de Serra"): the word wins
        // over the set order.
        assert_eq!(keys(&page), vec!["MKM/9", "LEA/1"]);
    }

    #[sqlx::test]
    async fn search_filters_on_a_set(pool: PgPool) {
        let adapter = catalog_with(
            &pool,
            &[
                named("MKM", "9", "Lightning Bolt", None),
                named("LEA", "161", "Lightning Bolt", None),
                named("LEA", "162", "Serra Angel", None),
            ],
        )
        .await;

        let by_name = adapter
            .search(Some("lightning"), Some(&SetCode::new("LEA")), first_page())
            .await
            .unwrap();
        let whole_set = adapter
            .search(None, Some(&SetCode::new("LEA")), first_page())
            .await
            .unwrap();

        assert_eq!(keys(&by_name), vec!["LEA/161"]);
        assert_eq!(keys(&whole_set), vec!["LEA/161", "LEA/162"]);
    }

    #[sqlx::test]
    async fn search_is_paginated_with_the_total_of_every_page(pool: PgPool) {
        let cards: Vec<CatalogCard> = (1..=5)
            .map(|n| named("LEA", &n.to_string(), "Lightning Bolt", None))
            .collect();
        let adapter = catalog_with(&pool, &cards).await;

        let second = adapter
            .search(
                Some("lightning"),
                None,
                Pagination::try_new(1, 2, 1_000).unwrap(),
            )
            .await
            .unwrap();
        let beyond = adapter
            .search(
                Some("lightning"),
                None,
                Pagination::try_new(9, 2, 1_000).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(keys(&second), vec!["LEA/3", "LEA/4"]);
        assert_eq!(second.total, 5);
        assert_eq!(second.pagination.page(), 1);
        assert!(beyond.items.is_empty());
    }

    #[sqlx::test]
    async fn search_finds_nothing_for_an_unknown_name(pool: PgPool) {
        let adapter = catalog_with(&pool, &[named("LEA", "161", "Lightning Bolt", None)]).await;

        let page = adapter
            .search(Some("zzzzzz"), None, first_page())
            .await
            .unwrap();

        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
    }

    #[sqlx::test]
    async fn a_large_catalog_is_written_in_one_replacement(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        let cards: Vec<CatalogCard> = (0..12_000)
            .map(|n| catalog_card("MKM", &n.to_string()))
            .collect();

        adapter.replace_all(&[set("MKM")], &cards).await.unwrap();

        assert_eq!(count(&pool, Table::CatalogCard).await, 12_000);
    }

    // ---- trend_prices ----

    async fn price(pool: &PgPool, id_produit: i32, date: &str, trend: i32, trend_foil: i32) {
        sqlx::query(
            "INSERT INTO cardmarket_price (id_produit, date, trend, trend_foil)
             VALUES ($1, $2::date, $3, $4)",
        )
        .bind(id_produit)
        .bind(date)
        .bind(trend)
        .bind(trend_foil)
        .execute(pool)
        .await
        .unwrap();
    }

    fn key(set_code: &str, number: &str) -> (SetCode, String) {
        (SetCode::new(set_code), number.to_string())
    }

    #[sqlx::test]
    async fn trend_prices_of_a_card_nobody_owns_come_from_its_cardmarket_id(pool: PgPool) {
        let adapter = catalog_with(&pool, &[catalog_card("MKM", "12")]).await;
        price(&pool, 42, "2026-11-12", 120, 900).await;

        let prices = adapter.trend_prices(&[key("MKM", "12")]).await.unwrap();

        assert_eq!(
            prices,
            vec![TrendPrices {
                trend_cents: Some(120),
                foil_trend_cents: Some(900),
            }]
        );
    }

    #[sqlx::test]
    async fn trend_prices_read_the_latest_price_of_the_card(pool: PgPool) {
        let adapter = catalog_with(&pool, &[catalog_card("MKM", "12")]).await;
        price(&pool, 42, "2026-11-10", 100, 800).await;
        price(&pool, 42, "2026-11-12", 120, 900).await;
        price(&pool, 42, "2026-11-11", 110, 850).await;

        let prices = adapter.trend_prices(&[key("MKM", "12")]).await.unwrap();

        assert_eq!(prices[0].trend_cents, Some(120));
    }

    #[sqlx::test]
    async fn trend_prices_are_absent_without_catalog_card_cardmarket_id_or_price(pool: PgPool) {
        let without_id = CatalogCard {
            cardmarket_id: None,
            ..catalog_card("MKM", "13")
        };
        let without_price = CatalogCard {
            cardmarket_id: Some(43),
            ..catalog_card("MKM", "14")
        };
        let adapter = catalog_with(
            &pool,
            &[catalog_card("MKM", "12"), without_id, without_price],
        )
        .await;
        price(&pool, 42, "2026-11-12", 120, 900).await;

        let prices = adapter
            .trend_prices(&[
                key("MKM", "99"),
                key("MKM", "13"),
                key("MKM", "14"),
                key("MKM", "12"),
            ])
            .await
            .unwrap();

        let absent = TrendPrices::default();
        assert_eq!(prices[..3], [absent, absent, absent]);
        assert_eq!(prices[3].trend_cents, Some(120));
    }

    #[sqlx::test]
    async fn trend_prices_without_keys_are_empty(pool: PgPool) {
        let adapter = catalog_with(&pool, &[catalog_card("MKM", "12")]).await;

        assert!(adapter.trend_prices(&[]).await.unwrap().is_empty());
    }
}
