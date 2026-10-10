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
