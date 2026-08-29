use crate::application::error::AppError;
use crate::application::imported_card::ImportedCard;
use crate::application::repository::CardRepository;
use crate::domain::card::{CardId, CollectionEntry};
use crate::domain::user::User;
use crate::infrastructure::adapter_out::repository::entities::{CardIdEntity, CardNameEntity};
use async_trait::async_trait;
use sqlx::{Pool, Postgres};

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
    async fn get_all_without_cardmarket_id(&self) -> Result<Vec<(CardId, uuid::Uuid)>, AppError> {
        Ok(sqlx::query_as!(
            CardIdEntity,
            "SELECT
                card.set_code,
                set_name.name as set_name,
                card.collector_number,
                card.language_code,
                card.foil,
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

    async fn get_all_without_gatherer_id(&self) -> Result<Vec<(CardId, String)>, AppError> {
        Ok(sqlx::query_as!(
            CardNameEntity,
            "SELECT
                card.set_code,
                card.collector_number,
                card.language_code,
                card.foil,
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

    async fn find_by_scryfall_id(
        &self,
        scryfall_id: uuid::Uuid,
    ) -> Result<Option<(Option<u32>, bool)>, AppError> {
        let record = sqlx::query!(
            "SELECT cardmarket_id, foil FROM card WHERE scryfall_id = $1",
            scryfall_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| (r.cardmarket_id.map(|id| id as u32), r.foil)))
    }

    async fn save(&self, user: User, card: ImportedCard) -> Result<(), AppError> {
        let ImportedCard { card, binder_name } = card;

        let CollectionEntry::Mine {
            quantity,
            purchase_price,
            added_at,
            ..
        } = &card.collection_entry
        else {
            panic!("save() is only called for cards owned by the importing user");
        };

        sqlx::query!(
        r#"INSERT INTO card (set_code, collector_number, language_code, foil, name, rarity, scryfall_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT(set_code, collector_number, language_code, foil)
                DO UPDATE
                SET name          = $5,
                    rarity        = $6,
                    scryfall_id   = $7"#,
            card.id.set_code.to_string(),
            card.id.collector_number,
            card.id.language_code.to_string(),
            card.id.foil,
            card.name,
            card.rarity_code.to_string(),
            card.scryfall_id,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query!(
        r#"INSERT INTO collection_entry (set_code, collector_number, language_code, foil, user_id, quantity, purchase_price, added_at, binder_name)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT(set_code, collector_number, language_code, foil, user_id, binder_name)
                DO UPDATE
                SET quantity       = $6,
                    purchase_price = $7,
                    added_at       = $8"#,
            card.id.set_code.to_string(),
            card.id.collector_number,
            card.id.language_code.to_string(),
            card.id.foil,
            user.id.as_str(),
            *quantity as i32,
            *purchase_price as i32,
            added_at,
            binder_name,
        )
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_cardmarket_id(
        &self,
        id: CardId,
        cardmarket_id: Option<u32>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"UPDATE card
                SET cardmarket_id = $1
                WHERE set_code = $2 AND collector_number = $3 AND language_code = $4 AND foil = $5;"#,
            cardmarket_id.map(|id| id as i32),
            id.set_code.to_string(),
            id.collector_number,
            id.language_code.to_string(),
            id.foil)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_gatherer_id(
        &self,
        id: CardId,
        gatherer_id: Option<String>,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"UPDATE card
                SET the_gatherer_id = $1
                WHERE set_code = $2 AND collector_number = $3 AND language_code = $4 AND foil = $5;"#,
            gatherer_id,
            id.set_code.to_string(),
            id.collector_number,
            id.language_code.to_string(),
            id.foil)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

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
    async fn save_card_updates_existing_card(pool: PgPool) {
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
            .save(
                User::for_testing(),
                ImportedCard {
                    card,
                    binder_name: None,
                },
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
            .save(
                User::for_testing(),
                ImportedCard {
                    card: updated_card,
                    binder_name: None,
                },
            )
            .await
            .unwrap();

        let rows = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].quantity, 5);
        assert_eq!(rows[0].purchase_price, 1500);
    }

    #[sqlx::test]
    async fn save_creates_distinct_rows_for_different_binder_names(pool: PgPool) {
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
            .save(
                User::for_testing(),
                ImportedCard {
                    card: card.clone(),
                    binder_name: Some("Binder A".to_string()),
                },
            )
            .await
            .unwrap();
        repository
            .save(
                User::for_testing(),
                ImportedCard {
                    card,
                    binder_name: Some("Binder B".to_string()),
                },
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
    async fn save_creates_distinct_rows_for_named_and_null_binder(pool: PgPool) {
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
            .save(
                User::for_testing(),
                ImportedCard {
                    card: card.clone(),
                    binder_name: Some("Binder A".to_string()),
                },
            )
            .await
            .unwrap();
        repository
            .save(
                User::for_testing(),
                ImportedCard {
                    card,
                    binder_name: None,
                },
            )
            .await
            .unwrap();

        let rows = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(rows.len(), 2);
    }

    #[sqlx::test]
    async fn save_upserts_same_row_when_binder_name_is_null(pool: PgPool) {
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
            .save(
                User::for_testing(),
                ImportedCard {
                    card: card.clone(),
                    binder_name: None,
                },
            )
            .await
            .unwrap();
        repository
            .save(
                User::for_testing(),
                ImportedCard {
                    card,
                    binder_name: None,
                },
            )
            .await
            .unwrap();

        let rows = fetch_collection_entries(&pool, "test-user-id").await;
        assert_eq!(rows.len(), 1, "second save should upsert, not duplicate");
        assert_eq!(rows[0].quantity, 1);
    }

    #[sqlx::test]
    async fn delete_all_removes_all_cards(pool: PgPool) {
        insert_card_without_cardmarket_id(&pool, "FDN", "87", "FR", false, "Goblin Boarders").await;
        insert_card_without_cardmarket_id(&pool, "FDN", "12", "EN", true, "Goblin Boarders").await;
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
        insert_card_without_cardmarket_id(&pool, "FDN", "87", "FR", false, "Goblin Boarders").await;
        insert_card(&pool, "FDN", "12", "EN", true, "Goblin Boarders", 123).await;
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
        assert_eq!(
            cards[0].0,
            CardId::new("FDN", "87", LanguageCode::FR, false)
        );
    }

    #[sqlx::test]
    async fn get_all_without_gatherer_id_returns_only_cards_without_gatherer_id(pool: PgPool) {
        insert_card_without_cardmarket_id(&pool, "FDN", "87", "FR", false, "Goblin Boarders").await;
        insert_card(&pool, "FDN", "12", "EN", true, "Goblin Boarders", 123).await;

        let repository = CardRepositoryAdapter::new(pool);
        repository
            .update_gatherer_id(
                CardId::new("FDN", "12", LanguageCode::EN, true),
                Some("ABC123".to_string()),
            )
            .await
            .unwrap();

        let cards = repository.get_all_without_gatherer_id().await.unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards[0].0,
            CardId::new("FDN", "87", LanguageCode::FR, false)
        );
        assert_eq!(cards[0].1, "Goblin Boarders");
    }

    #[sqlx::test]
    async fn update_gatherer_id_sets_the_value(pool: PgPool) {
        insert_card_without_cardmarket_id(&pool, "FDN", "87", "FR", false, "Goblin Boarders").await;

        let repository = CardRepositoryAdapter::new(pool);
        let card_id = CardId::new("FDN", "87", LanguageCode::FR, false);
        repository
            .update_gatherer_id(card_id.clone(), Some("ABC123".to_string()))
            .await
            .unwrap();

        let remaining = repository.get_all_without_gatherer_id().await.unwrap();
        assert!(remaining.is_empty());
    }

    #[sqlx::test]
    async fn find_by_scryfall_id_returns_cardmarket_id_and_foil_when_present(pool: PgPool) {
        let scryfall_id = Uuid::new_v4();
        insert_card_with_scryfall_id(
            &pool,
            "FDN",
            "87",
            "FR",
            true,
            "Goblin Boarders",
            scryfall_id,
            Some(123),
        )
        .await;

        let result = CardRepositoryAdapter::new(pool)
            .find_by_scryfall_id(scryfall_id)
            .await
            .unwrap();

        assert_eq!(result, Some((Some(123), true)));
    }

    #[sqlx::test]
    async fn find_by_scryfall_id_returns_none_cardmarket_id_when_not_linked(pool: PgPool) {
        let scryfall_id = Uuid::new_v4();
        insert_card_with_scryfall_id(
            &pool,
            "FDN",
            "87",
            "FR",
            false,
            "Goblin Boarders",
            scryfall_id,
            None,
        )
        .await;

        let result = CardRepositoryAdapter::new(pool)
            .find_by_scryfall_id(scryfall_id)
            .await
            .unwrap();

        assert_eq!(result, Some((None, false)));
    }

    #[sqlx::test]
    async fn find_by_scryfall_id_returns_none_when_card_unknown(pool: PgPool) {
        let result = CardRepositoryAdapter::new(pool)
            .find_by_scryfall_id(Uuid::new_v4())
            .await
            .unwrap();

        assert_eq!(result, None);
    }
}
