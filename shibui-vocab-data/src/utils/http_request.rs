use std::future::Future;

pub trait HttpRequestMaker {
    fn get(&self, url: &str) -> impl Future<Output = anyhow::Result<String>>;
}

pub struct DefaultHttpRequestMaker {
    client: reqwest::Client,
}

impl DefaultHttpRequestMaker {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl HttpRequestMaker for DefaultHttpRequestMaker {
    async fn get(&self, url: &str) -> anyhow::Result<String> {
        let result = self.client.get(url).send().await?.text().await?;
        Ok(result)
    }
}
