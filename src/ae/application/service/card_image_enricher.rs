use crate::application::caller::{GathererCaller, GathererLookup, GathererMiss, ScryfallCaller};
use crate::application::error::AppError;
use crate::application::repository::{CardImageLookup, CardImageRepository, CardRepository};
use crate::application::service::enrichment_queue::{Enricher, EnrichmentQueue};
use crate::application::use_case::EnqueueCardImageUpdateUseCase;
use crate::domain::card::CardId;
use crate::domain::card_image::{
    CardImageSource, CardImages, EnglishImage, ImageOrigin, MIN_GATHERER_IMAGE_WIDTH,
    english_card_id, webp_width,
};
use crate::domain::language_code::LanguageCode;
use async_trait::async_trait;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

/// Downloads a card's images from the best source available and stores them (ADR 0017).
///
/// Sources are tried in order — Gatherer in the card's language, Gatherer in English, Scryfall —
/// and the first one providing every face is kept. Only a "not found" moves on to the next
/// source; a technical error leaves the card pending.
///
/// Without its own image, a card falls back on the English image of the same card, stored once
/// under the English card's name and shared with it: an existing English image is reused, and
/// only ever replaced by a better one.
pub struct CardImageEnricher {
    card_repository: Arc<dyn CardRepository>,
    card_image_repository: Arc<dyn CardImageRepository>,
    gatherer_caller: Arc<dyn GathererCaller>,
    scryfall_caller: Arc<dyn ScryfallCaller>,
}

impl CardImageEnricher {
    pub fn new(
        card_repository: Arc<dyn CardRepository>,
        card_image_repository: Arc<dyn CardImageRepository>,
        gatherer_caller: Arc<dyn GathererCaller>,
        scryfall_caller: Arc<dyn ScryfallCaller>,
    ) -> Self {
        Self {
            card_repository,
            card_image_repository,
            gatherer_caller,
            scryfall_caller,
        }
    }

    /// Cards in fallback, and pending cards.
    async fn in_fallback_or_pending(&self) -> Result<Vec<(CardId, CardImageLookup)>, AppError> {
        self.card_repository.get_all_in_fallback_or_pending().await
    }

    /// The Gatherer images of the card in `language` if Gatherer has them all wide enough,
    /// otherwise why they are skipped.
    async fn gatherer_images(
        &self,
        card_id: &CardId,
        name: &str,
        language: LanguageCode,
    ) -> Result<Result<CardImages, Skipped>, AppError> {
        let card = match self
            .gatherer_caller
            .get_card(
                card_id.set_code.clone(),
                card_id.collector_number.clone(),
                language.clone(),
                name.to_string(),
            )
            .await?
        {
            GathererLookup::Found(card) => card,
            GathererLookup::NotFound(miss) => return Ok(Err(Skipped::Gatherer(language, miss))),
        };

        // Clients still build Gatherer URLs from this id, resolved in the card's language only.
        if language == card_id.language_code {
            self.card_repository
                .update_gatherer_id(card_id.clone(), Some(card.gatherer_id))
                .await?;
        }

        if card.images.all_faces_at_least(MIN_GATHERER_IMAGE_WIDTH) {
            Ok(Ok(card.images))
        } else {
            Ok(Err(Skipped::TooSmall(
                language,
                FaceWidths::of(&card.images),
            )))
        }
    }

    /// Fills the English card's Gatherer id when it reuses an image another card downloaded, by
    /// reading its page only.
    async fn fill_english_gatherer_id(&self, card_id: &CardId, name: &str) -> Result<(), AppError> {
        if let Some(gatherer_id) = self
            .gatherer_caller
            .get_gatherer_id(
                card_id.set_code.clone(),
                card_id.collector_number.clone(),
                LanguageCode::EN,
                name.to_string(),
            )
            .await?
        {
            self.card_repository
                .update_gatherer_id(card_id.clone(), Some(gatherer_id))
                .await?;
        }
        Ok(())
    }

    /// Stores the card's own image, in its language.
    async fn store_own(&self, card_id: &CardId, images: CardImages) -> Result<(), AppError> {
        self.card_image_repository.save(card_id, &images).await?;
        self.card_repository
            .update_image_source(
                card_id.clone(),
                CardImageSource::GathererLocalized,
                images.back.is_some(),
            )
            .await?;
        tracing::info!("{} image ✓ {}", card_id, CardImageSource::GathererLocalized);
        Ok(())
    }

    /// Stores the English image, shared with the cards already using it, which follow it. The
    /// caller only stores an image better than the existing one.
    async fn store_english(
        &self,
        card_id: &CardId,
        origin: ImageOrigin,
        images: CardImages,
        skipped: &SkippedSteps,
    ) -> Result<(), AppError> {
        self.card_image_repository
            .save(&english_card_id(card_id), &images)
            .await?;
        let image = EnglishImage {
            origin,
            has_back: images.back.is_some(),
        };
        self.use_english(card_id, image, skipped).await
    }

    /// Records that the card uses the English image; `skipped` says why every better image was
    /// not kept, for the logs.
    async fn use_english(
        &self,
        card_id: &CardId,
        image: EnglishImage,
        skipped: &SkippedSteps,
    ) -> Result<(), AppError> {
        self.card_repository
            .record_english_image(card_id.clone(), image)
            .await?;
        tracing::info!(
            "{} image ✓ {} ({})",
            card_id,
            CardImageSource::of_english_image(&card_id.language_code, image.origin),
            skipped
        );
        Ok(())
    }
}

/// Why a better image was not kept, logged with the image finally used.
#[derive(Debug, PartialEq, Eq)]
enum Skipped {
    Gatherer(LanguageCode, GathererMiss),
    TooSmall(LanguageCode, FaceWidths),
    /// The English image already stored was reused as it is.
    Reused(ImageOrigin),
    ScryfallNotFound,
}

impl Display for Skipped {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Skipped::Gatherer(language, miss) => write!(f, "Gatherer {language}: {miss}"),
            Skipped::TooSmall(language, widths) => write!(
                f,
                "Gatherer {language}: image too small ({widths} < {MIN_GATHERER_IMAGE_WIDTH} px)"
            ),
            Skipped::Reused(ImageOrigin::Gatherer) => {
                f.write_str("English image from Gatherer reused")
            }
            Skipped::Reused(ImageOrigin::Scryfall) => {
                f.write_str("English image from Scryfall reused")
            }
            Skipped::ScryfallNotFound => f.write_str("Scryfall: not found"),
        }
    }
}

/// Every step skipped for a card, in order, e.g. `Gatherer FR: page not found; Gatherer EN: …`.
#[derive(Debug, Default)]
struct SkippedSteps(Vec<Skipped>);

impl SkippedSteps {
    fn push(&mut self, skipped: Skipped) {
        self.0.push(skipped);
    }
}

impl Display for SkippedSteps {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, skipped) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{skipped}")?;
        }
        Ok(())
    }
}

/// The width of every face of a card's images, `None` when unreadable.
#[derive(Debug, PartialEq, Eq)]
struct FaceWidths {
    front: Option<u32>,
    back: Option<Option<u32>>,
}

impl FaceWidths {
    fn of(images: &CardImages) -> Self {
        Self {
            front: webp_width(&images.front),
            back: images.back.as_deref().map(webp_width),
        }
    }
}

/// E.g. `front 646 px, back 223 px`.
impl Display for FaceWidths {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let face = |f: &mut Formatter<'_>, name: &str, width: Option<u32>| match width {
            Some(width) => write!(f, "{name} {width} px"),
            None => write!(f, "{name} unreadable"),
        };
        face(f, "front", self.front)?;
        if let Some(back) = self.back {
            f.write_str(", ")?;
            face(f, "back", back)?;
        }
        Ok(())
    }
}

#[async_trait]
impl Enricher for CardImageEnricher {
    type Lookup = CardImageLookup;
    const NAME: &'static str = "Card image";

    async fn pending(&self) -> Result<Vec<(CardId, CardImageLookup)>, AppError> {
        self.card_repository.get_all_without_image_source().await
    }

    async fn resolve(&self, card_id: &CardId, lookup: CardImageLookup) -> Result<(), AppError> {
        let mut skipped = SkippedSteps::default();

        // 1. The card's own image, in its language. An English card's own image is the English
        //    image, handled below with the cards in fallback.
        if card_id.language_code != LanguageCode::EN {
            match self
                .gatherer_images(card_id, &lookup.name, card_id.language_code.clone())
                .await?
            {
                Ok(images) => return self.store_own(card_id, images).await,
                Err(reason) => skipped.push(reason),
            }
        }

        // 2. The English image, shared: reused when it already comes from Gatherer, replaced only
        //    by a better one, and never downloaded from Scryfall twice.
        let existing = self.card_repository.find_english_image(card_id).await?;
        if let Some(image) = existing
            && image.origin == ImageOrigin::Gatherer
        {
            // Clients still read the_gatherer_id (until #425), which the English card only gets
            // from its own Gatherer page.
            if card_id.language_code == LanguageCode::EN {
                self.fill_english_gatherer_id(card_id, &lookup.name).await?;
            }
            skipped.push(Skipped::Reused(ImageOrigin::Gatherer));
            return self.use_english(card_id, image, &skipped).await;
        }
        match self
            .gatherer_images(card_id, &lookup.name, LanguageCode::EN)
            .await?
        {
            Ok(images) => {
                return self
                    .store_english(card_id, ImageOrigin::Gatherer, images, &skipped)
                    .await;
            }
            Err(reason) => skipped.push(reason),
        }
        if let Some(image) = existing {
            skipped.push(Skipped::Reused(image.origin));
            return self.use_english(card_id, image, &skipped).await;
        }

        match self
            .scryfall_caller
            .get_card_images(lookup.scryfall_id)
            .await?
        {
            Some(images) => {
                self.store_english(card_id, ImageOrigin::Scryfall, images, &skipped)
                    .await
            }
            None => {
                skipped.push(Skipped::ScryfallNotFound);
                tracing::warn!(
                    "No image found for card {}, left pending ({})",
                    card_id,
                    skipped
                );
                Ok(())
            }
        }
    }
}

#[async_trait]
impl EnqueueCardImageUpdateUseCase for EnrichmentQueue<CardImageEnricher> {
    async fn enqueue_pending_updates(&self) -> Result<usize, AppError> {
        self.enqueue_pending().await
    }

    async fn enqueue_fallback_and_pending_updates(&self) -> Result<usize, AppError> {
        let cards = self.enricher().in_fallback_or_pending().await?;
        self.enqueue(cards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::caller::{
        GathererCard, GathererMiss, MockGathererCaller, MockScryfallCaller,
    };
    use crate::application::error::InfraError;
    use crate::application::repository::{
        MockCardImageRepository, MockCardPricesViewRepository, MockCardRepository,
    };
    use crate::application::service::enrichment_queue::EnrichmentWorker;
    use crate::domain::card_image::test_images::webp_of_width;
    use uuid::Uuid;

    const BIG: u16 = 744;
    const SMALL: u16 = 200;

    fn make_card_id(language: LanguageCode) -> CardId {
        CardId::new("FDN", "87", language)
    }

    fn lookup() -> CardImageLookup {
        CardImageLookup {
            name: "Goblin Boarders".to_string(),
            scryfall_id: Uuid::nil(),
        }
    }

    fn images(front: u16, back: Option<u16>) -> CardImages {
        CardImages {
            front: webp_of_width(front),
            back: back.map(webp_of_width),
        }
    }

    fn gatherer_card(front: u16, back: Option<u16>) -> GathererCard {
        GathererCard {
            gatherer_id: "GID".to_string(),
            images: images(front, back),
        }
    }

    fn english_image(origin: ImageOrigin, has_back: bool) -> EnglishImage {
        EnglishImage { origin, has_back }
    }

    fn technical_error() -> AppError {
        InfraError::CallError("503 Service Unavailable".to_string()).into()
    }

    /// What Gatherer answers in one language: `Ok(None)` is "not found", `Err` a technical error.
    type Answer = Result<Option<GathererCard>, ()>;

    /// Gatherer answers per language; a language mapped to `None` must never be asked.
    fn gatherer(fr: Option<Answer>, en: Option<Answer>) -> MockGathererCaller {
        let mut caller = MockGathererCaller::new();
        for (language, answer) in [(LanguageCode::FR, fr), (LanguageCode::EN, en)] {
            let expectation = caller
                .expect_get_card()
                .withf(move |_, _, asked, _| *asked == language);
            match answer {
                Some(answer) => {
                    expectation.returning(move |_, _, _, _| {
                        let answer = answer
                            .clone()
                            .map(|card| match card {
                                Some(card) => GathererLookup::Found(card),
                                None => GathererLookup::NotFound(GathererMiss::NoPage),
                            })
                            .map_err(|_| technical_error());
                        Box::pin(async move { answer })
                    });
                }
                None => {
                    expectation.times(0);
                }
            }
        }
        caller
    }

    fn scryfall(answer: Result<Option<CardImages>, ()>) -> MockScryfallCaller {
        let mut caller = MockScryfallCaller::new();
        caller.expect_get_card_images().returning(move |_| {
            let answer = answer.clone().map_err(|_| technical_error());
            Box::pin(async move { answer })
        });
        caller
    }

    fn no_scryfall_call() -> MockScryfallCaller {
        let mut caller = MockScryfallCaller::new();
        caller.expect_get_card_images().times(0);
        caller
    }

    /// What the card ends up recorded with.
    enum Recorded {
        Nothing,
        /// Its own image.
        Own {
            has_back: bool,
        },
        English(EnglishImage),
    }

    /// Accepts any gatherer id update, answers `existing` as the English image already stored,
    /// and expects exactly `recorded`.
    fn card_repository(existing: Option<EnglishImage>, recorded: Recorded) -> MockCardRepository {
        let mut repository = MockCardRepository::new();
        repository
            .expect_update_gatherer_id()
            .returning(|_, _| Box::pin(async { Ok(()) }));
        repository
            .expect_find_english_image()
            .returning(move |_| Box::pin(async move { Ok(existing) }));
        let (own_updates, english_updates) = match recorded {
            Recorded::Nothing => (0, 0),
            Recorded::Own { has_back } => {
                repository
                    .expect_update_image_source()
                    .withf(move |_, source, back| {
                        *source == CardImageSource::GathererLocalized && *back == has_back
                    })
                    .times(1)
                    .returning(|_, _, _| Box::pin(async { Ok(()) }));
                (1, 0)
            }
            Recorded::English(expected) => {
                repository
                    .expect_record_english_image()
                    .withf(move |_, image| *image == expected)
                    .times(1)
                    .returning(|_, _| Box::pin(async { Ok(()) }));
                (0, 1)
            }
        };
        if own_updates == 0 {
            repository.expect_update_image_source().times(0);
        }
        if english_updates == 0 {
            repository.expect_record_english_image().times(0);
        }
        repository
    }

    /// Expects `saved` to be written under the name of the card `file_id`, or nothing written.
    fn image_repository(saved: Option<(CardId, CardImages)>) -> MockCardImageRepository {
        let mut repository = MockCardImageRepository::new();
        match saved {
            Some((file_id, expected)) => {
                repository
                    .expect_save()
                    .withf(move |id, images| *id == file_id && *images == expected)
                    .times(1)
                    .returning(|_, _| Box::pin(async { Ok(()) }));
            }
            None => {
                repository.expect_save().times(0);
            }
        }
        repository
    }

    fn enricher(
        card_repository: MockCardRepository,
        image_repository: MockCardImageRepository,
        gatherer: MockGathererCaller,
        scryfall: MockScryfallCaller,
    ) -> CardImageEnricher {
        CardImageEnricher::new(
            Arc::new(card_repository),
            Arc::new(image_repository),
            Arc::new(gatherer),
            Arc::new(scryfall),
        )
    }

    async fn resolve(enricher: CardImageEnricher, language: LanguageCode) -> Result<(), AppError> {
        enricher.resolve(&make_card_id(language), lookup()).await
    }

    #[test]
    fn face_widths_list_every_face() {
        assert_eq!(
            FaceWidths::of(&images(646, None)).to_string(),
            "front 646 px"
        );
        assert_eq!(
            FaceWidths::of(&images(744, Some(223))).to_string(),
            "front 744 px, back 223 px"
        );
        let unreadable = CardImages {
            front: b"garbage".to_vec(),
            back: None,
        };
        assert_eq!(FaceWidths::of(&unreadable).to_string(), "front unreadable");
    }

    #[test]
    fn skipped_steps_read_as_a_log_line() {
        let mut skipped = SkippedSteps::default();
        skipped.push(Skipped::Gatherer(LanguageCode::FR, GathererMiss::NoPage));
        skipped.push(Skipped::TooSmall(
            LanguageCode::EN,
            FaceWidths::of(&images(200, None)),
        ));
        skipped.push(Skipped::ScryfallNotFound);

        assert_eq!(
            skipped.to_string(),
            "Gatherer FR: page not found; Gatherer EN: image too small (front 200 px < 500 px); \
             Scryfall: not found"
        );
    }

    #[test]
    fn a_reused_image_reads_with_its_origin() {
        assert_eq!(
            Skipped::Reused(ImageOrigin::Gatherer).to_string(),
            "English image from Gatherer reused"
        );
        assert_eq!(
            Skipped::Reused(ImageOrigin::Scryfall).to_string(),
            "English image from Scryfall reused"
        );
    }

    /// A queue over `card_repository` and its worker, never run but kept alive by the caller for
    /// cards to be queued.
    fn queue(
        card_repository: MockCardRepository,
    ) -> (
        EnrichmentQueue<CardImageEnricher>,
        EnrichmentWorker<CardImageEnricher>,
    ) {
        let enricher = enricher(
            card_repository,
            MockCardImageRepository::new(),
            MockGathererCaller::new(),
            MockScryfallCaller::new(),
        );
        EnrichmentQueue::new(enricher, Arc::new(MockCardPricesViewRepository::new()))
    }

    #[tokio::test]
    async fn pending_updates_queue_only_the_cards_without_image_source() {
        let mut repository = MockCardRepository::new();
        repository
            .expect_get_all_without_image_source()
            .returning(|| Box::pin(async { Ok(vec![(make_card_id(LanguageCode::FR), lookup())]) }));
        repository.expect_get_all_in_fallback_or_pending().times(0);
        let (queue, _worker) = queue(repository);

        assert_eq!(queue.enqueue_pending_updates().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn fallback_and_pending_updates_also_queue_the_cards_in_fallback() {
        let mut repository = MockCardRepository::new();
        repository
            .expect_get_all_in_fallback_or_pending()
            .returning(|| {
                Box::pin(async {
                    Ok(vec![
                        (make_card_id(LanguageCode::FR), lookup()),
                        (make_card_id(LanguageCode::DE), lookup()),
                    ])
                })
            });
        repository.expect_get_all_without_image_source().times(0);
        let (queue, _worker) = queue(repository);

        let enqueued = queue.enqueue_fallback_and_pending_updates().await.unwrap();

        assert_eq!(enqueued, 2);
    }

    #[tokio::test]
    async fn pending_lists_cards_without_image_source() {
        let mut repository = MockCardRepository::new();
        repository
            .expect_get_all_without_image_source()
            .returning(|| Box::pin(async { Ok(vec![(make_card_id(LanguageCode::FR), lookup())]) }));
        let enricher = enricher(
            repository,
            MockCardImageRepository::new(),
            MockGathererCaller::new(),
            MockScryfallCaller::new(),
        );

        let pending = enricher.pending().await.unwrap();

        assert_eq!(pending, vec![(make_card_id(LanguageCode::FR), lookup())]);
    }

    #[tokio::test]
    async fn gatherer_in_the_card_language_is_its_own_image() {
        let enricher = enricher(
            card_repository(None, Recorded::Own { has_back: false }),
            image_repository(Some((make_card_id(LanguageCode::FR), images(BIG, None)))),
            gatherer(Some(Ok(Some(gatherer_card(BIG, None)))), None),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn the_localized_gatherer_id_is_stored_even_when_its_image_is_too_small() {
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_update_gatherer_id()
            .withf(|id, gid| *id == make_card_id(LanguageCode::FR) && gid.as_deref() == Some("GID"))
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        card_repository
            .expect_find_english_image()
            .returning(|_| Box::pin(async { Ok(None) }));
        card_repository
            .expect_record_english_image()
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let enricher = enricher(
            card_repository,
            image_repository(Some((make_card_id(LanguageCode::EN), images(BIG, None)))),
            gatherer(
                Some(Ok(Some(gatherer_card(SMALL, None)))),
                Some(Ok(Some(gatherer_card(BIG, None)))),
            ),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn without_its_own_image_a_card_stores_the_english_one_under_the_english_name() {
        let enricher = enricher(
            card_repository(
                None,
                Recorded::English(english_image(ImageOrigin::Gatherer, false)),
            ),
            image_repository(Some((make_card_id(LanguageCode::EN), images(BIG, None)))),
            gatherer(Some(Ok(None)), Some(Ok(Some(gatherer_card(BIG, None))))),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn a_too_small_localized_image_falls_back_on_the_english_one() {
        let enricher = enricher(
            card_repository(
                None,
                Recorded::English(english_image(ImageOrigin::Gatherer, false)),
            ),
            image_repository(Some((make_card_id(LanguageCode::EN), images(BIG, None)))),
            gatherer(
                Some(Ok(Some(gatherer_card(SMALL, None)))),
                Some(Ok(Some(gatherer_card(BIG, None)))),
            ),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn an_english_image_from_gatherer_is_reused_without_any_download() {
        let existing = english_image(ImageOrigin::Gatherer, true);
        let enricher = enricher(
            card_repository(Some(existing), Recorded::English(existing)),
            image_repository(None),
            gatherer(Some(Ok(None)), None),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn an_english_card_reuses_the_english_image_a_card_in_fallback_stored() {
        let existing = english_image(ImageOrigin::Gatherer, false);
        // No image download (get_card): only the page, for the card's Gatherer id.
        let mut gatherer_caller = gatherer(None, None);
        gatherer_caller
            .expect_get_gatherer_id()
            .withf(|_, _, language, _| *language == LanguageCode::EN)
            .times(1)
            .returning(|_, _, _, _| Box::pin(async { Ok(Some("EN-GID".to_string())) }));
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_find_english_image()
            .returning(move |_| Box::pin(async move { Ok(Some(existing)) }));
        card_repository
            .expect_update_gatherer_id()
            .withf(|id, gid| {
                *id == make_card_id(LanguageCode::EN) && gid.as_deref() == Some("EN-GID")
            })
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        card_repository
            .expect_record_english_image()
            .withf(move |_, image| *image == existing)
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let enricher = enricher(
            card_repository,
            image_repository(None),
            gatherer_caller,
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::EN).await.unwrap();
    }

    #[tokio::test]
    async fn an_english_card_without_a_gatherer_page_reuses_the_image_without_an_id() {
        let existing = english_image(ImageOrigin::Gatherer, false);
        let mut gatherer_caller = gatherer(None, None);
        gatherer_caller
            .expect_get_gatherer_id()
            .returning(|_, _, _, _| Box::pin(async { Ok(None) }));
        let mut card_repository = MockCardRepository::new();
        card_repository
            .expect_find_english_image()
            .returning(move |_| Box::pin(async move { Ok(Some(existing)) }));
        card_repository.expect_update_gatherer_id().times(0);
        card_repository
            .expect_record_english_image()
            .withf(move |_, image| *image == existing)
            .times(1)
            .returning(|_, _| Box::pin(async { Ok(()) }));
        let enricher = enricher(
            card_repository,
            image_repository(None),
            gatherer_caller,
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::EN).await.unwrap();
    }

    #[tokio::test]
    async fn an_english_image_from_scryfall_is_replaced_by_a_gatherer_one() {
        let enricher = enricher(
            card_repository(
                Some(english_image(ImageOrigin::Scryfall, false)),
                Recorded::English(english_image(ImageOrigin::Gatherer, false)),
            ),
            image_repository(Some((make_card_id(LanguageCode::EN), images(BIG, None)))),
            gatherer(Some(Ok(None)), Some(Ok(Some(gatherer_card(BIG, None))))),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn an_english_image_from_scryfall_is_reused_rather_than_downloaded_again() {
        let existing = english_image(ImageOrigin::Scryfall, false);
        let enricher = enricher(
            card_repository(Some(existing), Recorded::English(existing)),
            image_repository(None),
            gatherer(Some(Ok(None)), Some(Ok(None))),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn scryfall_is_stored_as_the_english_image_when_gatherer_has_nothing() {
        let enricher = enricher(
            card_repository(
                None,
                Recorded::English(english_image(ImageOrigin::Scryfall, false)),
            ),
            image_repository(Some((make_card_id(LanguageCode::EN), images(SMALL, None)))),
            gatherer(Some(Ok(None)), Some(Ok(Some(gatherer_card(SMALL, None))))),
            scryfall(Ok(Some(images(SMALL, None)))),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn an_english_card_asks_gatherer_once() {
        let mut gatherer_caller = MockGathererCaller::new();
        gatherer_caller
            .expect_get_card()
            .withf(|_, _, language, _| *language == LanguageCode::EN)
            .times(1)
            .returning(|_, _, _, _| {
                Box::pin(async { Ok(GathererLookup::NotFound(GathererMiss::NoPage)) })
            });
        let enricher = enricher(
            card_repository(
                None,
                Recorded::English(english_image(ImageOrigin::Scryfall, false)),
            ),
            image_repository(Some((make_card_id(LanguageCode::EN), images(BIG, None)))),
            gatherer_caller,
            scryfall(Ok(Some(images(BIG, None)))),
        );

        resolve(enricher, LanguageCode::EN).await.unwrap();
    }

    #[tokio::test]
    async fn a_gatherer_technical_error_leaves_the_card_pending_without_trying_scryfall() {
        let enricher = enricher(
            card_repository(None, Recorded::Nothing),
            image_repository(None),
            gatherer(Some(Err(())), None),
            no_scryfall_call(),
        );

        assert!(resolve(enricher, LanguageCode::FR).await.is_err());
    }

    #[tokio::test]
    async fn an_english_gatherer_technical_error_leaves_the_card_pending() {
        let enricher = enricher(
            card_repository(
                Some(english_image(ImageOrigin::Scryfall, false)),
                Recorded::Nothing,
            ),
            image_repository(None),
            gatherer(Some(Ok(None)), Some(Err(()))),
            no_scryfall_call(),
        );

        assert!(resolve(enricher, LanguageCode::FR).await.is_err());
    }

    #[tokio::test]
    async fn a_scryfall_technical_error_leaves_the_card_pending() {
        let enricher = enricher(
            card_repository(None, Recorded::Nothing),
            image_repository(None),
            gatherer(Some(Ok(None)), Some(Ok(None))),
            scryfall(Err(())),
        );

        assert!(resolve(enricher, LanguageCode::FR).await.is_err());
    }

    #[tokio::test]
    async fn a_card_without_any_image_stays_pending_without_error() {
        let enricher = enricher(
            card_repository(None, Recorded::Nothing),
            image_repository(None),
            gatherer(Some(Ok(None)), Some(Ok(None))),
            scryfall(Ok(None)),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn a_double_faced_card_stores_both_faces_from_the_same_source() {
        let enricher = enricher(
            card_repository(None, Recorded::Own { has_back: true }),
            image_repository(Some((
                make_card_id(LanguageCode::FR),
                images(BIG, Some(BIG)),
            ))),
            gatherer(Some(Ok(Some(gatherer_card(BIG, Some(BIG))))), None),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn a_too_small_back_moves_both_faces_to_the_next_source() {
        let enricher = enricher(
            card_repository(
                None,
                Recorded::English(english_image(ImageOrigin::Gatherer, true)),
            ),
            image_repository(Some((
                make_card_id(LanguageCode::EN),
                images(BIG, Some(BIG)),
            ))),
            gatherer(
                Some(Ok(Some(gatherer_card(BIG, Some(SMALL))))),
                Some(Ok(Some(gatherer_card(BIG, Some(BIG))))),
            ),
            no_scryfall_call(),
        );

        resolve(enricher, LanguageCode::FR).await.unwrap();
    }

    #[tokio::test]
    async fn a_failed_file_write_does_not_record_the_source() {
        let mut image_repository = MockCardImageRepository::new();
        image_repository.expect_save().returning(|_, _| {
            Box::pin(async { Err(InfraError::RepositoryError("disk full".into()).into()) })
        });
        let enricher = enricher(
            card_repository(None, Recorded::Nothing),
            image_repository,
            gatherer(Some(Ok(Some(gatherer_card(BIG, None)))), None),
            no_scryfall_call(),
        );

        assert!(resolve(enricher, LanguageCode::FR).await.is_err());
    }
}
