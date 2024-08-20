pub trait HttpRequestMaker {
    async fn get(&self, url: &str) -> anyhow::Result<String>;
}

pub struct DefaultHttpRequestMaker {
    client: reqwest::Client,
}

impl DefaultHttpRequestMaker {
    pub async fn get(&self, url: &str) -> anyhow::Result<String> {
        let result = self.client.get(url).send().await?.text().await?;
        Ok(result)
    }

    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}
