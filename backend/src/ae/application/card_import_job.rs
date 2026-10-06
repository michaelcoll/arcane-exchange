use crate::application::imported_card::ImportedCard;
use crate::domain::card_import::CardImportId;
use crate::domain::user::User;

/// A parsed import handed off to the background worker: the (already parsed and validated)
/// cards to write, and the id of the `CardImport` row tracking its progress.
pub struct CardImportJob {
    pub import_id: CardImportId,
    pub user: User,
    pub cards: Vec<ImportedCard>,
}
