use crate::application::error::AppError;
use crate::application::repository::{
    CardPricesViewRepository, CardRepository, SetNameRepository, TradingBinderRepository,
};
use crate::application::service::parse_service::parse_cards;
use crate::application::use_case::{
    EnqueueCardMarketIdUpdateUseCase, EnqueueGathererIdUpdateUseCase, ImportCardUseCase,
};
use crate::domain::user::User;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ImportCardService {
    card_repository: Arc<dyn CardRepository>,
    set_name_repository: Arc<dyn SetNameRepository>,
    enqueue_cardmarket_ids: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
    enqueue_gatherer_ids: Arc<dyn EnqueueGathererIdUpdateUseCase>,
    card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
    trading_binder_repository: Arc<dyn TradingBinderRepository>,
}

impl ImportCardService {
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        set_name_repository: Arc<dyn SetNameRepository>,
        enqueue_cardmarket_ids: Arc<dyn EnqueueCardMarketIdUpdateUseCase>,
        enqueue_gatherer_ids: Arc<dyn EnqueueGathererIdUpdateUseCase>,
        card_prices_view_repository: Arc<dyn CardPricesViewRepository>,
        trading_binder_repository: Arc<dyn TradingBinderRepository>,
    ) -> Self {
        Self {
            card_repository,
            set_name_repository,
            enqueue_cardmarket_ids,
            enqueue_gatherer_ids,
            card_prices_view_repository,
            trading_binder_repository,
        }
    }
}

#[async_trait]
impl ImportCardUseCase for ImportCardService {
    async fn import_cards(&self, csv: &str, user: User) -> Result<(), AppError> {
        let cards = parse_cards(csv)?;

        self.card_repository.delete_all(user.clone()).await?;

        for imported in cards {
            if !self
                .set_name_repository
                .exists_by_code(imported.card.id.set_code.clone())
                .await?
            {
                let set_name = imported.card.set_name.clone();
                self.set_name_repository.save(set_name).await?;
            }

            self.card_repository.save(user.clone(), imported).await?;
        }

        self.trading_binder_repository
            .purge_missing(&user.id)
            .await?;

        self.enqueue_cardmarket_ids
            .enqueue_pending_updates()
            .await?;
        self.enqueue_gatherer_ids.enqueue_pending_updates().await?;
        self.card_prices_view_repository.refresh().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::imported_card::ImportedCard;
    use crate::application::repository::{
        MockCardPricesViewRepository, MockCardRepository, MockSetNameRepository,
        MockTradingBinderRepository,
    };
    use crate::application::use_case::{
        MockEnqueueCardMarketIdUpdateUseCase, MockEnqueueGathererIdUpdateUseCase,
    };
    use crate::domain::card::{Card, CollectionEntry};
    use crate::domain::language_code::LanguageCode;
    use crate::domain::rarity_code::RarityCode;
    use crate::domain::set_name::{SetCode, SetName};
    use chrono::{DateTime, Utc};
    use mockall::predicate::eq;
    use uuid::Uuid;

    #[tokio::test]
    async fn import_cards_saves_cards_and_set_names_successfully() {
        let mut card_repository = MockCardRepository::new();
        let mut set_name_repository = MockSetNameRepository::new();
        let mut enqueue_use_case = MockEnqueueCardMarketIdUpdateUseCase::new();
        let mut enqueue_gatherer_use_case = MockEnqueueGathererIdUpdateUseCase::new();

        let set_code = SetCode::new("FDN");
        let set_name = SetName {
            code: set_code.clone(),
            name: "Foundations".to_string(),
        };
        let card = Card::new_full(
            set_code.clone(),
            "Foundations",
            "87",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            Uuid::parse_str("4409a063-bf2a-4a49-803e-3ce6bd474353").unwrap(),
            None,
            None,
            CollectionEntry::Mine {
                quantity: 3,
                purchase_price: 8,
                added_at: DateTime::parse_from_rfc3339("2026-02-05T20:44:45.815Z")
                    .unwrap()
                    .with_timezone(&Utc),
                reserved: false,
            },
        );
        let imported_card = ImportedCard {
            card: card.clone(),
            binder_name: Some("bulk".to_string()),
        };

        card_repository
            .expect_delete_all()
            .with(eq(User::for_testing()))
            .returning(|_| Box::pin(async { Ok(()) }));
        set_name_repository
            .expect_exists_by_code()
            .with(eq(set_code.clone()))
            .returning(|_| Box::pin(async { Ok(false) }));
        set_name_repository
            .expect_save()
            .with(eq(set_name.clone()))
            .returning(|_| Box::pin(async { Ok(()) }));
        card_repository
            .expect_save()
            .with(eq(User::for_testing()), eq(imported_card.clone()))
            .returning(|_, _| Box::pin(async { Ok(()) }));
        enqueue_use_case
            .expect_enqueue_pending_updates()
            .returning(|| Box::pin(async { Ok(2) }));
        enqueue_gatherer_use_case
            .expect_enqueue_pending_updates()
            .returning(|| Box::pin(async { Ok(2) }));

        let mut card_prices_view_repository = MockCardPricesViewRepository::new();
        card_prices_view_repository
            .expect_refresh()
            .returning(|| Box::pin(async { Ok(()) }));

        let mut trading_binder_repository = MockTradingBinderRepository::new();
        trading_binder_repository
            .expect_purge_missing()
            .with(eq(User::for_testing().id))
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));

        let service = ImportCardService::new(
            Arc::new(card_repository),
            Arc::new(set_name_repository),
            Arc::new(enqueue_use_case),
            Arc::new(enqueue_gatherer_use_case),
            Arc::new(card_prices_view_repository),
            Arc::new(trading_binder_repository),
        );

        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
        bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";
        let result = service.import_cards(csv, User::for_testing()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn import_cards_calls_purge_missing_once_with_importing_user() {
        let mut card_repository = MockCardRepository::new();
        let mut set_name_repository = MockSetNameRepository::new();
        let mut enqueue_use_case = MockEnqueueCardMarketIdUpdateUseCase::new();
        let mut enqueue_gatherer_use_case = MockEnqueueGathererIdUpdateUseCase::new();

        let set_code = SetCode::new("FDN");
        let set_name = SetName {
            code: set_code.clone(),
            name: "Foundations".to_string(),
        };
        let card = Card::new_full(
            set_code.clone(),
            "Foundations",
            "87",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            Uuid::parse_str("4409a063-bf2a-4a49-803e-3ce6bd474353").unwrap(),
            None,
            None,
            CollectionEntry::Mine {
                quantity: 3,
                purchase_price: 8,
                added_at: DateTime::parse_from_rfc3339("2026-02-05T20:44:45.815Z")
                    .unwrap()
                    .with_timezone(&Utc),
                reserved: false,
            },
        );
        let imported_card = ImportedCard {
            card: card.clone(),
            binder_name: Some("bulk".to_string()),
        };

        card_repository
            .expect_delete_all()
            .with(eq(User::for_testing()))
            .returning(|_| Box::pin(async { Ok(()) }));
        set_name_repository
            .expect_exists_by_code()
            .with(eq(set_code.clone()))
            .returning(|_| Box::pin(async { Ok(false) }));
        set_name_repository
            .expect_save()
            .with(eq(set_name.clone()))
            .returning(|_| Box::pin(async { Ok(()) }));
        card_repository
            .expect_save()
            .with(eq(User::for_testing()), eq(imported_card.clone()))
            .returning(|_, _| Box::pin(async { Ok(()) }));
        enqueue_use_case
            .expect_enqueue_pending_updates()
            .returning(|| Box::pin(async { Ok(2) }));
        enqueue_gatherer_use_case
            .expect_enqueue_pending_updates()
            .returning(|| Box::pin(async { Ok(2) }));

        let mut card_prices_view_repository = MockCardPricesViewRepository::new();
        card_prices_view_repository
            .expect_refresh()
            .returning(|| Box::pin(async { Ok(()) }));

        let mut trading_binder_repository = MockTradingBinderRepository::new();
        trading_binder_repository
            .expect_purge_missing()
            .with(eq(User::for_testing().id))
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));

        let service = ImportCardService::new(
            Arc::new(card_repository),
            Arc::new(set_name_repository),
            Arc::new(enqueue_use_case),
            Arc::new(enqueue_gatherer_use_case),
            Arc::new(card_prices_view_repository),
            Arc::new(trading_binder_repository),
        );

        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
        bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";
        let result = service.import_cards(csv, User::for_testing()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn import_cards_rollback_on_card_save_error() {
        let mut card_repository = MockCardRepository::new();
        let mut set_name_repository = MockSetNameRepository::new();

        let set_code = SetCode::new("FDN");
        let set_name = SetName {
            code: set_code.clone(),
            name: "Foundations".to_string(),
        };
        let card = Card::new_full(
            set_code.clone(),
            "Foundations",
            "0",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            Uuid::parse_str("4409a063-bf2a-4a49-803e-3ce6bd474353").unwrap(),
            None,
            None,
            CollectionEntry::Mine {
                quantity: 0,
                purchase_price: 0,
                added_at: DateTime::parse_from_rfc3339("2026-02-05T20:44:45.815Z")
                    .unwrap()
                    .with_timezone(&Utc),
                reserved: false,
            },
        );
        let imported_card = ImportedCard {
            card: card.clone(),
            binder_name: Some("bulk".to_string()),
        };

        card_repository
            .expect_delete_all()
            .with(eq(User::for_testing()))
            .returning(|_| Box::pin(async { Ok(()) }));
        set_name_repository
            .expect_exists_by_code()
            .with(eq(set_name.code.clone()))
            .returning(|_| Box::pin(async { Ok(false) }));
        set_name_repository
            .expect_save()
            .with(eq(set_name.clone()))
            .returning(|_| Box::pin(async { Ok(()) }));
        card_repository
            .expect_save()
            .with(eq(User::for_testing()), eq(imported_card.clone()))
            .returning(|_, _| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "Save failed".to_string(),
                    )))
                })
            });

        let mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
        let mock_enqueue_gatherer = MockEnqueueGathererIdUpdateUseCase::new();
        let card_prices_view_repository = MockCardPricesViewRepository::new();
        let trading_binder_repository = MockTradingBinderRepository::new();

        let service = ImportCardService::new(
            Arc::new(card_repository),
            Arc::new(set_name_repository),
            Arc::new(mock_enqueue),
            Arc::new(mock_enqueue_gatherer),
            Arc::new(card_prices_view_repository),
            Arc::new(trading_binder_repository),
        );

        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
        bulk,binder,Goblin Boarders,FDN,Foundations,0,normal,common,0,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.00,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";
        let result = service.import_cards(csv, User::for_testing()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn import_cards_does_not_save_duplicate_set_name() {
        let mut card_repository = MockCardRepository::new();
        let mut set_name_repository = MockSetNameRepository::new();
        let mut enqueue_use_case = MockEnqueueCardMarketIdUpdateUseCase::new();
        let mut enqueue_gatherer_use_case = MockEnqueueGathererIdUpdateUseCase::new();

        let set_code = SetCode::new("FDN");
        let card = Card::new_full(
            set_code.clone(),
            "Foundations",
            "87",
            LanguageCode::FR,
            false,
            "Goblin Boarders",
            RarityCode::C,
            Uuid::parse_str("4409a063-bf2a-4a49-803e-3ce6bd474353").unwrap(),
            None,
            None,
            CollectionEntry::Mine {
                quantity: 3,
                purchase_price: 8,
                added_at: DateTime::parse_from_rfc3339("2026-02-05T20:44:45.815Z")
                    .unwrap()
                    .with_timezone(&Utc),
                reserved: false,
            },
        );
        let imported_card = ImportedCard {
            card: card.clone(),
            binder_name: Some("bulk".to_string()),
        };

        card_repository
            .expect_delete_all()
            .returning(|_| Box::pin(async { Ok(()) }));
        set_name_repository
            .expect_exists_by_code()
            .with(eq(set_code.clone()))
            .returning(|_| Box::pin(async { Ok(true) }));
        card_repository
            .expect_save()
            .with(eq(User::for_testing()), eq(imported_card.clone()))
            .returning(|_, _| Box::pin(async { Ok(()) }));
        enqueue_use_case
            .expect_enqueue_pending_updates()
            .returning(|| Box::pin(async { Ok(1) }));
        enqueue_gatherer_use_case
            .expect_enqueue_pending_updates()
            .returning(|| Box::pin(async { Ok(1) }));

        let mut card_prices_view_repository = MockCardPricesViewRepository::new();
        card_prices_view_repository
            .expect_refresh()
            .returning(|| Box::pin(async { Ok(()) }));

        let mut trading_binder_repository = MockTradingBinderRepository::new();
        trading_binder_repository
            .expect_purge_missing()
            .with(eq(User::for_testing().id))
            .times(1)
            .returning(|_| Box::pin(async { Ok(()) }));

        let service = ImportCardService::new(
            Arc::new(card_repository),
            Arc::new(set_name_repository),
            Arc::new(enqueue_use_case),
            Arc::new(enqueue_gatherer_use_case),
            Arc::new(card_prices_view_repository),
            Arc::new(trading_binder_repository),
        );

        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
        bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";
        let result = service.import_cards(csv, User::for_testing()).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn import_cards_fails_on_parse_error() {
        let card_repository = MockCardRepository::new();
        let set_name_repository = MockSetNameRepository::new();
        let mock_enqueue = MockEnqueueCardMarketIdUpdateUseCase::new();
        let mock_enqueue_gatherer = MockEnqueueGathererIdUpdateUseCase::new();
        let card_prices_view_repository = MockCardPricesViewRepository::new();
        let trading_binder_repository = MockTradingBinderRepository::new();

        let service = ImportCardService::new(
            Arc::new(card_repository),
            Arc::new(set_name_repository),
            Arc::new(mock_enqueue),
            Arc::new(mock_enqueue_gatherer),
            Arc::new(card_prices_view_repository),
            Arc::new(trading_binder_repository),
        );

        let invalid_csv = "Invalid,Data";
        let result = service.import_cards(invalid_csv, User::for_testing()).await;

        assert!(result.is_err());
    }
}
