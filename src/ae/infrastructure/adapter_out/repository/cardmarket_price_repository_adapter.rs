use crate::application::error::AppError;
use crate::application::repository::CardMarketPriceRepository;
use crate::domain::price::{FullPriceGuide, PriceHistoryEntry};
use crate::infrastructure::adapter_out::repository::entities::{
    CardMarketPriceEntity, CardMarketPriceHistoryEntity, CardMarketPriceRaw,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{Pool, Postgres, QueryBuilder};

pub struct CardMarketPriceRepositoryAdapter {
    pool: Pool<Postgres>,
}

impl CardMarketPriceRepositoryAdapter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CardMarketPriceRepository for CardMarketPriceRepositoryAdapter {
    async fn save(
        &self,
        date: NaiveDate,
        price_guides: Vec<FullPriceGuide>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        const CHUNK_SIZE: usize = 1000;

        for chunk in price_guides.chunks(CHUNK_SIZE) {
            let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
                "
                INSERT INTO cardmarket_price
                    (id_produit, date, low, trend, avg,
                     low_foil, trend_foil, avg_foil)
                ",
            );

            qb.push_values(chunk, |mut b, price_guide| {
                b.push_bind(price_guide.id_product as i32)
                    .push_bind(date)
                    .push_bind(price_guide.normal.low.as_i32())
                    .push_bind(price_guide.normal.trend.as_i32())
                    .push_bind(price_guide.normal.avg.as_i32())
                    .push_bind(price_guide.foil.low.as_i32())
                    .push_bind(price_guide.foil.trend.as_i32())
                    .push_bind(price_guide.foil.avg.as_i32());
            });

            qb.push(
                "
                ON CONFLICT (id_produit, date)
                DO UPDATE SET
                    low        = EXCLUDED.low,
                    trend      = EXCLUDED.trend,
                    avg        = EXCLUDED.avg,
                    low_foil   = EXCLUDED.low_foil,
                    trend_foil = EXCLUDED.trend_foil,
                    avg_foil   = EXCLUDED.avg_foil
                ",
            );

            qb.build().execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn find_by_id_and_date(
        &self,
        id_product: u32,
        date: NaiveDate,
    ) -> Result<Option<FullPriceGuide>, AppError> {
        let record = sqlx::query_as!(
            CardMarketPriceRaw,
            "SELECT id_produit, date, low, trend, avg,
                    low_foil, trend_foil, avg_foil
             FROM cardmarket_price
             WHERE id_produit = $1 AND date = $2",
            id_product as i32,
            date
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record
            .map(CardMarketPriceEntity::from)
            .map(FullPriceGuide::from))
    }

    async fn find_by_id_and_date_range(
        &self,
        id_product: u32,
        foil: bool,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<PriceHistoryEntry>, AppError> {
        let entities = sqlx::query_as!(
            CardMarketPriceHistoryEntity,
            r#"SELECT date,
                      CASE WHEN $2::boolean THEN low_foil ELSE low END as "low",
                      CASE WHEN $2::boolean THEN trend_foil ELSE trend END as "trend",
                      CASE WHEN $2::boolean THEN avg_foil ELSE avg END as "avg"
               FROM cardmarket_price
               WHERE id_produit = $1 AND date >= $3 AND date <= $4
               ORDER BY date"#,
            id_product as i32,
            foil,
            start_date,
            end_date,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(entities.into_iter().map(PriceHistoryEntry::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::price::{Price, PriceGuide};
    use crate::infrastructure::adapter_out::repository::common_repository_tests::fetch_cardmarket_price;
    use crate::infrastructure::adapter_out::repository::entities::PriceGuideEntity;
    use chrono::NaiveDate;

    impl PriceGuide {
        pub fn empty() -> Self {
            Self {
                low: Price::empty(),
                trend: Price::empty(),
                avg: Price::empty(),
            }
        }
    }

    impl FullPriceGuide {
        pub fn from_values(
            id_product: u32,
            normal_values: (i32, i32, i32),
            foil_values: (i32, i32, i32),
        ) -> Self {
            FullPriceGuide {
                id_product,
                normal: PriceGuide::from(PriceGuideEntity {
                    low: Some(normal_values.0),
                    trend: Some(normal_values.1),
                    avg: Some(normal_values.2),
                }),
                foil: PriceGuide::from(PriceGuideEntity {
                    low: Some(foil_values.0),
                    trend: Some(foil_values.1),
                    avg: Some(foil_values.2),
                }),
            }
        }
    }

    #[sqlx::test]
    async fn test_save_new_cardmarket_price(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let id_produit = 12345u32;
        let price_guides =
            FullPriceGuide::from_values(id_produit, (100, 150, 125), (200, 250, 225));

        let result = repository.save(date, vec![price_guides]).await;

        assert!(result.is_ok());

        let record = fetch_cardmarket_price(&pool, id_produit as i32, date).await;

        assert_eq!(record.id_produit, id_produit as i32);
        assert_eq!(record.date, date);
        assert_eq!(record.normal.low, Some(100));
        assert_eq!(record.normal.trend, Some(150));
        assert_eq!(record.normal.avg, Some(125));
        assert_eq!(record.foil.low, Some(200));
        assert_eq!(record.foil.trend, Some(250));
        assert_eq!(record.foil.avg, Some(225));
    }

    #[sqlx::test]
    async fn test_save_updates_existing_cardmarket_price(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let id_produit = 12346u32;

        let initial_price_guides =
            FullPriceGuide::from_values(id_produit, (100, 150, 125), (200, 250, 225));
        repository
            .save(date, vec![initial_price_guides])
            .await
            .unwrap();

        let updated_price_guides =
            FullPriceGuide::from_values(id_produit, (110, 160, 135), (210, 260, 235));
        let result = repository.save(date, vec![updated_price_guides]).await;

        assert!(result.is_ok());

        let record = fetch_cardmarket_price(&pool, id_produit as i32, date).await;

        assert_eq!(record.normal.low, Some(110));
        assert_eq!(record.normal.trend, Some(160));
        assert_eq!(record.normal.avg, Some(135));
        assert_eq!(record.foil.low, Some(210));
        assert_eq!(record.foil.trend, Some(260));
        assert_eq!(record.foil.avg, Some(235));
    }

    #[sqlx::test]
    async fn test_save_with_empty_price_values(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let id_produit = 12347u32;

        let price_guides = FullPriceGuide {
            id_product: id_produit,
            normal: PriceGuide::empty(),
            foil: PriceGuide::empty(),
        };

        let result = repository.save(date, vec![price_guides]).await;

        assert!(result.is_ok());

        let record = fetch_cardmarket_price(&pool, id_produit as i32, date).await;

        assert_eq!(record.id_produit, id_produit as i32);
        assert_eq!(record.date, date);
        assert_eq!(record.normal.low, None);
        assert_eq!(record.normal.trend, None);
        assert_eq!(record.normal.avg, None);
        assert_eq!(record.foil.low, None);
        assert_eq!(record.foil.trend, None);
        assert_eq!(record.foil.avg, None);
    }

    #[sqlx::test]
    async fn test_save_handles_multiple_products_same_date(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let price_guides_1 = FullPriceGuide::from_values(12348, (100, 150, 125), (200, 250, 225));
        let price_guides_2 = FullPriceGuide::from_values(12349, (300, 350, 325), (400, 450, 425));

        let result1 = repository.save(date, vec![price_guides_1]).await;
        let result2 = repository.save(date, vec![price_guides_2]).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cardmarket_price WHERE date = $1")
                .bind(date)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(count, 2);
    }

    #[sqlx::test]
    async fn test_find_by_id_and_date_returns_existing_record(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let id_produit = 12350u32;

        let price_guides =
            FullPriceGuide::from_values(id_produit, (100, 150, 125), (200, 250, 225));
        repository.save(date, vec![price_guides]).await.unwrap();

        let result = repository.find_by_id_and_date(id_produit, date).await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let guide = found.unwrap();
        assert_eq!(guide.id_product, id_produit);
        assert_eq!(guide.normal.low.value, Some(100));
        assert_eq!(guide.normal.trend.value, Some(150));
        assert_eq!(guide.normal.avg.value, Some(125));
        assert_eq!(guide.foil.low.value, Some(200));
        assert_eq!(guide.foil.trend.value, Some(250));
        assert_eq!(guide.foil.avg.value, Some(225));
    }

    #[sqlx::test]
    async fn test_find_by_id_and_date_returns_none_when_not_found(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let result = repository.find_by_id_and_date(99999u32, date).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[sqlx::test]
    async fn find_by_id_and_date_range_returns_normal_prices_sorted_by_date(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let id_produit = 20001u32;
        let date1 = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let date2 = NaiveDate::from_ymd_opt(2024, 1, 16).unwrap();

        repository
            .save(
                date2,
                vec![FullPriceGuide::from_values(
                    id_produit,
                    (12, 17, 22),
                    (120, 170, 220),
                )],
            )
            .await
            .unwrap();
        repository
            .save(
                date1,
                vec![FullPriceGuide::from_values(
                    id_produit,
                    (10, 15, 20),
                    (100, 150, 200),
                )],
            )
            .await
            .unwrap();

        let result = repository
            .find_by_id_and_date_range(id_produit, false, date1, date2)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].date, date1);
        assert_eq!(result[0].price_guide.low.value, Some(10));
        assert_eq!(result[0].price_guide.trend.value, Some(15));
        assert_eq!(result[0].price_guide.avg.value, Some(20));
        assert_eq!(result[1].date, date2);
        assert_eq!(result[1].price_guide.low.value, Some(12));
    }

    #[sqlx::test]
    async fn find_by_id_and_date_range_returns_foil_prices_when_foil_true(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let id_produit = 20002u32;
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        repository
            .save(
                date,
                vec![FullPriceGuide::from_values(
                    id_produit,
                    (10, 15, 20),
                    (100, 150, 200),
                )],
            )
            .await
            .unwrap();

        let result = repository
            .find_by_id_and_date_range(id_produit, true, date, date)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].price_guide.low.value, Some(100));
        assert_eq!(result[0].price_guide.trend.value, Some(150));
        assert_eq!(result[0].price_guide.avg.value, Some(200));
    }

    #[sqlx::test]
    async fn find_by_id_and_date_range_bounds_are_inclusive(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let id_produit = 20003u32;
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        repository
            .save(
                date,
                vec![FullPriceGuide::from_values(
                    id_produit,
                    (10, 15, 20),
                    (100, 150, 200),
                )],
            )
            .await
            .unwrap();

        let result = repository
            .find_by_id_and_date_range(id_produit, false, date, date)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
    }

    #[sqlx::test]
    async fn find_by_id_and_date_range_returns_empty_when_no_price_in_range(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let id_produit = 20004u32;
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        repository
            .save(
                date,
                vec![FullPriceGuide::from_values(
                    id_produit,
                    (10, 15, 20),
                    (100, 150, 200),
                )],
            )
            .await
            .unwrap();

        let result = repository
            .find_by_id_and_date_range(
                id_produit,
                false,
                NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 2, 28).unwrap(),
            )
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[sqlx::test]
    async fn find_by_id_and_date_range_returns_empty_when_product_unknown(pool: Pool<Postgres>) {
        let repository = CardMarketPriceRepositoryAdapter::new(pool.clone());
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();

        let result = repository
            .find_by_id_and_date_range(99999u32, false, date, date)
            .await
            .unwrap();

        assert!(result.is_empty());
    }
}
