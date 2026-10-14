use crate::application::error::AppError;
use crate::application::repository::SetNameRepository;
use crate::application::use_case::GetSetUseCase;
use crate::domain::error::FunctionalError;
use crate::domain::set_name::{SetCode, SetName};
use async_trait::async_trait;
use std::sync::Arc;

pub struct GetSetService {
    repository: Arc<dyn SetNameRepository>,
}

impl GetSetService {
    pub fn new(repository: Arc<dyn SetNameRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl GetSetUseCase for GetSetService {
    async fn get_set(&self, code: SetCode) -> Result<SetName, AppError> {
        self.repository
            .find_by_code(code)
            .await?
            .ok_or(FunctionalError::SetNotFound.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::repository::MockSetNameRepository;
    use mockall::predicate::eq;

    #[tokio::test]
    async fn get_set_returns_set_when_found() {
        let mut mock = MockSetNameRepository::new();
        mock.expect_find_by_code()
            .with(eq(SetCode::new("ECL")))
            .times(1)
            .returning(|_| {
                Box::pin(async {
                    Ok(Some(SetName {
                        code: SetCode::new("ECL"),
                        name: "Eclipsed".to_string(),
                    }))
                })
            });

        let service = GetSetService::new(Arc::new(mock));
        let result = service.get_set(SetCode::new("ECL")).await.unwrap();

        assert_eq!(result.name, "Eclipsed");
    }

    #[tokio::test]
    async fn get_set_returns_set_not_found_when_absent() {
        let mut mock = MockSetNameRepository::new();
        mock.expect_find_by_code()
            .with(eq(SetCode::new("XXX")))
            .times(1)
            .returning(|_| Box::pin(async { Ok(None) }));

        let service = GetSetService::new(Arc::new(mock));
        let result = service.get_set(SetCode::new("XXX")).await;

        assert!(matches!(
            result.unwrap_err(),
            AppError::Functional(FunctionalError::SetNotFound)
        ));
    }
}
