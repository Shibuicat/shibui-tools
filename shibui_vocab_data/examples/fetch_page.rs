use shibui_vocab_data::utils::http_request::{ChallengeHttpRequestMaker, HttpRequestMaker};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: fetch_page <warm_up_url> <page_url> <out_file>";
    let warm_up = args.next().expect(usage);
    let url = args.next().expect(usage);
    let out = args.next().expect(usage);

    let maker = ChallengeHttpRequestMaker::new(&warm_up);
    maker.start_session().await;
    match maker.get(&url).await {
        Ok(html) => {
            std::fs::write(&out, &html).expect("cannot write the output file");
            println!("saved {} bytes to {out}", html.len());
        }
        Err(error) => println!("fetch failed: {error}"),
    }
    maker.release_session().await;
}
