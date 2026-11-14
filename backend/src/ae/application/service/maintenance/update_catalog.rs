use crate::application::caller::{ScryfallBulkCard, ScryfallCaller};
use crate::application::error::{AppError, InfraError};
use crate::application::repository::CatalogRepository;
use crate::application::use_case::{ScanIndexUseCase, UpdateCatalogUseCase};
use crate::domain::card::MAX_COLLECTOR_NUMBER_LENGTH;
use crate::domain::catalog::{CatalogCard, CatalogSet};
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::SetCode;
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

pub struct UpdateCatalogService {
    scryfall_caller: Arc<dyn ScryfallCaller>,
    catalog_repository: Arc<dyn CatalogRepository>,
    scan_index: Arc<dyn ScanIndexUseCase>,
    /// Held for the whole load: there is one at a time.
    loading: Mutex<()>,
}

impl UpdateCatalogService {
    pub fn new(
        scryfall_caller: Arc<dyn ScryfallCaller>,
        catalog_repository: Arc<dyn CatalogRepository>,
        scan_index: Arc<dyn ScanIndexUseCase>,
    ) -> Self {
        Self {
            scryfall_caller,
            catalog_repository,
            scan_index,
            loading: Mutex::new(()),
        }
    }
}

#[async_trait]
impl UpdateCatalogUseCase for UpdateCatalogService {
    #[tracing::instrument(skip(self))]
    async fn update_catalog(&self) -> Result<(), AppError> {
        let Ok(_loading) = self.loading.try_lock() else {
            tracing::warn!("catalog load ignored: another one is already running");
            return Ok(());
        };
        let started = Instant::now();

        let mut bulk = self.scryfall_caller.stream_bulk_cards().await?;
        let mut builder = CatalogBuilder::default();
        while let Some(object) = bulk.next().await {
            builder.add(object?);
        }
        let (catalog, report) = builder.build();

        if catalog.cards.is_empty() {
            return Err(
                InfraError::CallError("the Scryfall bulk holds no card".to_string()).into(),
            );
        }
        self.catalog_repository
            .replace_all(&catalog.sets, &catalog.cards)
            .await?;
        self.scan_index.refresh_scan_index().await?;

        tracing::info!(
            loaded = catalog.cards.len(),
            tokens = report.tokens,
            other_discarded = report.other_discarded,
            rejected = report.rejected,
            duration_ms = started.elapsed().as_millis() as u64,
            "Catalog loaded successfully"
        );
        Ok(())
    }
}

struct Catalog {
    sets: Vec<CatalogSet>,
    cards: Vec<CatalogCard>,
}

#[derive(Default)]
struct LoadReport {
    tokens: usize,
    /// Art series and digital-only cards.
    other_discarded: usize,
    /// Entries that do not fit the columns or have an unknown rarity.
    rejected: usize,
}

/// What the objects of one set and number say, whatever their language.
struct Entry {
    /// The English object, or else the first one met in another language.
    reference: ScryfallBulkCard,
    reference_is_english: bool,
    french_name: Option<String>,
}

#[derive(Default)]
struct CatalogBuilder {
    entries: HashMap<(String, String), Entry>,
    report: LoadReport,
}

impl CatalogBuilder {
    fn add(&mut self, object: ScryfallBulkCard) {
        if is_token(&object) {
            self.report.tokens += 1;
            return;
        }
        if object.layout == "art_series" || object.digital {
            self.report.other_discarded += 1;
            return;
        }

        let key = (
            object.set_code.to_lowercase(),
            object.collector_number.clone(),
        );
        let is_english = object.language == "en";
        let french_name = (object.language == "fr")
            .then(|| printed_name(&object))
            .flatten();
        match self.entries.get_mut(&key) {
            Some(entry) => {
                if french_name.is_some() {
                    entry.french_name = french_name;
                }
                if is_english {
                    entry.reference = object;
                    entry.reference_is_english = true;
                }
            }
            None => {
                self.entries.insert(
                    key,
                    Entry {
                        reference: object,
                        reference_is_english: is_english,
                        french_name,
                    },
                );
            }
        }
    }

    fn build(mut self) -> (Catalog, LoadReport) {
        let mut cards = Vec::with_capacity(self.entries.len());
        let mut sets: BTreeMap<String, CatalogSet> = BTreeMap::new();
        for entry in self.entries.into_values() {
            match to_catalog_card(entry) {
                Ok((card, set)) => {
                    sets.entry(set.code.to_string())
                        .and_modify(|known| {
                            known.released_at = known.released_at.min(set.released_at)
                        })
                        .or_insert(set);
                    cards.push(card);
                }
                Err(reason) => {
                    self.report.rejected += 1;
                    tracing::warn!(reason, "entry left out of the catalog");
                }
            }
        }
        cards.sort_by(|a, b| {
            (a.set_code.to_string(), &a.collector_number)
                .cmp(&(b.set_code.to_string(), &b.collector_number))
        });
        let catalog = Catalog {
            sets: sets.into_values().collect(),
            cards,
        };
        (catalog, self.report)
    }
}

/// The `t` prefix of a set code is no criterion: `tsp` and `ths` are real sets.
fn is_token(object: &ScryfallBulkCard) -> bool {
    matches!(
        object.layout.as_str(),
        "token" | "double_faced_token" | "emblem"
    ) || object.set_type == "token"
}

/// The entry as the catalog keeps it, with its set as this entry knows it; the reason when it
/// cannot be kept.
fn to_catalog_card(entry: Entry) -> Result<(CatalogCard, CatalogSet), String> {
    let Entry {
        reference,
        reference_is_english,
        french_name,
    } = entry;

    let label = format!("{}/{}", reference.set_code, reference.collector_number);
    let set_code = SetCode::try_new(&reference.set_code).map_err(|e| format!("{label}: {e:?}"))?;
    if reference.collector_number.chars().count() > MAX_COLLECTOR_NUMBER_LENGTH {
        return Err(format!("{label}: collector number too long"));
    }
    // Scryfall's `bonus` rarity (a card outside the booster slots) is a Special.
    let rarity = match reference.rarity.as_str() {
        "bonus" => RarityCode::S,
        other => {
            RarityCode::try_new(other).map_err(|_| format!("{label}: unknown rarity {other}"))?
        }
    };
    // Scryfall joins the faces' names itself in the English `name` ("Front // Back"): the printed
    // name of another language is built the same way.
    let name = if reference_is_english {
        reference.name
    } else {
        printed_name(&reference).unwrap_or(reference.name)
    };

    let set = CatalogSet {
        code: set_code.clone(),
        name: reference.set_name,
        released_at: reference.released_at,
    };
    let card = CatalogCard {
        set_code,
        collector_number: reference.collector_number,
        rarity,
        name,
        french_name,
        scryfall_id: reference.scryfall_id,
        cardmarket_id: reference.cardmarket_id,
        image_url: reference.image_url,
    };
    Ok((card, set))
}

/// The name of a non-English object as printed: its own, or on a multi-face object (Scryfall leaves
/// it null there) the faces' joined by ` // `, as in the English `name`. `None` unless every face
/// has one.
fn printed_name(object: &ScryfallBulkCard) -> Option<String> {
    if object.printed_name.is_some() {
        return object.printed_name.clone();
    }
    let names: Option<Vec<&str>> = object
        .faces
        .iter()
        .map(|face| face.printed_name.as_deref())
        .collect();
    names
        .filter(|names| !names.is_empty())
        .map(|names| names.join(" // "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::{MockScryfallCaller, ScryfallBulkCard, ScryfallBulkCardFace};
    use crate::application::repository::MockCatalogRepository;
    use crate::application::use_case::MockScanIndexUseCase;
    use crate::domain::catalog::CatalogCard;
    use crate::domain::rarity_code::RarityCode;
    use chrono::NaiveDate;
    use mockall::Sequence;
    use std::sync::Mutex;
    use uuid::Uuid;

    fn object(set: &str, number: &str, language: &str) -> ScryfallBulkCard {
        ScryfallBulkCard {
            scryfall_id: Uuid::new_v4(),
            set_code: set.to_string(),
            set_name: format!("Set {set}"),
            set_type: "expansion".to_string(),
            collector_number: number.to_string(),
            language: language.to_string(),
            layout: "normal".to_string(),
            rarity: "common".to_string(),
            released_at: NaiveDate::from_ymd_opt(2024, 2, 9).unwrap(),
            name: format!("Card {set}/{number}"),
            printed_name: None,
            faces: vec![],
            cardmarket_id: None,
            digital: false,
            image_url: None,
        }
    }

    /// Scryfall's multi-face objects: the name of the object joins the faces', and a non-English
    /// one has no `printed_name` of its own but one per face.
    fn with_faces(
        faces: &[(&str, Option<&str>)],
        mut object: ScryfallBulkCard,
    ) -> ScryfallBulkCard {
        object.layout = "transform".to_string();
        object.name = faces
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(" // ");
        object.faces = faces
            .iter()
            .map(|(name, printed_name)| ScryfallBulkCardFace {
                name: name.to_string(),
                printed_name: printed_name.map(str::to_string),
            })
            .collect();
        object
    }

    fn with_layout(layout: &str, mut object: ScryfallBulkCard) -> ScryfallBulkCard {
        object.layout = layout.to_string();
        object
    }

    fn with_rarity(rarity: &str, mut object: ScryfallBulkCard) -> ScryfallBulkCard {
        object.rarity = rarity.to_string();
        object
    }

    fn printed_as(printed_name: &str, mut object: ScryfallBulkCard) -> ScryfallBulkCard {
        object.printed_name = Some(printed_name.to_string());
        object
    }

    /// What the service handed to `CatalogRepository::replace_all`.
    #[derive(Debug, Default)]
    struct Loaded {
        sets: Vec<CatalogSet>,
        cards: Vec<CatalogCard>,
    }

    impl Loaded {
        fn card(&self, set: &str, number: &str) -> Option<&CatalogCard> {
            self.cards
                .iter()
                .find(|card| card.set_code.to_string() == set && card.collector_number == number)
        }

        fn keys(&self) -> Vec<String> {
            let mut keys: Vec<String> = self
                .cards
                .iter()
                .map(|card| format!("{}/{}", card.set_code, card.collector_number))
                .collect();
            keys.sort();
            keys
        }
    }

    fn caller_streaming(items: Vec<Result<ScryfallBulkCard, AppError>>) -> MockScryfallCaller {
        let mut caller = MockScryfallCaller::new();
        caller
            .expect_stream_bulk_cards()
            .times(1)
            .return_once(move || {
                Box::pin(async { Ok(Box::pin(futures::stream::iter(items)) as _) })
            });
        caller
    }

    /// Runs a load over `objects` and returns what was written, or `None` if nothing was replaced.
    async fn load(objects: Vec<ScryfallBulkCard>) -> Option<Loaded> {
        let captured = Arc::new(Mutex::new(None));
        let sink = captured.clone();
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_replace_all()
            .times(0..=1)
            .returning(move |sets, cards| {
                *sink.lock().unwrap() = Some(Loaded {
                    sets: sets.to_vec(),
                    cards: cards.to_vec(),
                });
                Box::pin(async { Ok(()) })
            });
        let service = UpdateCatalogService::new(
            Arc::new(caller_streaming(objects.into_iter().map(Ok).collect())),
            Arc::new(repository),
            refreshed_scan_index(),
        );

        service.update_catalog().await.unwrap();

        captured.lock().unwrap().take()
    }

    #[tokio::test]
    async fn every_kind_of_token_is_left_out_of_the_catalog() {
        let mut token_set = object("tmkm", "13", "en");
        token_set.set_type = "token".to_string();
        let loaded = load(vec![
            object("mkm", "1", "en"),
            with_layout("token", object("mkm", "2", "en")),
            with_layout("double_faced_token", object("mkm", "3", "en")),
            with_layout("emblem", object("mkm", "4", "en")),
            token_set,
        ])
        .await
        .unwrap();

        assert_eq!(loaded.keys(), vec!["MKM/1"]);
    }

    #[tokio::test]
    async fn a_real_set_starting_with_t_is_kept() {
        let loaded = load(vec![object("tsp", "12", "en"), object("ths", "7", "en")])
            .await
            .unwrap();

        assert_eq!(loaded.keys(), vec!["THS/7", "TSP/12"]);
    }

    #[tokio::test]
    async fn the_token_of_a_set_does_not_shadow_the_card_with_the_same_number() {
        let mut token = with_layout("token", object("tmkm", "12", "en"));
        token.set_type = "token".to_string();

        let loaded = load(vec![token, object("mkm", "12", "en")]).await.unwrap();

        assert_eq!(loaded.keys(), vec!["MKM/12"]);
    }

    #[tokio::test]
    async fn art_series_and_digital_only_cards_are_left_out() {
        let mut digital = object("mkm", "2", "en");
        digital.digital = true;

        let loaded = load(vec![
            object("mkm", "1", "en"),
            with_layout("art_series", object("amkm", "1", "en")),
            digital,
        ])
        .await
        .unwrap();

        assert_eq!(loaded.keys(), vec!["MKM/1"]);
    }

    #[tokio::test]
    async fn a_multi_face_card_is_a_single_catalog_entry() {
        let loaded = load(vec![
            with_layout("transform", object("mid", "2", "en")),
            with_layout("modal_dfc", object("znr", "3", "en")),
            with_layout("meld", object("emn", "4", "en")),
            with_layout("split", object("rna", "5", "en")),
            with_layout("adventure", object("eld", "6", "en")),
            with_layout("flip", object("chk", "7", "en")),
        ])
        .await
        .unwrap();

        assert_eq!(
            loaded.keys(),
            vec!["CHK/7", "ELD/6", "EMN/4", "MID/2", "RNA/5", "ZNR/3"]
        );
    }

    #[tokio::test]
    async fn the_english_object_gives_the_data_and_the_french_one_the_french_name() {
        let mut english = object("mkm", "12", "en");
        english.name = "Delney, Streetwise Lookout".to_string();
        english.rarity = "mythic".to_string();
        english.cardmarket_id = Some(751193);
        let english_id = english.scryfall_id;
        let mut french = printed_as("Delney, vigie débrouillarde", object("mkm", "12", "fr"));
        french.rarity = "rare".to_string();
        french.cardmarket_id = Some(1);

        // The French object comes first: the order of the stream does not matter.
        let loaded = load(vec![french, english]).await.unwrap();

        let card = loaded.card("MKM", "12").unwrap();
        assert_eq!(loaded.cards.len(), 1);
        assert_eq!(card.name, "Delney, Streetwise Lookout");
        assert_eq!(
            card.french_name.as_deref(),
            Some("Delney, vigie débrouillarde")
        );
        assert_eq!(card.rarity, RarityCode::M);
        assert_eq!(card.scryfall_id, english_id);
        assert_eq!(card.cardmarket_id, Some(751193));
    }

    #[tokio::test]
    async fn the_image_url_is_the_one_of_the_english_reference_object() {
        let mut english = object("mkm", "12", "en");
        english.image_url = Some("https://img.example/en.jpg".to_string());
        let mut french = object("mkm", "12", "fr");
        french.image_url = Some("https://img.example/fr.jpg".to_string());

        // The French object comes last: it does not take the image over.
        let loaded = load(vec![english, french]).await.unwrap();

        assert_eq!(
            loaded.card("MKM", "12").unwrap().image_url.as_deref(),
            Some("https://img.example/en.jpg")
        );
    }

    #[tokio::test]
    async fn an_object_without_an_image_gives_a_catalog_card_without_one() {
        let loaded = load(vec![object("mkm", "12", "en")]).await.unwrap();

        assert_eq!(loaded.card("MKM", "12").unwrap().image_url, None);
    }

    #[tokio::test]
    async fn an_entry_without_a_french_object_has_no_french_name() {
        let loaded = load(vec![
            object("mkm", "12", "en"),
            printed_as("Name in German", object("mkm", "12", "de")),
        ])
        .await
        .unwrap();

        assert_eq!(loaded.card("MKM", "12").unwrap().french_name, None);
    }

    #[tokio::test]
    async fn an_entry_known_only_outside_english_is_kept_with_its_printed_name() {
        let mut german = printed_as(
            "Delney, Strassenkundiger Ausguck",
            object("mkm", "12", "de"),
        );
        german.rarity = "rare".to_string();
        let german_id = german.scryfall_id;

        let loaded = load(vec![german]).await.unwrap();

        let card = loaded.card("MKM", "12").unwrap();
        assert_eq!(card.name, "Delney, Strassenkundiger Ausguck");
        assert_eq!(card.french_name, None);
        assert_eq!(card.rarity, RarityCode::R);
        assert_eq!(card.scryfall_id, german_id);
    }

    #[tokio::test]
    async fn a_transform_takes_its_french_name_from_its_faces_like_its_english_one() {
        let english = with_faces(
            &[("Ambitious Farmhand", None), ("Seasoned Cathar", None)],
            object("mid", "2", "en"),
        );
        let french = with_faces(
            &[
                ("Ambitious Farmhand", Some("Fermière ambitieuse")),
                ("Seasoned Cathar", Some("Cathare aguerrie")),
            ],
            object("mid", "2", "fr"),
        );

        let loaded = load(vec![english, french]).await.unwrap();

        let card = loaded.card("MID", "2").unwrap();
        assert_eq!(card.name, "Ambitious Farmhand // Seasoned Cathar");
        assert_eq!(
            card.french_name.as_deref(),
            Some("Fermière ambitieuse // Cathare aguerrie")
        );
    }

    #[tokio::test]
    async fn a_transform_known_only_in_french_is_named_from_its_faces_both_ways() {
        let french = with_faces(
            &[
                ("Ambitious Farmhand", Some("Fermière ambitieuse")),
                ("Seasoned Cathar", Some("Cathare aguerrie")),
            ],
            object("mid", "2", "fr"),
        );

        let loaded = load(vec![french]).await.unwrap();

        let card = loaded.card("MID", "2").unwrap();
        assert_eq!(card.name, "Fermière ambitieuse // Cathare aguerrie");
        assert_eq!(
            card.french_name.as_deref(),
            Some("Fermière ambitieuse // Cathare aguerrie")
        );
    }

    #[tokio::test]
    async fn a_transform_known_only_in_german_is_kept_named_from_its_faces() {
        let german = with_faces(
            &[
                ("Ambitious Farmhand", Some("Ehrgeizige Bäuerin")),
                ("Seasoned Cathar", Some("Erfahrene Katharerin")),
            ],
            object("mid", "2", "de"),
        );

        let loaded = load(vec![german]).await.unwrap();

        let card = loaded.card("MID", "2").unwrap();
        assert_eq!(card.name, "Ehrgeizige Bäuerin // Erfahrene Katharerin");
        assert_eq!(card.french_name, None);
    }

    #[tokio::test]
    async fn a_french_transform_with_a_face_unprinted_keeps_no_french_name() {
        let french = with_faces(
            &[("Front", Some("Face avant")), ("Back", None)],
            object("mid", "2", "fr"),
        );

        let loaded = load(vec![object("mid", "2", "en"), french]).await.unwrap();

        assert_eq!(loaded.card("MID", "2").unwrap().french_name, None);
    }

    #[tokio::test]
    async fn an_entry_known_only_in_french_has_its_printed_name_both_ways() {
        let loaded = load(vec![printed_as("Delney, vigie", object("mkm", "12", "fr"))])
            .await
            .unwrap();

        let card = loaded.card("MKM", "12").unwrap();
        assert_eq!(card.name, "Delney, vigie");
        assert_eq!(card.french_name.as_deref(), Some("Delney, vigie"));
    }

    #[tokio::test]
    async fn bonus_is_special_and_an_unknown_rarity_leaves_the_entry_out() {
        let loaded = load(vec![
            with_rarity("bonus", object("sld", "1", "en")),
            with_rarity("legendary", object("sld", "2", "en")),
            with_rarity("mythic", object("sld", "3", "en")),
        ])
        .await
        .unwrap();

        assert_eq!(loaded.keys(), vec!["SLD/1", "SLD/3"]);
        assert_eq!(loaded.card("SLD", "1").unwrap().rarity, RarityCode::S);
    }

    #[tokio::test]
    async fn an_entry_too_long_for_the_columns_is_left_out_without_failing_the_load() {
        let loaded = load(vec![
            object("mkm", "1", "en"),
            object("mkm", "12345678901", "en"),
            object("toolong", "2", "en"),
            object("ab", "3", "en"),
        ])
        .await
        .unwrap();

        assert_eq!(loaded.keys(), vec!["MKM/1"]);
    }

    #[tokio::test]
    async fn a_six_character_set_code_is_kept() {
        let loaded = load(vec![object("pmps06", "1", "en")]).await.unwrap();

        assert_eq!(loaded.keys(), vec!["PMPS06/1"]);
    }

    #[tokio::test]
    async fn the_sets_of_the_catalog_are_written_with_their_names() {
        let mut rejected_set = object("zzz", "1", "en");
        rejected_set.rarity = "legendary".to_string();
        let loaded = load(vec![
            object("mkm", "1", "en"),
            object("mkm", "2", "en"),
            object("tsp", "1", "en"),
            rejected_set,
        ])
        .await
        .unwrap();

        let sets: Vec<(String, String)> = loaded
            .sets
            .iter()
            .map(|set| (set.code.to_string(), set.name.clone()))
            .collect();
        assert_eq!(
            sets,
            vec![
                ("MKM".to_string(), "Set mkm".to_string()),
                ("TSP".to_string(), "Set tsp".to_string())
            ]
        );
    }

    #[tokio::test]
    async fn the_release_date_of_a_set_is_the_oldest_of_its_kept_cards() {
        let mut oldest = object("mkm", "2", "en");
        oldest.released_at = NaiveDate::from_ymd_opt(2024, 2, 9).unwrap();
        let mut newer = object("mkm", "1", "en");
        newer.released_at = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        let mut digital = object("mkm", "3", "en");
        digital.released_at = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        digital.digital = true;
        let mut other = object("tsp", "1", "en");
        other.released_at = NaiveDate::from_ymd_opt(2007, 10, 5).unwrap();

        let loaded = load(vec![newer, oldest, digital, other]).await.unwrap();

        let dates: Vec<(String, NaiveDate)> = loaded
            .sets
            .iter()
            .map(|set| (set.code.to_string(), set.released_at))
            .collect();
        assert_eq!(
            dates,
            vec![
                (
                    "MKM".to_string(),
                    NaiveDate::from_ymd_opt(2024, 2, 9).unwrap()
                ),
                (
                    "TSP".to_string(),
                    NaiveDate::from_ymd_opt(2007, 10, 5).unwrap()
                ),
            ],
            "a card left out of the catalog does not date its set"
        );
    }

    fn failure(step: &str) -> AppError {
        AppError::Infra(InfraError::CallError(format!("{step} failed")))
    }

    #[tokio::test]
    async fn an_error_in_the_middle_of_the_stream_replaces_nothing() {
        let mut repository = MockCatalogRepository::new();
        repository.expect_replace_all().times(0);
        let service = UpdateCatalogService::new(
            Arc::new(caller_streaming(vec![
                Ok(object("mkm", "1", "en")),
                Err(failure("read")),
                Ok(object("mkm", "2", "en")),
            ])),
            Arc::new(repository),
            refreshed_scan_index(),
        );

        let result = service.update_catalog().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn a_bulk_that_cannot_be_opened_replaces_nothing() {
        let mut caller = MockScryfallCaller::new();
        caller
            .expect_stream_bulk_cards()
            .times(1)
            .returning(|| Box::pin(async { Err(failure("open")) }));
        let mut repository = MockCatalogRepository::new();
        repository.expect_replace_all().times(0);
        let service = UpdateCatalogService::new(
            Arc::new(caller),
            Arc::new(repository),
            refreshed_scan_index(),
        );

        assert!(service.update_catalog().await.is_err());
    }

    #[tokio::test]
    async fn an_empty_bulk_replaces_nothing() {
        let mut repository = MockCatalogRepository::new();
        repository.expect_replace_all().times(0);
        let service = UpdateCatalogService::new(
            Arc::new(caller_streaming(vec![Ok(with_layout(
                "token",
                object("tmkm", "1", "en"),
            ))])),
            Arc::new(repository),
            refreshed_scan_index(),
        );

        assert!(service.update_catalog().await.is_err());
    }

    #[tokio::test]
    async fn a_failing_replacement_is_reported() {
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_replace_all()
            .times(1)
            .returning(|_, _| Box::pin(async { Err(failure("replace")) }));
        let mut scan_index = MockScanIndexUseCase::new();
        scan_index.expect_refresh_scan_index().times(0);
        let service = UpdateCatalogService::new(
            Arc::new(caller_streaming(vec![Ok(object("mkm", "1", "en"))])),
            Arc::new(repository),
            Arc::new(scan_index),
        );

        assert!(service.update_catalog().await.is_err());
    }

    #[tokio::test]
    async fn the_scan_index_is_regenerated_once_the_catalog_is_replaced() {
        let mut sequence = Sequence::new();
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_replace_all()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let mut scan_index = MockScanIndexUseCase::new();
        scan_index
            .expect_refresh_scan_index()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(|| Box::pin(async { Ok(()) }));
        let service = UpdateCatalogService::new(
            Arc::new(caller_streaming(vec![Ok(object("mkm", "1", "en"))])),
            Arc::new(repository),
            Arc::new(scan_index),
        );

        service.update_catalog().await.unwrap();
    }

    #[tokio::test]
    async fn a_failing_scan_index_regeneration_is_reported() {
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_replace_all()
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let mut scan_index = MockScanIndexUseCase::new();
        scan_index
            .expect_refresh_scan_index()
            .returning(|| Box::pin(async { Err(failure("index")) }));
        let service = UpdateCatalogService::new(
            Arc::new(caller_streaming(vec![Ok(object("mkm", "1", "en"))])),
            Arc::new(repository),
            Arc::new(scan_index),
        );

        assert!(service.update_catalog().await.is_err());
    }

    /// A scan index that accepts being regenerated, for the tests that do not look at it.
    fn refreshed_scan_index() -> Arc<dyn ScanIndexUseCase> {
        let mut scan_index = MockScanIndexUseCase::new();
        scan_index
            .expect_refresh_scan_index()
            .returning(|| Box::pin(async { Ok(()) }));
        Arc::new(scan_index)
    }

    #[tokio::test]
    async fn a_load_triggered_while_another_is_running_is_ignored() {
        let (release, released) = tokio::sync::oneshot::channel::<()>();
        let mut caller = MockScryfallCaller::new();
        // Only the first load reaches the caller; it is held until the second one is done.
        caller
            .expect_stream_bulk_cards()
            .times(1)
            .return_once(move || {
                Box::pin(async move {
                    let _ = released.await;
                    Ok(Box::pin(futures::stream::iter(vec![Ok(object("mkm", "1", "en"))])) as _)
                })
            });
        let mut repository = MockCatalogRepository::new();
        repository
            .expect_replace_all()
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let service = Arc::new(UpdateCatalogService::new(
            Arc::new(caller),
            Arc::new(repository),
            refreshed_scan_index(),
        ));

        let first = tokio::spawn({
            let service = service.clone();
            async move { service.update_catalog().await }
        });
        tokio::task::yield_now().await;
        let second = service.update_catalog().await;
        release.send(()).unwrap();

        assert!(second.is_ok());
        assert!(first.await.unwrap().is_ok());
    }
}
