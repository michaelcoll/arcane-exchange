pub mod create_sealed_product;
pub mod delete;
pub mod get;
pub mod list;
pub mod update;

#[cfg(test)]
pub(crate) mod test_fixtures {
    use crate::domain::card::CardId;
    use crate::domain::language_code::LanguageCode;
    use crate::domain::purchase::{
        Purchase, PurchaseAmount, PurchaseId, PurchaseKind, PurchasedCard, SealedProductType,
    };
    use crate::domain::set_name::{SetCode, SetName};
    use chrono::NaiveDate;

    pub fn purchase_id() -> PurchaseId {
        PurchaseId(uuid::Uuid::nil())
    }

    pub fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
    }

    pub fn booster(name: Option<&str>) -> Purchase {
        Purchase {
            id: purchase_id(),
            kind: PurchaseKind::SealedProduct {
                product_type: SealedProductType::Booster,
                name: name.map(str::to_string),
                set: SetName {
                    code: SetCode::new("FDN"),
                    name: "Foundations".to_string(),
                },
            },
            amount: PurchaseAmount::try_new(499).unwrap(),
            purchased_on: day(),
        }
    }

    pub fn single_card() -> Purchase {
        Purchase {
            id: purchase_id(),
            kind: PurchaseKind::SingleCard {
                card: PurchasedCard {
                    id: CardId::new("FDN", "1", LanguageCode::FR),
                    name: "Goblin".to_string(),
                    set: SetName {
                        code: SetCode::new("FDN"),
                        name: "Foundations".to_string(),
                    },
                    image: None,
                },
                foil: false,
                quantity: 4,
            },
            amount: PurchaseAmount::try_new(800).unwrap(),
            purchased_on: day(),
        }
    }
}
