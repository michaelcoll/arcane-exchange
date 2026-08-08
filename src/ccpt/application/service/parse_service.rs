use crate::application::error::AppError;
use crate::domain::card::{Card, CardId, CollectionEntry};
use crate::domain::error::FunctionalError;
use crate::domain::language_code::LanguageCode;
use crate::domain::rarity_code::RarityCode;
use crate::domain::set_name::{SetCode, SetName};
use chrono::{DateTime, Utc};
use csv::{ReaderBuilder, Trim};
use std::collections::HashMap;
use uuid::Uuid;

pub fn parse_cards(csv: &str) -> Result<Vec<Card>, AppError> {
    let mut cards = Vec::new();

    if csv.lines().count() <= 1 {
        return Err(
            FunctionalError::WrongFormat("missing headers or empty file".to_string()).into(),
        );
    }

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(Trim::All)
        .from_reader(csv.as_bytes());

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

        let name = field_refs[2];
        let set_code =
            SetCode::try_new(field_refs[3]).map_err(|_| FunctionalError::ParseError {
                line: line_number,
                field: "set_code",
                value: field_refs[3].to_string(),
            })?;
        let set_name = SetName {
            code: set_code.clone(),
            name: field_refs[4].to_string(),
        };

        let collector_number = field_refs[5];

        let rarity_code =
            RarityCode::try_new(field_refs[7]).map_err(|_| FunctionalError::ParseError {
                line: line_number,
                field: "rarity",
                value: field_refs[7].to_string(),
            })?;

        let language_code: LanguageCode =
            LanguageCode::try_new(field_refs[15]).map_err(|_| FunctionalError::ParseError {
                line: line_number,
                field: "language_code",
                value: field_refs[15].to_string(),
            })?;
        let foil: bool = field_refs[6] != "normal";

        let quantity: u8 = field_refs[8]
            .parse()
            .map_err(|_e| FunctionalError::ParseError {
                line: line_number,
                field: "quantity",
                value: field_refs[8].to_string(),
            })?;

        let scryfall_id =
            Uuid::parse_str(field_refs[10]).map_err(|_e| FunctionalError::ParseError {
                line: line_number,
                field: "scryfall_id",
                value: field_refs[10].to_string(),
            })?;

        let purchase_price_float: f32 =
            field_refs[11]
                .parse()
                .map_err(|_e| FunctionalError::ParseError {
                    line: line_number,
                    field: "purchase_price",
                    value: field_refs[11].to_string(),
                })?;

        let purchase_price = (purchase_price_float * 100.0).round() as u32;

        let added_at: DateTime<Utc> = {
            let raw = field_refs[17];
            if raw.is_empty() {
                return Err(FunctionalError::ParseError {
                    line: line_number,
                    field: "added_at",
                    value: raw.to_string(),
                }
                .into());
            }
            DateTime::parse_from_rfc3339(raw)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_e| FunctionalError::ParseError {
                    line: line_number,
                    field: "added_at",
                    value: raw.to_string(),
                })?
        };

        CardId::try_new(
            set_code.clone(),
            collector_number,
            language_code.clone(),
            foil,
        )
        .map_err(|e| FunctionalError::ParseError {
            line: line_number,
            field: "collector_number",
            value: String::from(e),
        })?;

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
        cards.push(card);
    }

    let mut seen: HashMap<CardId, Card> = HashMap::new();
    let mut order: Vec<CardId> = Vec::new();
    for card in cards {
        if let Some(existing) = seen.get_mut(&card.id) {
            let CollectionEntry::Mine {
                quantity: existing_quantity,
                purchase_price: existing_purchase_price,
                added_at: existing_added_at,
                ..
            } = existing.collection_entry
            else {
                unreachable!("parsed cards always carry a CollectionEntry::Mine");
            };
            let CollectionEntry::Mine {
                quantity: new_quantity,
                purchase_price: new_purchase_price,
                added_at: new_added_at,
                ..
            } = card.collection_entry
            else {
                unreachable!("parsed cards always carry a CollectionEntry::Mine");
            };

            let new_qty = existing_quantity as u32 + new_quantity as u32;
            let total_cost = existing_purchase_price * existing_quantity as u32
                + new_purchase_price * new_quantity as u32;
            let added_at = existing_added_at.min(new_added_at);

            existing.collection_entry = CollectionEntry::Mine {
                quantity: new_qty.min(u8::MAX as u32) as u8,
                purchase_price: total_cost / new_qty,
                added_at,
                reserved: false,
            };
        } else {
            order.push(card.id.clone());
            seen.insert(card.id.clone(), card);
        }
    }

    Ok(order
        .into_iter()
        .map(|id| seen.remove(&id).unwrap())
        .collect())
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

        let cards = parse_cards(csv)?;

        assert_eq!(cards.len(), 3);

        assert_eq!(cards[0].id.set_code, SetCode::new("FDN"));
        assert_eq!(cards[0].id.collector_number, "87");
        assert_eq!(cards[0].id.language_code, LanguageCode::FR);
        assert!(!cards[0].id.foil);
        let CollectionEntry::Mine {
            quantity: q0,
            purchase_price: p0,
            ..
        } = cards[0].collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(q0, 3);
        assert_eq!(p0, 8);

        assert_eq!(cards[1].id.set_code, SetCode::new("GPT"));
        assert_eq!(cards[1].id.collector_number, "32");
        assert_eq!(cards[1].id.language_code, LanguageCode::FR);
        assert!(!cards[1].id.foil);
        let CollectionEntry::Mine {
            quantity: q1,
            purchase_price: p1,
            ..
        } = cards[1].collection_entry
        else {
            panic!("expected CollectionEntry::Mine");
        };
        assert_eq!(q1, 2);
        assert_eq!(p1, 17);

        assert_eq!(cards[2].id.set_code, SetCode::new("FDN"));
        assert_eq!(cards[2].id.collector_number, "217");
        assert_eq!(cards[2].id.language_code, LanguageCode::FR);
        assert!(!cards[2].id.foil);
        let CollectionEntry::Mine {
            quantity: q2,
            purchase_price: p2,
            ..
        } = cards[2].collection_entry
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

        let cards = parse_cards(csv)?;

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].name, "Dwynen, Gilt-Leaf Daen");
        assert_eq!(cards[0].id.set_code, SetCode::new("FDN"));
        assert_eq!(cards[0].id.collector_number, "217");

        Ok(())
    }

    #[test]
    fn import_cards_returns_error_for_invalid_set_code() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,\"Eirdu, Carrier of Dawn // Isilu, Carrier of Twilight\",EC,Lorwyn Eclipsed,13,normal,mythic,1,108961,b2d9d5ca-7e15-437a-bdfc-5972b42148fe,12.35,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let result = parse_cards(csv);
        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ParseError {
                line: 2,
                field: "set_code",
                value: _
            }))
        ));
    }

    #[test]
    fn import_cards_returns_error_for_invalid_language_code() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,\"Brigid, Clachan's Heart // Brigid, Doun's Mind\",ECL,Lorwyn Eclipsed,7,normal,rare,1,110841,cb7d5bbb-4f68-4e38-8bb0-a95af21b24c8,1.75,false,false,near_mint,xx,EUR,2026-02-05T20:44:45.815Z";

        let result = parse_cards(csv);
        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ParseError {
                line: 2,
                field: "language_code",
                value: _
            }))
        ));
    }

    #[test]
    fn import_cards_returns_error_for_invalid_quantity_number_format() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Stormshriek Feral // Flush Out,TDM,Tarkir: Dragonstorm,15,normal,common,1,104447,0ec92c44-7cf0-48a5-a3ca-bc633496d887,0.11,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z\n\
                   bulk,binder,Stormshriek Feral // Flush Out,TDM,Tarkir: Dragonstorm,15,normal,common,NOT_VALID_NUMBER,104447,0ec92c44-7cf0-48a5-a3ca-bc633496d887,0.11,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let result = parse_cards(csv);

        println!("{:?}", result);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ParseError {
                line: 3,
                field: "quantity",
                value: _,
            }))
        ));
    }

    #[test]
    fn import_cards_returns_error_for_invalid_float_format() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Stormshriek Feral // Flush Out,TDM,Tarkir: Dragonstorm,15,normal,common,1,104447,0ec92c44-7cf0-48a5-a3ca-bc633496d887,0a11,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let result = parse_cards(csv);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ParseError {
                line: 2,
                field: "purchase_price",
                value: _
            }))
        ));
    }

    #[test]
    fn import_cards_returns_error_for_too_long_collector_number() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Goblin Boarders,FDN,Foundations,12345678901,normal,common,3,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.08,false,false,near_mint,fr,EUR,2026-02-05T20:44:45.815Z";

        let result = parse_cards(csv);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ParseError {
                line: 2,
                field: "collector_number",
                value: _
            }))
        ));
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
                   My Deck,deck,Goblin Boarders,FDN,Foundations,87,normal,common,2,101506,4409a063-bf2a-4a49-803e-3ce6bd474353,0.10,false,false,near_mint,fr,EUR,2026-03-01T10:00:00.000Z";

        let cards = parse_cards(csv).unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id.set_code, SetCode::new("FDN"));
        assert_eq!(cards[0].id.collector_number, "87");
        let CollectionEntry::Mine {
            quantity,
            purchase_price,
            added_at,
            ..
        } = cards[0].collection_entry
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
    fn import_cards_returns_error_for_invalid_date_format() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Repeal,GPT,Guildpact,32,normal,common,2,27563,9e7dd929-4bba-46a6-86c9-b8ed853eb721,0.17,false,false,near_mint,fr,EUR,NOT_A_DATE";

        let result = parse_cards(csv);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ParseError {
                line: 2,
                field: "added_at",
                value: _,
            }))
        ));
    }

    #[test]
    fn import_cards_returns_error_for_empty_added_at() {
        let csv = "Binder Name,Binder Type,Name,Set code,Set name,Collector number,Foil,Rarity,Quantity,ManaBox ID,Scryfall ID,Purchase price,Misprint,Altered,Condition,Language,Purchase price currency,Added\n\
                   bulk,binder,Repeal,GPT,Guildpact,32,normal,common,2,27563,9e7dd929-4bba-46a6-86c9-b8ed853eb721,0.17,false,false,near_mint,fr,EUR,";

        let result = parse_cards(csv);

        assert!(matches!(
            result,
            Err(AppError::Functional(FunctionalError::ParseError {
                line: 2,
                field: "added_at",
                value: _,
            }))
        ));
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

        assert_eq!(result.clone().unwrap().len(), 1);
        assert_eq!(result.clone().unwrap()[0], card);
    }
}
