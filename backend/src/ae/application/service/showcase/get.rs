use crate::application::error::AppError;
use crate::application::repository::ShowcaseRepository;
use crate::application::use_case::GetShowcaseUseCase;
use crate::domain::showcase::ShowcaseCard;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ShowcaseService {
    repository: Arc<dyn ShowcaseRepository>,
}

impl ShowcaseService {
    pub fn new(repository: Arc<dyn ShowcaseRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetShowcaseUseCase for ShowcaseService {
    async fn get_showcase(&self) -> Result<Vec<ShowcaseCard>, AppError> {
        self.repository.find_most_expensive().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockShowcaseRepository;
    use crate::domain::card::CardId;
    use crate::domain::card_image::{CardImage, CardImageSource};
    use crate::domain::language_code::LanguageCode;

    fn showcase_card(set_code: &str, collector_number: &str) -> ShowcaseCard {
        ShowcaseCard {
            card_id: CardId::new(set_code, collector_number, LanguageCode::FR),
            image: CardImage {
                source: CardImageSource::GathererLocalized,
                has_back: false,
            },
        }
    }

    #[tokio::test]
    async fn get_showcase_returns_the_cards_of_the_repository() {
        let mut repository = MockShowcaseRepository::new();
        repository
            .expect_find_most_expensive()
            .times(1)
            .returning(|| {
                Box::pin(async { Ok(vec![showcase_card("LEA", "232"), showcase_card("MH2", "1")]) })
            });

        let service = ShowcaseService::new(Arc::new(repository));

        assert_eq!(
            service.get_showcase().await.unwrap(),
            vec![showcase_card("LEA", "232"), showcase_card("MH2", "1")]
        );
    }

    #[tokio::test]
    async fn get_showcase_propagates_the_repository_error() {
        let mut repository = MockShowcaseRepository::new();
        repository
            .expect_find_most_expensive()
            .times(1)
            .returning(|| {
                Box::pin(async {
                    Err(AppError::Infra(InfraError::RepositoryError(
                        "DB error".to_string(),
                    )))
                })
            });

        let service = ShowcaseService::new(Arc::new(repository));

        match service.get_showcase().await.unwrap_err() {
            AppError::Infra(InfraError::RepositoryError(msg)) => assert_eq!(msg, "DB error"),
            other => panic!("Expected RepositoryError, got {other:?}"),
        }
    }
}
