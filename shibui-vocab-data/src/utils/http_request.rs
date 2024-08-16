pub struct HttpRequestMaker {
    client: reqwest::Client,
}

impl HttpRequestMaker {
    async fn get(word: &str) -> anyhow::Result<String> {
        todo!()
    }

    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}
