use crate::application::error::AppError;
use crate::application::imported_card::ImportedCard;
use crate::domain::card::{Card, CardId, CollectionEntry, CopyId};
use crate::domain::card_import::CardImportLineError;
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::{SetCode, SetName};
use chrono::{DateTime, Utc};
use csv::{ReaderBuilder, Trim};
use std::collections::HashMap;
use uuid::Uuid;

/// Result of parsing a ManaBox collection CSV: the cards that could be read, the errors
/// encountered on individual lines (which do not prevent the rest of the file from being
/// parsed), and the number of data lines read (before the "Tokens" set filter and
/// deduplication — used as the source-of-truth count for the import's compte-rendu).
pub struct ParsedCollection {
    pub cards: Vec<ImportedCard>,
    pub errors: Vec<CardImportLineError>,
    pub source_lines: usize,
}

pub fn parse_cards(csv: &str) -> Result<ParsedCollection, AppError> {
    let estimated_lines = csv.lines().count();

    if estimated_lines <= 1 {
        return Err(
            FunctionalError::WrongFormat("missing headers or empty file".to_string()).into(),
        );
    }

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(Trim::All)
        .from_reader(csv.as_bytes());

    let mut cards = Vec::with_capacity(estimated_lines);
    let mut errors = Vec::new();
    let mut source_lines = 0usize;

    for (index, result) in reader.records().enumerate() {
        let line_number = index + 1 + 1; // +1 car lignes humaines, +1 car header

        let record = result.map_err(|e| FunctionalError::WrongFormat(e.to_string()))?;
        let field_refs: Vec<&str> = record.iter().collect();

        if field_refs.len() == 15 {
            return Err(FunctionalError::WrongFormat(
                "expecting a collection export, got a binder export".to_string(),
            )
            .into());
        }

        if field_refs.len() != 18 {
            return Err(FunctionalError::WrongFormat(format!(
                "expected 18 fields per line, got {}",
                field_refs.len()
            ))
            .into());
        }

        source_lines += 1;

        match parse_line(&field_refs, line_number) {
            Ok(Some(card)) => cards.push(card),
            Ok(None) => {} // card from a "Tokens" set: silently ignored, not an error
            Err(e) => errors.push(e),
        }
    }

    // Keyed on the full copy identity (finish included): a normal and a foil copy of the same
    // card, in the same binder, must remain two distinct entries, not be summed into one.
    let mut seen: HashMap<(CopyId, Option<String>), ImportedCard> =
        HashMap::with_capacity(cards.len());
    let mut order: Vec<(CopyId, Option<String>)> = Vec::with_capacity(cards.len());
    for imported in cards {
        let key = (imported.card.id.clone(), imported.binder_name.clone());
        if let Some(existing) = seen.get_mut(&key) {
            let CollectionEntry::Mine {
                quantity: existing_quantity,
                purchase_price: existing_purchase_price,
                added_at: existing_added_at,
                ..
            } = existing.card.collection_entry
            else {
                unreachable!("parsed cards always carry a CollectionEntry::Mine");
            };
            let CollectionEntry::Mine {
                quantity: new_quantity,
                purchase_price: new_purchase_price,
                added_at: new_added_at,
                ..
            } = imported.card.collection_entry
            else {
                unreachable!("parsed cards always carry a CollectionEntry::Mine");
            };

            let new_qty = existing_quantity as u32 + new_quantity as u32;
            let total_cost = existing_purchase_price * existing_quantity as u32
                + new_purchase_price * new_quantity as u32;
            let added_at = existing_added_at.min(new_added_at);

            existing.card.collection_entry = CollectionEntry::Mine {
                quantity: new_qty.min(u8::MAX as u32) as u8,
                purchase_price: total_cost / new_qty,
                added_at,
                reserved: false,
            };
        } else {
            order.push(key.clone());
            seen.insert(key, imported);
        }
    }

    let cards = order
        .into_iter()
        .map(|key| seen.remove(&key).unwrap())
        .collect();

    Ok(ParsedCollection {
        cards,
        errors,
        source_lines,
    })
}

fn line_error(line: usize, field: &'static str, value: &str) -> CardImportLineError {
    CardImportLineError {
        line,
        field: field.to_string(),
        value: value.to_string(),
    }
}

/// Parses a single 18-field CSV record. `Ok(None)` means the line was valid but belongs to a
/// "Tokens" set and is silently skipped, not an error.
fn parse_line(
    field_refs: &[&str],
    line_number: usize,
) -> Result<Option<ImportedCard>, CardImportLineError> {
    let binder_name = (!field_refs[0].is_empty()).then(|| field_refs[0].to_string());

    let name = field_refs[2];
    let set_code = SetCode::try_new(field_refs[3])
        .map_err(|_| line_error(line_number, "set_code", field_refs[3]))?;
    let set_name = SetName {
        code: set_code.clone(),
        name: field_refs[4].to_string(),
    };

    if set_name.name.contains("Tokens") {
        return Ok(None);
    }

    let collector_number = field_refs[5];

    let rarity_code = RarityCode::try_new(field_refs[7])
        .map_err(|_| line_error(line_number, "rarity", field_refs[7]))?;

    let language_code: LanguageCode = LanguageCode::try_new(field_refs[15])
        .map_err(|_| line_error(line_number, "language_code", field_refs[15]))?;
    let foil: bool = field_refs[6] != "normal";

    let quantity: u8 = field_refs[8]
        .parse()
        .map_err(|_e| line_error(line_number, "quantity", field_refs[8]))?;

    let scryfall_id = Uuid::parse_str(field_refs[10])
        .map_err(|_e| line_error(line_number, "scryfall_id", field_refs[10]))?;

    let purchase_price = if field_refs[11].is_empty() {
        0
    } else {
        let purchase_price_float: f32 = field_refs[11]
            .parse()
            .map_err(|_e| line_error(line_number, "purchase_price", field_refs[11]))?;

        (purchase_price_float * 100.0).round() as u32
    };

    let added_at: DateTime<Utc> = {
        let raw = field_refs[17];
        if raw.is_empty() {
            return Err(line_error(line_number, "added_at", raw));
        }
        DateTime::parse_from_rfc3339(raw)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(|_e| line_error(line_number, "added_at", raw))?
    };

    CardId::try_new(set_code.clone(), collector_number, language_code.clone())
        .map_err(|_| line_error(line_number, "collector_number", collector_number))?;

    let card = Card::new_full(
        set_code,
        set_name.name.clone(),
        collector_number,
        language_code,
        foil,
        name,
        rarity_code,
        scryfall_id,
        None,
        None,
        CollectionEntry::Mine {
            quantity,
            purchase_price,
            added_at,
            reserved: false,
        },
    );

    Ok(Some(ImportedCard { card, binder_name }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::set_name::SetCode;

    #[test]
    fn import_cards_parses_valid_csv() -> Result<(), AppError> {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   bulk,binder,Repeal,GPT,Guildpact,32,normal,common,2,27563,9e7dd929-4bba-46a6-86c9-b8ed853eb721,0.17,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   bulk,binder,\"Dwynen, Gilt-Leaf Daen\",FDN,Foundations,217,normal,uncommon,2,100086,01c00d7b-7fac-4f8c-a1ea-de2cf4d06627,0.2,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let parsed = parse_cards(csv)?;
        let cards = parsed.cards;

        assert_eq!(cards.len(), 3);
        assert!(parsed.errors.is_empty());
        assert_eq!(parsed.source_lines, 3);

        assert_eq!(cards[0].card.id.card_id.set_code, SetCode::new("FDN"));
        assert_eq!(cards[0].card.id.card_id.collector_number, "87");
        assert_eq!(cards[0].card.id.card_id.language_code, LanguageCode::FR);
        assert!(!cards[0].card.id.foil);
        assert_eq!(cards[0].binder_name, Some("bulk".to_string()));
        let CollectionEntry::Mine {
            quantity: q0,
            purchase_price: p0,
            ..
        } = cards[0].card.collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(q0, 3);
        assert_eq!(p0, 8);

        assert_eq!(cards[1].card.id.card_id.set_code, SetCode::new("GPT"));
        assert_eq!(cards[1].card.id.card_id.collector_number, "32");
        assert_eq!(cards[1].card.id.card_id.language_code, LanguageCode::FR);
        assert!(!cards[1].card.id.foil);
        let CollectionEntry::Mine {
            quantity: q1,
            purchase_price: p1,
            ..
        } = cards[1].card.collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(q1, 2);
        assert_eq!(p1, 17);

        assert_eq!(cards[2].card.id.card_id.set_code, SetCode::new("FDN"));
        assert_eq!(cards[2].card.id.card_id.collector_number, "217");
        assert_eq!(cards[2].card.id.card_id.language_code, LanguageCode::FR);
        assert!(!cards[2].card.id.foil);
        let CollectionEntry::Mine {
            quantity: q2,
            purchase_price: p2,
            ..
        } = cards[2].card.collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(q2, 2);
        assert_eq!(p2, 20);

        Ok(())
    }

    #[test]
    fn import_cards_handles_comma_inside_quoted_field() -> Result<(), AppError> {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,\"Dwynen, Gilt-Leaf Daen\",FDN,Foundations,217,normal,uncommon,2,100086,01c00d7b-7fac-4f8c-a1ea-de2cf4d06627,0.2,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let cards = parse_cards(csv)?.cards;

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].card.name, "Dwynen, Gilt-Leaf Daen");
        assert_eq!(cards[0].card.id.card_id.set_code, SetCode::new("FDN"));
        assert_eq!(cards[0].card.id.card_id.collector_number, "217");

        Ok(())
    }

    #[test]
    fn import_cards_collects_error_for_invalid_set_code_and_keeps_no_card() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,\"Eirdu, Carrier of Dawn // Isilu, Carrier of Twilight\",EC,Lorwyn Eclipsed,13,normal,mythic,1,108961,b2d9d5ca-7e15-437a-bdfc-5972b42148fe,12.35,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let parsed = parse_cards(csv).unwrap();
        assert!(parsed.cards.is_empty());
        assert_eq!(parsed.source_lines, 1);
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].line, 2);
        assert_eq!(parsed.errors[0].field, "set_code");
    }

    #[test]
    fn import_cards_collects_error_for_invalid_language_code() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,\"Brigid, Clachan's Heart // Brigid, Doun's Mind\",ECL,Lorwyn Eclipsed,7,normal,rare,1,110841,cb7d5bbb-4f68-4e38-8bb0-a95af21b24c8,1.75,false,false,near_mint,xx,EUR,2026-02-05T20:44:45.815Z";

        let parsed = parse_cards(csv).unwrap();
        assert!(parsed.cards.is_empty());
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].line, 2);
        assert_eq!(parsed.errors[0].field, "language_code");
    }

    #[test]
    fn import_cards_collects_error_for_invalid_quantity_but_keeps_valid_rows() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Stormshriek Feral // Flush Out,TDM,Tarkir: Dragonstorm,15,normal,common,1,104447,0ec92c44-7cf0-48a5-a3ca-bc633496d887,0.11,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   bulk,binder,Stormshriek Feral // Flush Out,TDM,Tarkir: Dragonstorm,16,normal,common,NOT_VALID_NUMBER,104447,0ec92c44-7cf0-48a5-a3ca-bc633496d887,0.11,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let parsed = parse_cards(csv).unwrap();

        assert_eq!(parsed.cards.len(), 1);
        assert_eq!(parsed.source_lines, 2);
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].line, 3);
        assert_eq!(parsed.errors[0].field, "quantity");
    }

    #[test]
    fn import_cards_collects_error_for_invalid_float_format() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Stormshriek Feral // Flush Out,TDM,Tarkir: Dragonstorm,15,normal,common,1,104447,0ec92c44-7cf0-48a5-a3ca-bc633496d887,0a11,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let parsed = parse_cards(csv).unwrap();

        assert!(parsed.cards.is_empty());
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].line, 2);
        assert_eq!(parsed.errors[0].field, "purchase_price");
    }

    #[test]
    fn import_cards_collects_error_for_too_long_collector_number() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,12345678901,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let parsed = parse_cards(csv).unwrap();

        assert!(parsed.cards.is_empty());
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].line, 2);
        assert_eq!(parsed.errors[0].field, "collector_number");
    }

    #[test]
    fn import_cards_handles_empty_csv() {
        let csv = "";

        let result = parse_cards(csv);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::WrongFormat(err))) if err == "missing headers or empty file"
        ));
    }

    #[test]
    fn import_cards_returns_error_for_binder_export_with_15_columns() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition\n\
               bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint";

        let result = parse_cards(csv);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::WrongFormat(err))) if err == "expecting a collection export, got a binder export"
        ));
    }

    #[test]
    fn import_cards_returns_error_for_wrong_column_count() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID\n\
               bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506";

        let result = parse_cards(csv);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::WrongFormat(err))) if err.contains("expected 18 fields per line")
        ));
    }

    #[test]
    fn import_cards_deduplicates_by_set_code_collector_number_language_foil() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,2,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.10,false,false,near_mint,fr,EUR,2026-03-01T10:00:00.000Z";

        let cards = parse_cards(csv).unwrap().cards;

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].card.id.card_id.set_code, SetCode::new("FDN"));
        assert_eq!(cards[0].card.id.card_id.collector_number, "87");
        assert_eq!(cards[0].binder_name, Some("bulk".to_string()));
        let CollectionEntry::Mine {
            quantity,
            purchase_price,
            added_at,
            ..
        } = cards[0].card.collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(quantity, 5);
        // weighted average: (3*8 + 2*10) / 5 = (24+20)/5 = 44/5 = 8
        assert_eq!(purchase_price, 8);
        // earliest date kept
        assert_eq!(added_at.to_rfc3339(), "2026-02-05T20:44:45.815+00:00");
    }

    #[test]
    fn parse_cards_does_not_merge_normal_and_foil_copies_in_the_same_binder() {
        // Regression test for the dedup key losing the finish once `CardId` no longer carries
        // `foil`: a normal and a foil copy of the same card, in the same binder, must remain
        // two distinct entries — not be silently summed into a single entry of quantity 2.
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,foil,common,1,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.50,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let cards = parse_cards(csv).unwrap().cards;

        assert_eq!(
            cards.len(),
            2,
            "normal and foil must remain two distinct collection entries"
        );
        let normal = cards
            .iter()
            .find(|c| !c.card.id.foil)
            .expect("a non-foil entry must be present");
        let foil = cards
            .iter()
            .find(|c| c.card.id.foil)
            .expect("a foil entry must be present");

        let CollectionEntry::Mine {
            quantity: normal_qty,
            ..
        } = normal.card.collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        let CollectionEntry::Mine {
            quantity: foil_qty, ..
        } = foil.card.collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(
            normal_qty, 3,
            "quantities must not be summed across finishes"
        );
        assert_eq!(foil_qty, 1);
    }

    #[test]
    fn parse_cards_does_not_merge_rows_with_different_binder_names() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   My Deck,deck,Goblin Boarders,FDN,Foundations,87,normal,common,2,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.10,false,false,near_mint,fr,EUR,2026-03-01T10:00:00.000Z";

        let cards = parse_cards(csv).unwrap().cards;

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].binder_name, Some("bulk".to_string()));
        assert_eq!(cards[1].binder_name, Some("My Deck".to_string()));
        let CollectionEntry::Mine { quantity: q0, .. } = cards[0].card.collection_entry else {
            panic!("expected CollectionEntry::Mine");
        };
        let CollectionEntry::Mine { quantity: q1, .. } = cards[1].card.collection_entry else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(q0, 3);
        assert_eq!(q1, 2);
    }

    #[test]
    fn parse_cards_does_not_merge_named_binder_with_null_binder() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   ,binder,Goblin Boarders,FDN,Foundations,87,normal,common,2,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.10,false,false,near_mint,fr,EUR,2026-03-01T10:00:00.000Z";

        let cards = parse_cards(csv).unwrap().cards;

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].binder_name, Some("bulk".to_string()));
        assert_eq!(cards[1].binder_name, None);
    }

    #[test]
    fn parse_cards_maps_empty_binder_name_to_none() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   ,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let cards = parse_cards(csv).unwrap().cards;

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].binder_name, None);
    }

    #[test]
    fn import_cards_collects_error_for_invalid_date_format() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Repeal,GPT,Guildpact,32,normal,common,2,27563,9e7dd929-4bba-46a6-86c9-b8ed853eb721,0.17,false,false,near_mint,fr,EUR,NOT_A_DATE";

        let parsed = parse_cards(csv).unwrap();

        assert!(parsed.cards.is_empty());
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].line, 2);
        assert_eq!(parsed.errors[0].field, "added_at");
    }

    #[test]
    fn import_cards_collects_error_for_empty_added_at() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Repeal,GPT,Guildpact,32,normal,common,2,27563,9e7dd929-4bba-46a6-86c9-b8ed853eb721,0.17,false,false,near_mint,fr,EUR,";

        let parsed = parse_cards(csv).unwrap();

        assert!(parsed.cards.is_empty());
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.errors[0].line, 2);
        assert_eq!(parsed.errors[0].field, "added_at");
    }

    #[test]
    fn import_cards_ignores_cards_from_token_sets() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   bulk,binder,Goblin Token,TFDN,Foundations Tokens,1,normal,common,1,101507,4409a063-bf2a-4a49-803e-3ce6bd474354,0.01,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let parsed = parse_cards(csv).unwrap();

        assert_eq!(parsed.cards.len(), 1);
        assert_eq!(parsed.cards[0].card.name, "Goblin Boarders");
        assert!(parsed.errors.is_empty());
        assert_eq!(parsed.source_lines, 2);
    }

    #[test]
    fn import_cards_treats_empty_purchase_price_as_zero() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let cards = parse_cards(csv).unwrap().cards;

        assert_eq!(cards.len(), 1);
        let CollectionEntry::Mine { purchase_price, .. } = cards[0].card.collection_entry else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(purchase_price, 0);
    }

    #[test]
    fn import_cards_is_valid_with_alphanum_collection_number() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
               bulk,binder,\"Felothar, Dawn of the Abzan\",PTDM,Tarkir: Dragonstorm Promos,184s,foil,rare,1,105214,09478378-c28b-4334-a0a1-157325ed8e5b,0.76,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let result = parse_cards(csv);

        let card = Card::new_full(
            "PTDM",
            "Tarkir: Dragonstorm Promos",
            "184s",
            LanguageCode::FR,
            true,
            "Felothar, Dawn of the Abzan",
            RarityCode::R,
            Uuid::parse_str("09478378-c28b-4334-a0a1-157325ed8e5b").unwrap(),
            None,
            None,
            CollectionEntry::Mine {
                quantity: 1,
                purchase_price: 76,
                added_at: DateTime::parse_from_rfc3339("2026-02-05T20:44:45.815Z")
                    .unwrap()
                    .with_timezone(&Utc),
                reserved: false,
            },
        );

        let cards = result.unwrap().cards;
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].card, card);
        assert_eq!(cards[0].binder_name, Some("bulk".to_string()));
    }

    #[test]
    fn import_cards_keeps_two_entries_for_same_card_in_two_binders() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,87,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   My Deck,deck,Goblin Boarders,FDN,Foundations,87,normal,common,1,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let cards = parse_cards(csv).unwrap().cards;

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].card.id, cards[1].card.id);
        assert_ne!(cards[0].binder_name, cards[1].binder_name);
    }
}
