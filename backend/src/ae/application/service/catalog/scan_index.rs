use crate::application::error::{AppError, InfraError};
use crate::application::repository::CatalogRepository;
use crate::application::use_case::ScanIndexUseCase;
use crate::domain::catalog::{ScanIndex, ScanIndexEntry};
use crate::domain::error::FunctionalError;
use async_trait::async_trait;
use flate2::Compression;
use flate2::write::GzEncoder;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::sync::{Arc, RwLock};

pub struct ScanIndexService {
    catalog_repository: Arc<dyn CatalogRepository>,
    /// `None` until a catalog has been loaded.
    current: RwLock<Option<Arc<ScanIndex>>>,
}

impl ScanIndexService {
    pub fn new(catalog_repository: Arc<dyn CatalogRepository>) -> Self {
        Self {
            catalog_repository,
            current: RwLock::new(None),
        }
    }
}

#[async_trait]
impl ScanIndexUseCase for ScanIndexService {
    #[tracing::instrument(skip(self))]
    async fn refresh_scan_index(&self) -> Result<(), AppError> {
        let entries = self.catalog_repository.scan_index_entries().await?;
        // An empty catalog is no catalog: better a 503 than an index the client takes as final.
        let index = if entries.is_empty() {
            None
        } else {
            Some(Arc::new(encode(&entries)?))
        };

        tracing::info!(
            entries = entries.len(),
            etag = index.as_ref().map(|i| i.etag.as_str()),
            "Scan index generated"
        );
        *self.current.write().expect("scan index lock poisoned") = index;
        Ok(())
    }

    async fn get_scan_index(&self) -> Result<Arc<ScanIndex>, AppError> {
        self.current
            .read()
            .expect("scan index lock poisoned")
            .clone()
            .ok_or_else(|| FunctionalError::ScanIndexUnavailable.into())
    }
}

fn encode(entries: &[ScanIndexEntry]) -> Result<ScanIndex, AppError> {
    let json = serde_json::to_vec(entries).map_err(encoding_error)?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&json).map_err(encoding_error)?;
    let gzipped_json = encoder.finish().map_err(encoding_error)?;

    let hash: String = Sha256::digest(&json)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(ScanIndex {
        etag: format!("\"{hash}\""),
        gzipped_json,
    })
}

fn encoding_error(error: impl std::fmt::Display) -> AppError {
    InfraError::CallError(format!("cannot encode the scan index: {error}")).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::error::InfraError;
    use crate::application::repository::MockCatalogRepository;
    use crate::domain::rarity_code::RarityCode;
    use crate::domain::set_name::SetCode;
    use flate2::read::GzDecoder;
    use serde_json::{Value, json};
    use std::io::Read;

    fn entry(set: &str, number: &str) -> ScanIndexEntry {
        ScanIndexEntry {
            set_code: SetCode::new(set),
            collector_number: number.to_string(),
            rarity: RarityCode::R,
            name: format!("Card {number}"),
            french_name: Some(format!("Carte {number}")),
        }
    }

    fn service_over(loads: Vec<Result<Vec<ScanIndexEntry>, AppError>>) -> Arc<ScanIndexService> {
        let mut loads = loads.into_iter();
        let mut repository = MockCatalogRepository::new();
        repository.expect_scan_index_entries().returning(move || {
            let next = loads.next().expect("unexpected extra load");
            Box::pin(async move { next })
        });
        Arc::new(ScanIndexService::new(Arc::new(repository)))
    }

    fn json_of(index: &ScanIndex) -> Value {
        let mut json = String::new();
        GzDecoder::new(index.gzipped_json.as_slice())
            .read_to_string(&mut json)
            .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[tokio::test]
    async fn there_is_no_index_before_a_catalog_is_loaded() {
        let service = service_over(vec![]);

        let result = service.get_scan_index().await;

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ScanIndexUnavailable))
        ));
    }

    #[tokio::test]
    async fn an_empty_catalog_is_no_index_rather_than_a_final_empty_one() {
        let service = service_over(vec![Ok(vec![])]);

        service.refresh_scan_index().await.unwrap();

        assert!(matches!(
            service.get_scan_index().await,
            Err(AppError::Functional(FunctionalError::ScanIndexUnavailable))
        ));
    }

    #[tokio::test]
    async fn the_index_is_the_gzipped_json_of_the_entries() {
        let mut french_less = entry("MKM", "7");
        french_less.french_name = None;
        let service = service_over(vec![Ok(vec![french_less, entry("MKM", "12")])]);

        service.refresh_scan_index().await.unwrap();
        let index = service.get_scan_index().await.unwrap();

        assert_eq!(
            json_of(&index),
            json!([
                { "set_code": "MKM", "collector_number": "7", "rarity": "R", "name": "Card 7" },
                { "set_code": "MKM", "collector_number": "12", "rarity": "R", "name": "Card 12",
                  "french_name": "Carte 12" },
            ])
        );
    }

    #[tokio::test]
    async fn the_etag_is_a_quoted_hash_of_the_content() {
        let service = service_over(vec![Ok(vec![entry("MKM", "12")])]);

        service.refresh_scan_index().await.unwrap();
        let etag = service.get_scan_index().await.unwrap().etag.clone();

        // `shasum -a 256` of the uncompressed JSON, computed independently of the code under test.
        assert_eq!(
            etag,
            "\"919d66e35251eae9086aae8ca4f936ef123087abae6e173bc031f9a0fa702a9c\""
        );
    }

    #[tokio::test]
    async fn reloading_an_unchanged_catalog_keeps_the_etag() {
        let service = service_over(vec![
            Ok(vec![entry("MKM", "12")]),
            Ok(vec![entry("MKM", "12")]),
        ]);

        service.refresh_scan_index().await.unwrap();
        let first = service.get_scan_index().await.unwrap().etag.clone();
        service.refresh_scan_index().await.unwrap();
        let second = service.get_scan_index().await.unwrap().etag.clone();

        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn a_changed_catalog_changes_the_etag() {
        let mut renamed = entry("MKM", "12");
        renamed.name = "Other".to_string();
        let service = service_over(vec![Ok(vec![entry("MKM", "12")]), Ok(vec![renamed])]);

        service.refresh_scan_index().await.unwrap();
        let first = service.get_scan_index().await.unwrap().etag.clone();
        service.refresh_scan_index().await.unwrap();
        let second = service.get_scan_index().await.unwrap().etag.clone();

        assert_ne!(first, second);
    }

    #[tokio::test]
    async fn a_failing_refresh_keeps_the_previous_index() {
        let service = service_over(vec![
            Ok(vec![entry("MKM", "12")]),
            Err(AppError::Infra(InfraError::RepositoryError(
                "db down".to_string(),
            ))),
        ]);
        service.refresh_scan_index().await.unwrap();
        let before = service.get_scan_index().await.unwrap();

        let result = service.refresh_scan_index().await;

        assert!(result.is_err());
        assert_eq!(service.get_scan_index().await.unwrap(), before);
    }
}
