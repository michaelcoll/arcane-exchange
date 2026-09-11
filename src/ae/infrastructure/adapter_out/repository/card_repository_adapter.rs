use crate::application::error::AppError;
use crate::application::imported_card::ImportedCard;
use crate::application::repository::CardRepository;
use crate::domain::card::{CardId, CollectionEntry};
use crate::domain::user::User;
use crate::infrastructure::adapter_out::repository::entities::{CardIdEntity, CardNameEntity};
use async_trait::async_trait;
use sqlx::{Pool, Postgres, QueryBuilder};
use std::collections::HashSet;

pub struct CardRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CardRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CardRepository for CardRepositoryAdapter {
    #[tracing::instrument(name = "card_repo.get_all_without_cardmarket_id", skip_all, fields(sentry.op = "db"))]
    async fn get_all_without_cardmarket_id(&self) -> Result<Vec<(CardId, uuid::Uuid)>, AppError> {
        Ok(sqlx::query_as!(
            CardIdEntity,
            "SELECT
                card.set_code,
                set_name.name as set_name,
                card.collector_number,
                card.language_code,
                card.scryfall_id
            FROM card
            JOIN set_name ON card.set_code = set_name.set_code
            WHERE card.cardmarket_id IS NULL"
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|e| (e.clone().into(), e.scryfall_id))
        .collect::<Vec<(CardId, uuid::Uuid)>>())
    }

    #[tracing::instrument(name = "card_repo.get_all_without_gatherer_id", skip_all, fields(sentry.op = "db"))]
    async fn get_all_without_gatherer_id(&self) -> Result<Vec<(CardId, String)>, AppError> {
        Ok(sqlx::query_as!(
            CardNameEntity,
            "SELECT
                card.set_code,
                card.collector_number,
                card.language_code,
                card.name
            FROM card
            WHERE card.the_gatherer_id IS NULL"
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|e| (e.clone().into(), e.name))
        .collect::<Vec<(CardId, String)>>())
    }

    #[tracing::instrument(name = "card_repo.find_by_scryfall_id", skip_all, fields(sentry.op = "db"))]
    async fn find_by_scryfall_id(
        &self,
        scryfall_id: uuid::Uuid,
    ) -> Result<Option<Option<u32>>, AppError> {
        // Several rows can share a scryfall_id — the same printing in several languages. The
        // cardmarket_id is identical across languages, but the pick must still be deterministic:
        // prefer the French row when it exists.
        let record = sqlx::query!(
            r#"SELECT cardmarket_id
               FROM card
               WHERE scryfall_id = $1
               ORDER BY (language_code = 'FR') DESC, language_code
               LIMIT 1"#,
            scryfall_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| r.cardmarket_id.map(|id| id as u32)))
    }

    #[tracing::instrument(name = "card_repo.save_all", skip_all, fields(sentry.op = "db"))]
    async fn save_all(&self, user: &User, cards: &[ImportedCard]) -> Result<(), AppError> {
        if cards.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // A card can appear once per binder AND in both finishes (see `ImportedCard` dedup key
        // in `parse_service`, which keeps foil), so the same `CardId` (finish aside) may show up
        // several times here. `INSERT ... ON CONFLICT DO UPDATE` cannot affect the same row
        // twice in one statement, so the `card` table only gets the first occurrence of each
        // catalog id — the following are identical game data anyway (only the finish differs,
        // and the catalog no longer tracks it).
        let mut seen_card_ids = HashSet::with_capacity(cards.len());
        let distinct_cards: Vec<&ImportedCard> = cards
            .iter()
            .filter(|imported| seen_card_ids.insert(imported.card.id.card_id.clone()))
            .collect();

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO card (set_code, collector_number, language_code, name, rarity, scryfall_id)",
        );
        qb.push_values(&distinct_cards, |mut b, imported| {
            let card = &imported.card;
            b.push_bind(card.id.card_id.set_code.to_string())
                .push_bind(card.id.card_id.collector_number.clone())
                .push_bind(card.id.card_id.language_code.to_string())
                .push_bind(card.name.clone())
                .push_bind(card.rarity_code.to_string())
                .push_bind(card.scryfall_id);
        });
        qb.push(
            "ON CONFLICT (set_code, collector_number, language_code)
             DO UPDATE SET name = EXCLUDED.name, rarity = EXCLUDED.rarity, scryfall_id = EXCLUDED.scryfall_id",
        );
        qb.build().execute(&mut *tx).await?;

        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO collection_entry
                (set_code, collector_number, language_code, foil, user_id, quantity, purchase_price, added_at, binder_name)",
        );
        qb.push_values(cards, |mut b, imported| {
            let card = &imported.card;
            let CollectionEntry::Mine {
                quantity,
                purchase_price,
                added_at,
                ..
            } = &card.collection_entry
            else {
                panic!("save_all() is only called for cards owned by the importing user");
            };
            b.push_bind(card.id.card_id.set_code.to_string())
                .push_bind(card.id.card_id.collector_number.clone())
                .push_bind(card.id.card_id.language_code.to_string())
                .push_bind(card.id.foil)
                .push_bind(user.id.as_str())
                .push_bind(*quantity as i32)
                .push_bind(*purchase_price as i32)
                .push_bind(*added_at)
                .push_bind(imported.binder_name.clone());
        });
        qb.push(
            "ON CONFLICT (set_code, collector_number, language_code, foil, user_id, binder_name)
             DO UPDATE SET quantity = EXCLUDED.quantity, purchase_price = EXCLUDED.purchase_price, added_at = EXCLUDED.added_at",
        );
        qb.build().execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    #[tracing::instrument(name = "card_repo.update_cardmarket_id", skip_all, fields(sentry.op = "db"))]
    async fn update_cardmarket_id(
        &self,
        id: CardId,
        cardmarket_id: Option<u32>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"UPDATE card
                SET cardmarket_id = $1
                WHERE set_code = $2 AND collector_number = $3 AND language_code = $4;"#,
            cardmarket_id.map(|id| id as i32),
            id.set_code.to_string(),
            id.collector_number,
            id.language_code.to_string()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "card_repo.update_gatherer_id", skip_all, fields(sentry.op = "db"))]
    async fn update_gatherer_id(
        &self,
        id: CardId,
        gatherer_id: Option<String>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"UPDATE card
                SET the_gatherer_id = $1
                WHERE set_code = $2 AND collector_number = $3 AND language_code = $4;"#,
            gatherer_id,
            id.set_code.to_string(),
            id.collector_number,
            id.language_code.to_string()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "card_repo.delete_all", skip_all, fields(sentry.op = "db"))]
    async fn delete_all(&self, user: User) -> Result<(), AppError> {
        sqlx::query!(
            "DELETE FROM collection_entry WHERE user_id = $1",
            user.id.as_str()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::card::Card;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::rarity_code::RarityCode;
    use crate::infrastructure::adapter_out::repository::common_repository_tests::{
        fetch_collection_entries, insert_card, insert_card_with_scryfall_id,
        insert_card_without_cardmarket_id, insert_collection_entry, insert_user,
    };
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

    #[sqlx::test]
    async fn save_all_updates_existing_card(pool: PgPool) {
        insert_user(&pool, "test-user-id", "testuser").await;
        let repository = CardRepositoryAdapter::new(pool.clone());

        let card = Card::new(
            "FDN",
            "Foundations",
            "87",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            3,
            500,
        );
        repository
            .save_all(
                &User::for_testing(),
                &[ImportedCard {
                    card,
                    binder_name: None,
                }],
            )
            .await
            .unwrap();

        let updated_card = Card::new(
            "FDN",
            "Foundations",
            "87",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            5,
            1500,
        );
        repository
            .save_all(
                &User::for_testing(),
                &[ImportedCard {
                    card: updated_card,
                    binder_name: None,
                }],
            )
            .await
            .unwrap();

        let rows = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].quantity, 5);
        assert_eq!(rows[0].purchase_price, 1500);
    }

    #[sqlx::test]
    async fn save_all_creates_distinct_rows_for_different_binder_names(pool: PgPool) {
        insert_user(&pool, "test-user-id", "testuser").await;
        let repository = CardRepositoryAdapter::new(pool.clone());

        let card = Card::new(
            "FDN",
            "Foundations",
            "87",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            2,
            100,
        );
        repository
            .save_all(
                &User::for_testing(),
                &[ImportedCard {
                    card: card.clone(),
                    binder_name: Some("Binder A".to_string()),
                }],
            )
            .await
            .unwrap();
        repository
            .save_all(
                &User::for_testing(),
                &[ImportedCard {
                    card,
                    binder_name: Some("Binder B".to_string()),
                }],
            )
            .await
            .unwrap();

        let rows = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].binder_name, Some("Binder A".to_string()));
        assert_eq!(rows[0].quantity, 2);
        assert_eq!(rows[1].binder_name, Some("Binder B".to_string()));
        assert_eq!(rows[1].quantity, 2);
    }

    #[sqlx::test]
    async fn save_all_creates_distinct_rows_for_named_and_null_binder(pool: PgPool) {
        insert_user(&pool, "test-user-id", "testuser").await;
        let repository = CardRepositoryAdapter::new(pool.clone());

        let card = Card::new(
            "FDN",
            "Foundations",
            "87",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            1,
            100,
        );
        repository
            .save_all(
                &User::for_testing(),
                &[ImportedCard {
                    card: card.clone(),
                    binder_name: Some("Binder A".to_string()),
                }],
            )
            .await
            .unwrap();
        repository
            .save_all(
                &User::for_testing(),
                &[ImportedCard {
                    card,
                    binder_name: None,
                }],
            )
            .await
            .unwrap();

        let rows = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(rows.len(), 2);
    }

    #[sqlx::test]
    async fn save_all_upserts_same_row_when_binder_name_is_null(pool: PgPool) {
        insert_user(&pool, "test-user-id", "testuser").await;
        let repository = CardRepositoryAdapter::new(pool.clone());

        let card = Card::new(
            "FDN",
            "Foundations",
            "87",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            1,
            100,
        );
        repository
            .save_all(
                &User::for_testing(),
                &[ImportedCard {
                    card: card.clone(),
                    binder_name: None,
                }],
            )
            .await
            .unwrap();
        repository
            .save_all(
                &User::for_testing(),
                &[ImportedCard {
                    card,
                    binder_name: None,
                }],
            )
            .await
            .unwrap();

        let rows = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(rows.len(), 1, "second save should upsert, not duplicate");
        assert_eq!(rows[0].quantity, 1);
    }

    #[sqlx::test]
    async fn delete_all_removes_all_cards(pool: PgPool) {
        insert_card_without_cardmarket_id(&pool, "FDN", "87", "FR", "Goblin Boarders").await;
        insert_card_without_cardmarket_id(&pool, "FDN", "12", "EN", "Goblin Boarders").await;
        insert_user(&pool, "test-user-id", "testuser").await;
        insert_collection_entry(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "test-user-id",
            3,
            500,
            Utc::now(),
        )
        .await;
        insert_collection_entry(
            &pool,
            "FDN",
            "12",
            "EN",
            true,
            "test-user-id",
            2,
            1000,
            Utc::now(),
        )
        .await;

        let repository = CardRepositoryAdapter::new(pool.clone());
        repository.delete_all(User::for_testing()).await.unwrap();

        let rows = fetch_collection_entries(&pool, "test-user-id").await;
        assert!(
            rows.is_empty(),
            "all cards should be deleted from the database"
        );
    }

    #[sqlx::test]
    async fn get_all_without_cardmarket_id_returns_only_cards_without_cardmarket_id(pool: PgPool) {
        insert_card_without_cardmarket_id(&pool, "FDN", "87", "FR", "Goblin Boarders").await;
        insert_card(&pool, "FDN", "12", "EN", "Goblin Boarders", 123).await;
        insert_user(&pool, "test-user-id", "testuser").await;
        insert_collection_entry(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "test-user-id",
            3,
            500,
            Utc::now(),
        )
        .await;
        insert_collection_entry(
            &pool,
            "FDN",
            "12",
            "EN",
            true,
            "test-user-id",
            2,
            1000,
            Utc::now(),
        )
        .await;

        let cards = CardRepositoryAdapter::new(pool)
            .get_all_without_cardmarket_id()
            .await
            .unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].0, CardId::new("FDN", "87", LanguageCode::FR));
    }

    #[sqlx::test]
    async fn get_all_without_gatherer_id_returns_only_cards_without_gatherer_id(pool: PgPool) {
        insert_card_without_cardmarket_id(&pool, "FDN", "87", "FR", "Goblin Boarders").await;
        insert_card(&pool, "FDN", "12", "EN", "Goblin Boarders", 123).await;

        let repository = CardRepositoryAdapter::new(pool);
        repository
            .update_gatherer_id(
                CardId::new("FDN", "12", LanguageCode::EN),
                Some("ABC123".to_string()),
            )
            .await
            .unwrap();

        let cards = repository.get_all_without_gatherer_id().await.unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].0, CardId::new("FDN", "87", LanguageCode::FR));
        assert_eq!(cards[0].1, "Goblin Boarders");
    }

    #[sqlx::test]
    async fn update_gatherer_id_sets_the_value(pool: PgPool) {
        insert_card_without_cardmarket_id(&pool, "FDN", "87", "FR", "Goblin Boarders").await;

        let repository = CardRepositoryAdapter::new(pool);
        let card_id = CardId::new("FDN", "87", LanguageCode::FR);
        repository
            .update_gatherer_id(card_id.clone(), Some("ABC123".to_string()))
            .await
            .unwrap();

        let remaining = repository.get_all_without_gatherer_id().await.unwrap();
        assert!(remaining.is_empty());
    }

    #[sqlx::test]
    async fn find_by_scryfall_id_returns_cardmarket_id_when_present(pool: PgPool) {
        let scryfall_id = Uuid::new_v4();
        insert_card_with_scryfall_id(
            &pool,
            "FDN",
            "87",
            "FR",
            "Goblin Boarders",
            scryfall_id,
            Some(123),
        )
        .await;

        let result = CardRepositoryAdapter::new(pool)
            .find_by_scryfall_id(scryfall_id)
            .await
            .unwrap();

        assert_eq!(result, Some(Some(123)));
    }

    #[sqlx::test]
    async fn find_by_scryfall_id_returns_none_cardmarket_id_when_not_linked(pool: PgPool) {
        let scryfall_id = Uuid::new_v4();
        insert_card_with_scryfall_id(
            &pool,
            "FDN",
            "87",
            "FR",
            "Goblin Boarders",
            scryfall_id,
            None,
        )
        .await;

        let result = CardRepositoryAdapter::new(pool)
            .find_by_scryfall_id(scryfall_id)
            .await
            .unwrap();

        assert_eq!(result, Some(None));
    }

    #[sqlx::test]
    async fn find_by_scryfall_id_is_deterministic_across_languages(pool: PgPool) {
        // Several rows (one per language) can share a scryfall_id — a printing's normal and
        // foil copies, in every language, all point to the same Scryfall card. The pick must
        // not depend on insertion or scan order.
        let scryfall_id = Uuid::new_v4();
        insert_card_with_scryfall_id(
            &pool,
            "FDN",
            "87",
            "EN",
            "Goblin Boarders",
            scryfall_id,
            Some(999),
        )
        .await;
        insert_card_with_scryfall_id(
            &pool,
            "FDN",
            "87",
            "FR",
            "Goblin Boarders",
            scryfall_id,
            Some(123),
        )
        .await;
        insert_card_with_scryfall_id(
            &pool,
            "FDN",
            "87",
            "DE",
            "Goblin Boarders",
            scryfall_id,
            Some(999),
        )
        .await;

        let result = CardRepositoryAdapter::new(pool)
            .find_by_scryfall_id(scryfall_id)
            .await
            .unwrap();

        assert_eq!(
            result,
            Some(Some(123)),
            "the French row must be preferred when it exists"
        );
    }

    #[sqlx::test]
    async fn find_by_scryfall_id_returns_none_when_card_unknown(pool: PgPool) {
        let result = CardRepositoryAdapter::new(pool)
            .find_by_scryfall_id(Uuid::new_v4())
            .await
            .unwrap();

        assert_eq!(result, None);
    }

    fn imported_card(
        collector_number: &str,
        quantity: u8,
        purchase_price: u32,
        binder_name: Option<&str>,
    ) -> ImportedCard {
        imported_card_with_foil(
            collector_number,
            false,
            quantity,
            purchase_price,
            binder_name,
        )
    }

    fn imported_card_with_foil(
        collector_number: &str,
        foil: bool,
        quantity: u8,
        purchase_price: u32,
        binder_name: Option<&str>,
    ) -> ImportedCard {
        ImportedCard {
            card: Card::new(
                "FDN",
                "Foundations",
                collector_number,
                LanguageCode::FR,
                foil,
                "Goblin Boarders",
                RarityCode::C,
                quantity,
                purchase_price,
            ),
            binder_name: binder_name.map(str::to_string),
        }
    }

    #[sqlx::test]
    async fn save_all_does_not_conflict_when_the_same_card_appears_in_both_finishes(pool: PgPool) {
        // Regression test for the `ON CONFLICT DO UPDATE` violation that `save_all`'s catalog
        // dedup key must avoid: a normal and a foil row of the same card in the same import
        // must not be inserted twice for the same `card` primary key in one statement.
        insert_user(&pool, "test-user-id", "testuser").await;
        let repository = CardRepositoryAdapter::new(pool.clone());

        let cards = vec![
            imported_card_with_foil("87", false, 3, 800, Some("bulk")),
            imported_card_with_foil("87", true, 1, 1500, Some("bulk")),
        ];

        repository
            .save_all(&User::for_testing(), &cards)
            .await
            .unwrap();

        let card_rows = sqlx::query!("SELECT collector_number FROM card WHERE set_code = 'FDN'")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(
            card_rows.len(),
            1,
            "one card row for the shared catalog identity"
        );

        let entries = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(
            entries.len(),
            2,
            "one collection_entry row per finish, not summed into one"
        );
    }

    #[sqlx::test]
    async fn save_all_does_not_error_when_the_same_card_appears_in_two_binders(pool: PgPool) {
        insert_user(&pool, "test-user-id", "testuser").await;
        let repository = CardRepositoryAdapter::new(pool.clone());

        let cards = vec![
            imported_card("87", 3, 800, Some("bulk")),
            imported_card("87", 1, 800, Some("deck")),
        ];

        repository
            .save_all(&User::for_testing(), &cards)
            .await
            .unwrap();

        let card_rows = sqlx::query!("SELECT collector_number FROM card WHERE set_code = 'FDN'")
            .fetch_all(&pool)
            .await
            .unwrap();
        assert_eq!(card_rows.len(), 1, "one card row for the shared CardId");

        let entries = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(entries.len(), 2, "one collection_entry row per binder");
    }

    #[sqlx::test]
    async fn save_all_upserts_quantity_and_price_like_save(pool: PgPool) {
        insert_user(&pool, "test-user-id", "testuser").await;
        let repository = CardRepositoryAdapter::new(pool.clone());

        repository
            .save_all(&User::for_testing(), &[imported_card("87", 3, 500, None)])
            .await
            .unwrap();
        repository
            .save_all(&User::for_testing(), &[imported_card("87", 5, 1500, None)])
            .await
            .unwrap();

        let entries = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].quantity, 5);
        assert_eq!(entries[0].purchase_price, 1500);
    }

    #[sqlx::test]
    async fn save_all_does_nothing_for_an_empty_slice(pool: PgPool) {
        let repository = CardRepositoryAdapter::new(pool);
        repository
            .save_all(&User::for_testing(), &[])
            .await
            .unwrap();
    }
}
