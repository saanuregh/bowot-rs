#![allow(dead_code)]
use crate::constants::DEFAULT_PREFIX;
use chrono::{DateTime, Duration, Utc};
use sqlx::{
    postgres::{PgPool, PgQueryResult},
    query, query_as,
};
use std::ops::Sub;

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

#[derive(Debug)]
pub struct Trigger {
    pub phrase: String,
    pub reply: String,
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
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
            self.guild_id,
            DEFAULT_PREFIX.to_string(),
            &vec![]
        )
        .execute(self.pool)
        .await?)
    }

    pub async fn get_prefix(&self) -> anyhow::Result<String> {
        Ok(query!(
            r#"
            SELECT prefix
            FROM guilds
            WHERE id = $1
            "#,
            self.guild_id
        )
        .fetch_one(self.pool)
        .await?
        .prefix)
    }

    pub async fn set_prefix(&self, prefix: &str) -> anyhow::Result<String> {
        Ok(query!(
            r#"
            UPDATE guilds
            SET prefix = $2
            WHERE id = $1 
            RETURNING prefix 
            "#,
            self.guild_id,
            prefix
        )
        .fetch_one(self.pool)
        .await?
        .prefix)
    }

    pub async fn get_disabled_commands(&self) -> anyhow::Result<Vec<String>> {
        Ok(query!(
            r#"
            SELECT disabled_commands
            FROM guilds
            WHERE id = $1
            "#,
            self.guild_id
        )
        .fetch_one(self.pool)
        .await?
        .disabled_commands)
    }

    pub async fn set_disabled_commands(
        &self,
        disabled_commands: Vec<String>,
    ) -> anyhow::Result<Vec<String>> {
        Ok(query!(
            r#"
            UPDATE guilds
            SET disabled_commands = $2
            WHERE id = $1
            RETURNING disabled_commands 
            "#,
            self.guild_id,
            &disabled_commands
        )
        .fetch_one(self.pool)
        .await?
        .disabled_commands)
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

    pub async fn delete_trigger(&self, phrase: String) -> anyhow::Result<Option<String>> {
        Ok(query!(
            r#"
            DELETE FROM triggers
            WHERE guild_id = $1 AND phrase = $2
            RETURNING phrase
            "#,
            self.guild_id,
            phrase
        )
        .fetch_optional(self.pool)
        .await?
        .map(|row| row.phrase))
    }

    pub async fn insert_trigger(
        &self,
        phrase: String,
        reply: String,
    ) -> anyhow::Result<PgQueryResult> {
        Ok(query!(
            r#"
            INSERT INTO triggers
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
            phrase,
            reply,
            self.guild_id,
        )
        .execute(self.pool)
        .await?)
    }

    pub async fn get_triggers(&self) -> anyhow::Result<Vec<Trigger>> {
        Ok(query_as!(
            Trigger,
            r#"
            SELECT phrase, reply, guild_id
            FROM triggers
            WHERE guild_id = $1
            "#,
            self.guild_id
        )
        .fetch_all(self.pool)
        .await?)
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

pub struct HydrateReminder<'a> {
    pool: &'a PgPool,
}

impl<'a> HydrateReminder<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_all(&self) -> anyhow::Result<Vec<i64>> {
        Ok(query!(
            r#"
            SELECT id
            FROM hydrate_reminders
            "#
        )
        .fetch_all(self.pool)
        .await?
        .iter()
        .map(|row| row.id)
        .collect())
    }

    pub async fn delete(&self, member_id: impl Into<i64>) -> anyhow::Result<Option<i64>> {
        Ok(query!(
            r#"
            DELETE FROM hydrate_reminders
            WHERE id = $1
            RETURNING id
            "#,
            member_id.into()
        )
        .fetch_optional(self.pool)
        .await?
        .map(|row| row.id))
    }

    pub async fn insert(&self, member_id: impl Into<i64>) -> anyhow::Result<PgQueryResult> {
        Ok(query!(
            r#"
            INSERT INTO hydrate_reminders
            VALUES ($1)
            ON CONFLICT DO NOTHING
            "#,
            member_id.into(),
        )
        .execute(self.pool)
        .await?)
    }
}
