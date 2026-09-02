use super::Store;
use anyhow::Result;
use sqlx::Row;

impl Store {
    pub async fn set_frame_ability_card(
        &self,
        frame_id: &str,
        card_id: &str,
        card_name: &str,
        created_at: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO frame_ability_cards(frame_id, card_id, card_name, created_at) \
             VALUES(?,?,?,?) \
             ON CONFLICT(frame_id) DO UPDATE SET \
             card_id=excluded.card_id, \
             card_name=excluded.card_name, \
             created_at=excluded.created_at",
        )
        .bind(frame_id)
        .bind(card_id)
        .bind(card_name)
        .bind(created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn frame_ability_card(
        &self,
        frame_id: &str,
    ) -> Result<Option<(String, String)>> {
        let row = sqlx::query(
            "SELECT card_id, card_name FROM frame_ability_cards WHERE frame_id=?",
        )
        .bind(frame_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| {
            (
                row.try_get("card_id").unwrap_or_default(),
                row.try_get("card_name").unwrap_or_default(),
            )
        }))
    }

    /// Insert a daily resume marker. Returns true when this is the first report
    /// for the frame on `usage_date`.
    pub async fn mark_ability_card_resume_reported(
        &self,
        frame_id: &str,
        usage_date: &str,
        reported_at: i64,
    ) -> Result<bool> {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO ability_card_daily_resume(frame_id, usage_date, reported_at) \
             VALUES(?,?,?)",
        )
        .bind(frame_id)
        .bind(usage_date)
        .bind(reported_at)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn resume_dedupe_allows_one_report_per_day() {
        let root = std::env::temp_dir().join(format!("wisp-ability-card-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let store = Store::open(&root.join("store.sqlite")).await.unwrap();
        store.create_project("p1", "P", "").await.unwrap();
        store
            .create_frame("f1", "p1", "OPERON", "m")
            .await
            .unwrap();

        store
            .set_frame_ability_card("f1", "topic-coach", "选题引导", 10)
            .await
            .unwrap();
        assert_eq!(
            store.frame_ability_card("f1").await.unwrap(),
            Some(("topic-coach".into(), "选题引导".into()))
        );

        assert!(store
            .mark_ability_card_resume_reported("f1", "2026-09-02", 100)
            .await
            .unwrap());
        assert!(!store
            .mark_ability_card_resume_reported("f1", "2026-09-02", 101)
            .await
            .unwrap());
        assert!(store
            .mark_ability_card_resume_reported("f1", "2026-09-03", 102)
            .await
            .unwrap());

        drop(store);
        let _ = std::fs::remove_dir_all(root);
    }
}
