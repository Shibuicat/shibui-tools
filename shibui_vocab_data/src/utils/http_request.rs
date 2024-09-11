use std::future::Future;

use anyhow::bail;

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
        let response = self.client.get(url).send().await?;
        //verify current url in case of redirect to /dictionary/english since we let reqwest
        //handle auto redirect
        if response.url().path() == "/dictionary/english/" {
            bail!(format!("Endpoint {url} doesn't exist"));
        }

        let result = response.text().await?;
        Ok(result)
    }
}
