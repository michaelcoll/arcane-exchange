use crate::application::error::AppError;
use crate::application::repository::CollectionEntryRepository;
use crate::application::use_case::GetCollectionEntriesUseCase;
use crate::domain::card::CardId;
use crate::domain::collection_entry::CardEntries;
use crate::domain::user::UserId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetCollectionEntriesService {
    repository: Arc<dyn CollectionEntryRepository>,
}

impl GetCollectionEntriesService {
    pub fn new(repository: Arc<dyn CollectionEntryRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetCollectionEntriesUseCase for GetCollectionEntriesService {
    async fn get_collection_entries(
        &self,
        user_id: &UserId,
        card_id: CardId,
    ) -> Result<CardEntries, AppError> {
        self.repository.find_by_card(user_id, &card_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockCollectionEntryRepository;
    use crate::domain::language_code::LanguageCode;

    #[tokio::test]
    async fn returns_the_entries_of_the_repository_for_the_card_and_the_player() {
        let mut repository = MockCollectionEntryRepository::new();
        repository
            .expect_find_by_card()
            .withf(|user_id, card_id| {
                user_id.as_str() == "user_1"
                    && card_id == &CardId::new("FDN", "1", LanguageCode::FR)
            })
            .times(1)
            .returning(|_, _| {
                Box::pin(async {
                    Ok(CardEntries {
                        reserved_foil: 2,
                        ..CardEntries::default()
                    })
                })
            });
        let service = GetCollectionEntriesService::new(Arc::new(repository));

        let entries = service
            .get_collection_entries(
                &UserId::new("user_1"),
                CardId::new("FDN", "1", LanguageCode::FR),
            )
            .await
            .unwrap();

        assert_eq!(entries.reserved_foil, 2);
    }
}
