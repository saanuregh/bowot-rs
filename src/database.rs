use std::ops::Sub;

use chrono::{DateTime, Duration, Utc};
use sqlx::{
    postgres::{PgPool, PgQueryResult},
    query, query_as,
};

pub struct Guild<'a> {
    pool: &'a PgPool,
    guild_id: i64,
}

#[derive(Debug)]
pub struct Member {
    pub id: i64,
    pub last_daily: DateTime<Utc>,
    pub coins: i64,
    pub guild_id: i64,
}

impl<'a> Guild<'a> {
    pub fn new(pool: &'a PgPool, guild_id: impl Into<i64>) -> Self {
        Self {
            pool,
            guild_id: guild_id.into(),
        }
    }

    pub async fn delete(&self) -> anyhow::Result<Option<i64>> {
        Ok(query!(
            r#"
            DELETE FROM guilds
            WHERE id = $1
            RETURNING id
            "#,
            self.guild_id
        )
        .fetch_optional(self.pool)
        .await?
        .map(|row| row.id))
    }

    pub async fn insert(&self) -> anyhow::Result<PgQueryResult> {
        Ok(query!(
            r#"
            INSERT INTO guilds
            VALUES ($1)
            ON CONFLICT DO NOTHING
            "#,
            self.guild_id,
        )
        .execute(self.pool)
        .await?)
    }

    pub async fn delete_member(&self, member_id: impl Into<i64>) -> anyhow::Result<Option<i64>> {
        Ok(query!(
            r#"
            DELETE FROM members
            WHERE id = $1 AND guild_id = $2
            RETURNING id
            "#,
            member_id.into(),
            self.guild_id
        )
        .fetch_optional(self.pool)
        .await?
        .map(|row| row.id))
    }

    pub async fn insert_member(&self, member_id: impl Into<i64>) -> anyhow::Result<PgQueryResult> {
        let last_daily = Utc::now().sub(Duration::days(1));
        let coins: i64 = 0;
        Ok(query!(
            r#"
            INSERT INTO members
            VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING
            "#,
            member_id.into(),
            self.guild_id,
            coins,
            last_daily,
        )
        .execute(self.pool)
        .await?)
    }

    pub async fn get_members(&self) -> anyhow::Result<Vec<Member>> {
        Ok(query_as!(
            Member,
            r#"
            SELECT id, last_daily, coins, guild_id
            FROM members
            WHERE guild_id = $1
            "#,
            self.guild_id
        )
        .fetch_all(self.pool)
        .await?)
    }

    pub async fn get_member(&self, member_id: impl Into<i64>) -> anyhow::Result<Option<Member>> {
        Ok(query_as!(
            Member,
            r#"
            SELECT id, last_daily, coins, guild_id
            FROM members
            WHERE guild_id = $1 AND id = $2
            "#,
            self.guild_id,
            member_id.into()
        )
        .fetch_optional(self.pool)
        .await?)
    }

    pub async fn set_member_economy(
        &self,
        member_id: impl Into<i64>,
        coins: i64,
        last_daily: Option<DateTime<Utc>>,
    ) -> anyhow::Result<()> {
        let query = match last_daily {
            Some(d) => {
                query!(
                    r#"
                    UPDATE members
                    SET coins = $3, last_daily = $4
                    WHERE guild_id = $1 AND id = $2
                    "#,
                    self.guild_id,
                    member_id.into(),
                    coins,
                    d,
                )
            }
            None => {
                query!(
                    r#"
                    UPDATE members
                    SET coins = $3
                    WHERE guild_id = $1 AND id = $2
                    "#,
                    self.guild_id,
                    member_id.into(),
                    coins,
                )
            }
        };
        query.execute(self.pool).await?;
        Ok(())
    }
}

pub async fn get_all_guild_ids(pool: &PgPool) -> anyhow::Result<Vec<i64>> {
    Ok(query!(
        r#"
        SELECT id
        FROM guilds
        "#
    )
    .fetch_all(pool)
    .await?
    .iter()
    .map(|row| row.id)
    .collect())
}
