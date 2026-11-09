use super::dto::ScanIndexEntryResponse;
use crate::application::repository::MockCatalogRepository;
use crate::application::service::catalog::scan_index::ScanIndexService;
use crate::application::use_case::{
    MockScanIndexUseCase, MockSearchCatalogUseCase, ScanIndexUseCase,
};
use crate::domain::catalog::{CatalogSearchItem, ScanIndex, ScanIndexEntry};
use crate::domain::error::FunctionalError;
use crate::domain::pagination::{PageRequest, Paginated};
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::{SetCode, SetName};
use crate::infrastructure::testing::{
    BEARER, assert_error, assert_routes_refuse_anonymous, authed, json_of, send,
};
use crate::infrastructure::{AppState, create_router};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde_json::json;
use std::io::{Read, Write};
use std::sync::Arc;

const JSON: &str = r#"[{"set_code":"MKM","collector_number":"12","rarity":"R","name":"Card 12"}]"#;
const ETAG: &str = "\"abc123\"";

fn scan_index() -> ScanIndex {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(JSON.as_bytes()).unwrap();
    ScanIndex {
        etag: ETAG.to_string(),
        gzipped_json: encoder.finish().unwrap(),
    }
}

fn app_serving(index: Option<ScanIndex>) -> axum::Router {
    let mut mock = MockScanIndexUseCase::new();
    mock.expect_get_scan_index().returning(move || {
        let result = match index.clone() {
            Some(index) => Ok(Arc::new(index)),
            None => Err(FunctionalError::ScanIndexUnavailable.into()),
        };
        Box::pin(async move { result })
    });
    create_router(AppState {
        scan_index_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

async fn get_with_etag(app: axum::Router, if_none_match: &str) -> Response {
    let request = Request::builder()
        .method(Method::GET)
        .uri("/catalog/scan-index")
        .header(header::AUTHORIZATION, BEARER)
        .header(header::IF_NONE_MATCH, if_none_match)
        .body(Body::empty())
        .unwrap();
    send(app, request).await
}

async fn body_bytes(response: Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec()
}

#[tokio::test]
async fn the_scan_index_is_served_as_gzipped_json_with_its_etag() {
    let response = authed(
        app_serving(Some(scan_index())),
        Method::GET,
        "/catalog/scan-index",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::ETAG], ETAG);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
    assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
    let mut json = String::new();
    GzDecoder::new(body_bytes(response).await.as_slice())
        .read_to_string(&mut json)
        .unwrap();
    assert_eq!(json, JSON);
}

/// What the documented `ScanIndexEntryResponse` says the served bytes are: the index really built
/// by `ScanIndexService` from catalog entries, decoded as the DTO (which refuses unknown fields).
#[tokio::test]
async fn the_served_body_decodes_as_the_documented_entries() {
    let mut repository = MockCatalogRepository::new();
    repository.expect_scan_index_entries().returning(|| {
        let entries = vec![
            ScanIndexEntry {
                set_code: SetCode::new("MID"),
                collector_number: "2".to_string(),
                rarity: RarityCode::U,
                name: "Ambitious Farmhand // Seasoned Cathar".to_string(),
                french_name: Some("Fermière ambitieuse // Cathare aguerrie".to_string()),
            },
            ScanIndexEntry {
                set_code: SetCode::new("MKM"),
                collector_number: "12".to_string(),
                rarity: RarityCode::M,
                name: "Delney, Streetwise Lookout".to_string(),
                french_name: None,
            },
        ];
        Box::pin(async move { Ok(entries) })
    });
    let service = Arc::new(ScanIndexService::new(Arc::new(repository)));
    service.refresh_scan_index().await.unwrap();
    let app = create_router(AppState {
        scan_index_use_case: service,
        ..AppState::for_testing()
    });

    let response = authed(app, Method::GET, "/catalog/scan-index").await;

    assert_eq!(response.status(), StatusCode::OK);
    let entries: Vec<ScanIndexEntryResponse> =
        serde_json::from_reader(GzDecoder::new(body_bytes(response).await.as_slice())).unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| (
                entry.set_code.as_str(),
                entry.collector_number.as_str(),
                entry.rarity.as_str(),
                entry.name.as_str(),
                entry.french_name.as_deref(),
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "MID",
                "2",
                "U",
                "Ambitious Farmhand // Seasoned Cathar",
                Some("Fermière ambitieuse // Cathare aguerrie")
            ),
            ("MKM", "12", "M", "Delney, Streetwise Lookout", None),
        ]
    );
}

#[tokio::test]
async fn an_equal_if_none_match_gets_a_not_modified_without_body() {
    let response = get_with_etag(app_serving(Some(scan_index())), ETAG).await;

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert_eq!(response.headers()[header::ETAG], ETAG);
    assert!(body_bytes(response).await.is_empty());
}

#[tokio::test]
async fn an_if_none_match_among_several_tags_still_matches() {
    let response = get_with_etag(app_serving(Some(scan_index())), "\"old\", \"abc123\"").await;

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn a_different_if_none_match_gets_the_whole_index() {
    let response = get_with_etag(app_serving(Some(scan_index())), "\"stale\"").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::ETAG], ETAG);
}

#[tokio::test]
async fn without_a_loaded_catalog_the_answer_is_a_503_with_its_code() {
    let response = authed(app_serving(None), Method::GET, "/catalog/scan-index").await;

    assert_error(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "scan_index_unavailable",
    )
    .await;
}

#[tokio::test]
async fn the_catalog_routes_require_a_token() {
    assert_routes_refuse_anonymous(
        || app_serving(Some(scan_index())),
        &[
            (Method::GET, "/catalog/scan-index"),
            (Method::GET, "/catalog/cards?q=bolt"),
        ],
    )
    .await;
}

// ---- GET /catalog/cards ----

fn item(set_code: &str, number: &str, french_name: Option<&str>) -> CatalogSearchItem {
    CatalogSearchItem {
        set: SetName::new(set_code, format!("Set {set_code}")),
        collector_number: number.to_string(),
        rarity: RarityCode::U,
        name: "Lightning Bolt".to_string(),
        french_name: french_name.map(str::to_string),
        image_url: Some("https://img.example/bolt.jpg".to_string()),
    }
}

fn search_app(mock: MockSearchCatalogUseCase) -> axum::Router {
    create_router(AppState {
        search_catalog_use_case: Arc::new(mock),
        ..AppState::for_testing()
    })
}

#[tokio::test]
async fn the_search_lists_the_impressions_with_their_set_and_image() {
    let mut mock = MockSearchCatalogUseCase::new();
    mock.expect_search_catalog()
        .times(1)
        .withf(|query, set_code, page| {
            query.as_deref() == Some("foudre")
                && set_code
                    .as_ref()
                    .is_some_and(|code| code.to_string() == "LEA")
                && *page
                    == PageRequest {
                        page: 1,
                        page_size: 5,
                    }
        })
        .returning(|_, _, page| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![item("LEA", "161", Some("Foudre")), item("LEA", "H3", None)],
                    total: 7,
                    pagination: page.paginate(1_000).unwrap(),
                })
            })
        });

    let response = authed(
        search_app(mock),
        Method::GET,
        "/catalog/cards?q=foudre&set_code=lea&page=1&page_size=5",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = json_of(response).await;
    assert_eq!(body["total"], 7);
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 5);
    assert_eq!(
        body["items"][0],
        json!({
            "set": { "code": "LEA", "name": "Set LEA" },
            "collector_number": "161",
            "rarity_code": "U",
            "name": "Lightning Bolt",
            "french_name": "Foudre",
            "image_url": "https://img.example/bolt.jpg",
        })
    );
    assert!(body["items"][1]["french_name"].is_null());
}

#[tokio::test]
async fn the_search_defaults_to_the_first_page_of_twenty() {
    let mut mock = MockSearchCatalogUseCase::new();
    mock.expect_search_catalog()
        .times(1)
        .withf(|query, set_code, page| {
            query.is_none() && set_code.is_none() && *page == PageRequest::default()
        })
        .returning(|_, _, page| {
            Box::pin(async move {
                Ok(Paginated {
                    items: vec![],
                    total: 0,
                    pagination: page.paginate(1_000).unwrap(),
                })
            })
        });

    let response = authed(search_app(mock), Method::GET, "/catalog/cards").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_of(response).await,
        json!({ "items": [], "total": 0, "page": 0, "page_size": 20 })
    );
}

#[tokio::test]
async fn the_search_maps_a_page_out_of_bounds_to_a_bad_request() {
    let mut mock = MockSearchCatalogUseCase::new();
    mock.expect_search_catalog().returning(|_, _, _| {
        Box::pin(async {
            Err(FunctionalError::PaginationTooDeep {
                requested_offset: 2_000,
                max: 1_000,
            }
            .into())
        })
    });

    let response = authed(
        search_app(mock),
        Method::GET,
        "/catalog/cards?q=bolt&page=100&page_size=20",
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "pagination_too_deep").await;
}

#[tokio::test]
async fn the_search_refuses_an_invalid_set_code() {
    let response = authed(
        create_router(AppState::for_testing()),
        Method::GET,
        "/catalog/cards?set_code=A",
    )
    .await;

    assert_error(response, StatusCode::BAD_REQUEST, "invalid_set_code").await;
}
