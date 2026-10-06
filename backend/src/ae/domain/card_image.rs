//! Card images: which source they come from, the faces a source provides and the file names they
//! are stored under.

use crate::domain::card::CardId;
use crate::domain::language_code::LanguageCode;
use std::fmt::{Display, Formatter, Write};

/// Where a card's images come from, from the most to the least preferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardImageSource {
    /// Gatherer, in the card's own language.
    GathererLocalized,
    /// Gatherer, in English, for a non-English card.
    GathererEn,
    /// Scryfall `normal`, the last resort.
    Scryfall,
}

impl CardImageSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CardImageSource::GathererLocalized => "gatherer_localized",
            CardImageSource::GathererEn => "gatherer_en",
            CardImageSource::Scryfall => "scryfall",
        }
    }

    /// The source stored as [`Self::as_str`].
    pub fn from_stored(value: &str) -> Option<Self> {
        [
            CardImageSource::GathererLocalized,
            CardImageSource::GathererEn,
            CardImageSource::Scryfall,
        ]
        .into_iter()
        .find(|source| source.as_str() == value)
    }

    /// Where the image file itself comes from.
    pub fn origin(&self) -> ImageOrigin {
        match self {
            CardImageSource::GathererLocalized | CardImageSource::GathererEn => {
                ImageOrigin::Gatherer
            }
            CardImageSource::Scryfall => ImageOrigin::Scryfall,
        }
    }

    /// The source of a card in fallback (any card but the English one) using an English image
    /// from `origin`.
    pub fn of_fallback(origin: ImageOrigin) -> Self {
        match origin {
            ImageOrigin::Gatherer => CardImageSource::GathererEn,
            ImageOrigin::Scryfall => CardImageSource::Scryfall,
        }
    }

    /// The source of a card in `language` using an English image from `origin`: the English
    /// card's own Gatherer image is localized for it, a fallback for any other card.
    pub fn of_english_image(language: &LanguageCode, origin: ImageOrigin) -> Self {
        match (origin, language) {
            (ImageOrigin::Gatherer, LanguageCode::EN) => CardImageSource::GathererLocalized,
            _ => CardImageSource::of_fallback(origin),
        }
    }
}

/// Where an image file comes from. Gatherer is better than Scryfall: a stored image is only ever
/// replaced by a better one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageOrigin {
    Gatherer,
    Scryfall,
}

impl ImageOrigin {
    /// The version of the URLs of an image file: a file is only ever replaced by one
    /// from another origin, so its URL changes with it.
    pub fn url_version(&self) -> &'static str {
        match self {
            ImageOrigin::Gatherer => "gatherer",
            ImageOrigin::Scryfall => "scryfall",
        }
    }
}

/// The image stored for a card: where it comes from, and whether it has a back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardImage {
    pub source: CardImageSource,
    pub has_back: bool,
}

impl CardImage {
    /// The file `face` of the image of `card_id` is stored under: its own for an image in its
    /// language, the English card's for a card in fallback. `None` for the back of a
    /// single-faced card.
    pub fn file_name(&self, card_id: &CardId, face: CardFace) -> Option<String> {
        if face == CardFace::Back && !self.has_back {
            return None;
        }
        let image_card_id = match self.source {
            CardImageSource::GathererLocalized => card_id.clone(),
            CardImageSource::GathererEn | CardImageSource::Scryfall => english_card_id(card_id),
        };
        Some(card_image_file_name(&image_card_id, face))
    }
}

/// The English image shared by the cards of a set and number in fallback and the English card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnglishImage {
    pub origin: ImageOrigin,
    pub has_back: bool,
}

/// The id of the English card of the same set and number, whose image a card in fallback uses.
pub fn english_card_id(card_id: &CardId) -> CardId {
    CardId {
        language_code: LanguageCode::EN,
        ..card_id.clone()
    }
}

impl Display for CardImageSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A Gatherer image narrower than this is one of its thumbnails (≈ 200 px) rather than a real
/// image, and counts as missing. Set just above Scryfall `normal` (488 px), the last resort, so a
/// Gatherer image kept is never smaller than what Scryfall would give.
pub const MIN_GATHERER_IMAGE_WIDTH: u32 = 500;

/// The WebP images of every face of a card, as a source provides them. Only a physical
/// double-faced card (transform, modal, meld) has a back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardImages {
    pub front: Vec<u8>,
    pub back: Option<Vec<u8>>,
}

impl CardImages {
    /// Whether every face is a WebP image at least `min_width` pixels wide.
    pub fn all_faces_at_least(&self, min_width: u32) -> bool {
        std::iter::once(&self.front)
            .chain(&self.back)
            .all(|face| webp_width(face).is_some_and(|width| width >= min_width))
    }
}

/// Reads an image's width from its WebP header (lossy `VP8 `, lossless `VP8L` or extended
/// `VP8X`), or `None` if the bytes are not a WebP image.
pub fn webp_width(bytes: &[u8]) -> Option<u32> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WEBP" {
        return None;
    }
    match bytes.get(12..16)? {
        b"VP8 " => {
            // Frame tag (3 bytes), start code 9d 01 2a, then 14-bit width.
            if bytes.get(23..26)? != [0x9d, 0x01, 0x2a] {
                return None;
            }
            let raw = u16::from_le_bytes([*bytes.get(26)?, *bytes.get(27)?]);
            Some(u32::from(raw & 0x3fff))
        }
        b"VP8L" => {
            // Signature 0x2f, then 14-bit width minus one.
            if *bytes.get(20)? != 0x2f {
                return None;
            }
            let bits = u32::from_le_bytes([
                *bytes.get(21)?,
                *bytes.get(22)?,
                *bytes.get(23)?,
                *bytes.get(24)?,
            ]);
            Some((bits & 0x3fff) + 1)
        }
        b"VP8X" => {
            // 24-bit canvas width minus one.
            let raw = u32::from_le_bytes([*bytes.get(24)?, *bytes.get(25)?, *bytes.get(26)?, 0]);
            Some(raw + 1)
        }
        _ => None,
    }
}

/// A face of a card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardFace {
    Front,
    Back,
}

/// The file a face of the image of `card_id` is stored under: `{SET}_{number}_{LANGUAGE}.webp`,
/// suffixed `_back` for the back, where the language is the image's own. A card in fallback uses
/// the file of its [`english_card_id`]. Never derived from an external catalog.
///
/// Every character of the collector number outside `[A-Za-z0-9-]` (`★`, `_`, `/`…) is escaped
/// as `~XX`, one per UTF-8 byte, so the name stays URL- and filesystem-safe and two images never
/// share a file.
pub fn card_image_file_name(card_id: &CardId, face: CardFace) -> String {
    let mut name = format!(
        "{}_{}_{}",
        card_id.set_code,
        escape(&card_id.collector_number),
        card_id.language_code
    );
    if face == CardFace::Back {
        name.push_str("_back");
    }
    name.push_str(".webp");
    name
}

fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        if c.is_ascii_alphanumeric() || c == '-' {
            escaped.push(c);
        } else {
            let mut buffer = [0; 4];
            for byte in c.encode_utf8(&mut buffer).bytes() {
                // Writing to a String cannot fail.
                let _ = write!(escaped, "~{byte:02X}");
            }
        }
    }
    escaped
}

#[cfg(test)]
pub mod test_images {
    /// A minimal lossy WebP header (`VP8 `) declaring `width` pixels.
    pub fn webp_of_width(width: u16) -> Vec<u8> {
        let mut bytes = b"RIFF\0\0\0\0WEBPVP8 \0\0\0\0".to_vec();
        bytes.extend_from_slice(&[0, 0, 0, 0x9d, 0x01, 0x2a]);
        bytes.extend_from_slice(&width.to_le_bytes());
        bytes.extend_from_slice(&100u16.to_le_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::test_images::webp_of_width;
    use super::*;
    use crate::domain::language_code::LanguageCode;

    #[test]
    fn a_source_round_trips_through_its_stored_value() {
        for source in [
            CardImageSource::GathererLocalized,
            CardImageSource::GathererEn,
            CardImageSource::Scryfall,
        ] {
            assert_eq!(CardImageSource::from_stored(source.as_str()), Some(source));
        }
        assert_eq!(CardImageSource::from_stored("cardmarket"), None);
    }

    #[test]
    fn a_source_displays_as_its_stored_value() {
        assert_eq!(CardImageSource::GathererEn.to_string(), "gatherer_en");
    }

    #[test]
    fn a_card_in_fallback_is_never_gatherer_localized() {
        assert_eq!(
            CardImageSource::of_fallback(ImageOrigin::Gatherer),
            CardImageSource::GathererEn
        );
        assert_eq!(
            CardImageSource::of_fallback(ImageOrigin::Scryfall),
            CardImageSource::Scryfall
        );
    }

    #[test]
    fn the_english_card_owns_its_gatherer_image() {
        assert_eq!(
            CardImageSource::of_english_image(&LanguageCode::EN, ImageOrigin::Gatherer),
            CardImageSource::GathererLocalized
        );
        assert_eq!(
            CardImageSource::of_english_image(&LanguageCode::FR, ImageOrigin::Gatherer),
            CardImageSource::GathererEn
        );
        assert_eq!(
            CardImageSource::of_english_image(&LanguageCode::EN, ImageOrigin::Scryfall),
            CardImageSource::Scryfall
        );
        assert_eq!(
            CardImageSource::of_english_image(&LanguageCode::FR, ImageOrigin::Scryfall),
            CardImageSource::Scryfall
        );
    }

    #[test]
    fn a_source_keeps_the_origin_of_its_english_image() {
        for origin in [ImageOrigin::Gatherer, ImageOrigin::Scryfall] {
            for language in [LanguageCode::EN, LanguageCode::FR] {
                assert_eq!(
                    CardImageSource::of_english_image(&language, origin).origin(),
                    origin
                );
            }
        }
    }

    #[test]
    fn a_fallback_uses_the_file_of_the_english_card() {
        let card_id = CardId::new("ISD", "51", LanguageCode::FR);
        assert_eq!(
            card_image_file_name(&english_card_id(&card_id), CardFace::Back),
            "ISD_51_EN_back.webp"
        );
    }

    #[test]
    fn webp_width_reads_a_lossy_header() {
        assert_eq!(webp_width(&webp_of_width(200)), Some(200));
    }

    #[test]
    fn webp_width_reads_a_lossless_header() {
        // 744 px wide: width - 1 = 743 = 0x2e7 in the low 14 bits.
        let mut bytes = b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x2f".to_vec();
        bytes.extend_from_slice(&743u32.to_le_bytes());
        assert_eq!(webp_width(&bytes), Some(744));
    }

    #[test]
    fn webp_width_reads_an_extended_header() {
        // Header of a real Gatherer image, 744 × 1039.
        let bytes = [
            0x52, 0x49, 0x46, 0x46, 0x68, 0x84, 0x01, 0x00, 0x57, 0x45, 0x42, 0x50, 0x56, 0x50,
            0x38, 0x58, 0x0a, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0xe7, 0x02, 0x00, 0x0e,
            0x04, 0x00,
        ];
        assert_eq!(webp_width(&bytes), Some(744));
    }

    #[test]
    fn webp_width_rejects_other_formats() {
        assert_eq!(webp_width(b"\xff\xd8\xff\xe0 not a webp"), None);
        assert_eq!(webp_width(b"RIFF"), None);
        assert_eq!(webp_width(b""), None);
    }

    #[test]
    fn webp_width_rejects_a_corrupted_header() {
        let mut lossy = webp_of_width(200);
        lossy[23] = 0;
        assert_eq!(webp_width(&lossy), None);

        let mut lossless = b"RIFF\0\0\0\0WEBPVP8L\0\0\0\0\x00".to_vec();
        lossless.extend_from_slice(&743u32.to_le_bytes());
        assert_eq!(webp_width(&lossless), None);

        assert_eq!(webp_width(b"RIFF\0\0\0\0WEBPALPH"), None);
    }

    #[test]
    fn all_faces_at_least_checks_the_front_of_a_single_faced_card() {
        let images = CardImages {
            front: webp_of_width(672),
            back: None,
        };
        assert!(images.all_faces_at_least(672));
        assert!(!images.all_faces_at_least(673));
    }

    #[test]
    fn a_gatherer_image_is_kept_from_500_px_wide() {
        // PIP (Fallout) images are 646 px wide on Gatherer; ISD French ones only 200 px.
        let at = |width| CardImages {
            front: webp_of_width(width),
            back: None,
        };
        assert!(at(646).all_faces_at_least(MIN_GATHERER_IMAGE_WIDTH));
        assert!(at(500).all_faces_at_least(MIN_GATHERER_IMAGE_WIDTH));
        assert!(!at(499).all_faces_at_least(MIN_GATHERER_IMAGE_WIDTH));
    }

    #[test]
    fn all_faces_at_least_fails_on_a_single_small_face() {
        let images = CardImages {
            front: webp_of_width(744),
            back: Some(webp_of_width(200)),
        };
        assert!(!images.all_faces_at_least(MIN_GATHERER_IMAGE_WIDTH));
    }

    #[test]
    fn all_faces_at_least_fails_on_an_unreadable_face() {
        let images = CardImages {
            front: b"garbage".to_vec(),
            back: None,
        };
        assert!(!images.all_faces_at_least(1));
    }

    #[test]
    fn file_name_is_derived_from_the_card_id() {
        let card_id = CardId::new("FDN", "87", LanguageCode::FR);
        assert_eq!(
            card_image_file_name(&card_id, CardFace::Front),
            "FDN_87_FR.webp"
        );
        assert_eq!(
            card_image_file_name(&card_id, CardFace::Back),
            "FDN_87_FR_back.webp"
        );
    }

    #[test]
    fn a_card_with_its_own_image_uses_its_own_file() {
        let card_id = CardId::new("FDN", "87", LanguageCode::FR);
        let image = CardImage {
            source: CardImageSource::GathererLocalized,
            has_back: true,
        };
        assert_eq!(
            image.file_name(&card_id, CardFace::Front),
            Some("FDN_87_FR.webp".to_string())
        );
        assert_eq!(
            image.file_name(&card_id, CardFace::Back),
            Some("FDN_87_FR_back.webp".to_string())
        );
    }

    #[test]
    fn a_card_in_fallback_uses_the_file_of_the_english_image() {
        let card_id = CardId::new("FDN", "87", LanguageCode::FR);
        for source in [CardImageSource::GathererEn, CardImageSource::Scryfall] {
            let image = CardImage {
                source,
                has_back: false,
            };
            assert_eq!(
                image.file_name(&card_id, CardFace::Front),
                Some("FDN_87_EN.webp".to_string())
            );
        }
    }

    #[test]
    fn a_single_faced_card_has_no_back_file() {
        let image = CardImage {
            source: CardImageSource::Scryfall,
            has_back: false,
        };
        assert_eq!(
            image.file_name(&CardId::new("FDN", "87", LanguageCode::EN), CardFace::Back),
            None
        );
    }

    #[test]
    fn an_origin_versions_the_urls_of_its_file() {
        assert_eq!(ImageOrigin::Gatherer.url_version(), "gatherer");
        assert_eq!(ImageOrigin::Scryfall.url_version(), "scryfall");
    }

    #[test]
    fn file_name_escapes_unsafe_collector_number_characters() {
        let card_id = CardId::new("PLST", "SLD-1★_/", LanguageCode::EN);
        assert_eq!(
            card_image_file_name(&card_id, CardFace::Front),
            "PLST_SLD-1~E2~98~85~5F~2F_EN.webp"
        );
    }
}
