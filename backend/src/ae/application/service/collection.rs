pub mod add_entry;
pub mod attach_card_to_sealed_product;
pub mod get;
pub mod get_card_import;
pub mod get_entries;
pub mod get_price_history;
pub mod get_stats;
pub mod import_card;
pub mod move_entry;
pub mod remove_entry;
pub mod run_card_import;
pub mod update_entry;

/// Test fixtures shared by the tests of the Collection entry use cases.
#[cfg(test)]
mod test_fixtures {
    use crate::application::repository::MockCardPricesRefreshRequester;
    use crate::application::use_case::MockCreateCardUseCase;
    use crate::domain::card::CardId;
    use crate::domain::collection_entry::CollectionEntryId;
    use uuid::Uuid;

    pub(super) fn entry_id() -> CollectionEntryId {
        CollectionEntryId(Uuid::from_u128(1))
    }

    /// A card creation that hands back the card id with its collector number normalized.
    pub(super) fn creating_card(times: usize) -> MockCreateCardUseCase {
        let mut create_card = MockCreateCardUseCase::new();
        create_card
            .expect_create_card()
            .times(times)
            .returning(|card_id| {
                Box::pin(async move {
                    Ok(CardId::new(
                        card_id.set_code,
                        card_id.collector_number.trim_start_matches('0').to_string(),
                        card_id.language_code,
                    ))
                })
            });
        create_card
    }

    /// A refresh requester that expects to be asked exactly `times` times.
    pub(super) fn refresh(times: usize) -> MockCardPricesRefreshRequester {
        let mut refresh = MockCardPricesRefreshRequester::new();
        refresh
            .expect_request_card_prices_refresh()
            .times(times)
            .return_const(());
        refresh
    }
}
