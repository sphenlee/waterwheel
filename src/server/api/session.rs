use highnoon::Result;
use highnoon::filter::session::SessionStore;

pub struct RedisSessionStore {
    client: redis::Client,
}

impl RedisSessionStore {
    pub fn new(url: &str) -> anyhow::Result<Self> {
        Ok(RedisSessionStore {
            client: redis::Client::open(url)?
        })
    }
}

#[async_trait::async_trait]
impl SessionStore for RedisSessionStore {
    async fn get(&self, id: &str) -> Result<Option<String>> {
        let mut con = self.client.get_tokio_connection().await?;
        let val = redis::Cmd::get(id).query_async(&mut con).await?;
        Ok(val)
    }

    async fn set(&mut self, id: String, value: String) -> Result<()> {
        let mut con = self.client.get_tokio_connection().await?;
        redis::Cmd::set(&id, &value).query_async(&mut con).await?;
        Ok(())
    }

    async fn clear(&mut self, id: &str) -> Result<()> {
        todo!()
    }
}
