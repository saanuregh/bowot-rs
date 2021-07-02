use crate::utils::ytdl::{serde_value_to_ytdl, YoutubeDlOutput};
use bb8_redis::{
    bb8::Pool,
    redis::{self, aio::ConnectionLike},
    RedisConnectionManager,
};
use redis::Cmd;
use serde_json::{self, Value};

pub struct YtdlCache {
    pool: Pool<RedisConnectionManager>,
    query: String,
    data: Option<YoutubeDlOutput>,
}

impl YtdlCache {
    pub fn new(
        pool: Pool<RedisConnectionManager>,
        query: String,
        data: Option<YoutubeDlOutput>,
    ) -> Self {
        Self { pool, query, data }
    }

    pub async fn get(&self) -> anyhow::Result<YoutubeDlOutput> {
        let mut conn = self.pool.get().await?;
        let reply = Cmd::get(&self.query)
            .query_async::<_, String>(&mut *conn)
            .await?;
        let value = serde_json::from_str::<Value>(&reply)?;
        return Ok(serde_value_to_ytdl(value)?);
    }

    pub async fn set(&self) -> anyhow::Result<()> {
        let mut conn = self.pool.get().await?;

        if let Some(data) = &self.data {
            match data {
                YoutubeDlOutput::Playlist(data) => {
                    let entries = data.entries.clone().unwrap_or_default();
                    if entries.is_empty() {
                        return Ok(());
                    }
                    _set(&mut *conn, self.query.clone(), serde_json::to_string(data)?).await?;
                    if data.extractor.clone().unwrap() == "youtube:search" {
                        let search_result = &entries[0];
                        if search_result.duration.is_none() {
                            return Ok(());
                        }
                        _set(
                            &mut *conn,
                            search_result.webpage_url.clone().unwrap(),
                            serde_json::to_string(search_result)?,
                        )
                        .await?;
                    }
                }
                YoutubeDlOutput::SingleVideo(data) => {
                    if data.duration.is_none() {
                        return Ok(());
                    }
                    _set(
                        &mut *conn,
                        data.webpage_url.clone().unwrap(),
                        serde_json::to_string(data)?,
                    )
                    .await?;
                }
            };
        }

        Ok(())
    }
}

async fn _set<C: ConnectionLike>(conn: &mut C, query: String, data: String) -> anyhow::Result<()> {
    Ok(Cmd::set_ex(query, data, 864000).query_async(conn).await?)
}
