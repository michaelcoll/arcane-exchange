/// How many Commander decks listed by EDHREC play a card, out of those whose colour identity
/// allows it (GLOSSARY: Taux d'inclusion). Stored as the two raw counts: the percentage is derived
/// when read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InclusionRate {
    pub num_decks: u32,
    pub potential_decks: u32,
}

impl InclusionRate {
    /// The share of decks playing the card, from 0 to 100. `None` when no deck could play it: the
    /// rate is then unknown, not zero.
    pub fn percentage(&self) -> Option<f64> {
        (self.potential_decks > 0)
            .then(|| f64::from(self.num_decks) / f64::from(self.potential_decks) * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_is_the_share_of_decks_playing_the_card() {
        let rate = InclusionRate {
            num_decks: 1,
            potential_decks: 4,
        };

        assert_eq!(rate.percentage(), Some(25.0));
    }

    #[test]
    fn percentage_is_unknown_when_no_deck_could_play_the_card() {
        let rate = InclusionRate {
            num_decks: 0,
            potential_decks: 0,
        };

        assert_eq!(rate.percentage(), None);
    }
}
