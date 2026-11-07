use crate::application::error::AppError;
use crate::application::repository::CatalogRepository;
use crate::domain::catalog::{CatalogCard, ScanIndexEntry};
use crate::domain::set_name::{SetCode, SetName};
use crate::infrastructure::adapter_out::repository::entities::{
    cardmarket_id_from_db, cardmarket_id_to_db, from_db_rarity,
};
use async_trait::async_trait;
use sqlx::{Pool, Postgres, QueryBuilder};

/// Rows per `INSERT`: 7 binds each stay far under Postgres's 65535 parameters.
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
                 scryfall_id, cardmarket_id) ",
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
                        .push_bind(cardmarket_id);
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
            "SELECT rarity, name, french_name, scryfall_id, cardmarket_id
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
                })
            })
            .transpose()?)
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
        }
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

        let row: (String, String, Option<String>, Uuid, Option<i32>) = sqlx::query_as(
            "SELECT rarity, name, french_name, scryfall_id, cardmarket_id
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
                Some(42)
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

    #[sqlx::test]
    async fn a_large_catalog_is_written_in_one_replacement(pool: PgPool) {
        let adapter = CatalogRepositoryAdapter::new(pool.clone());
        let cards: Vec<CatalogCard> = (0..12_000)
            .map(|n| catalog_card("MKM", &n.to_string()))
            .collect();

        adapter.replace_all(&[set("MKM")], &cards).await.unwrap();

        assert_eq!(count(&pool, Table::CatalogCard).await, 12_000);
    }
}
