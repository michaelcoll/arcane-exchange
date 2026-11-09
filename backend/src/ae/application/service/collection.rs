pub mod add_entry;
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
