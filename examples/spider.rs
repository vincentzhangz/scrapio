//! Spider example - define custom crawl behavior
//!
//! Run with: cargo run --example spider
use scrapio_classic::spider::{Item, Request, Spider, SpiderOutput};
use scrapio_runtime::{Runtime, TokioRuntime};

struct MySpider;

impl Spider for MySpider {
    fn name(&self) -> &str {
        "my_spider"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://www.rust-lang.org/".to_string()]
    }

    fn parse(&self, response: &scrapio_classic::Response) -> SpiderOutput {
        // Extract data item
        let mut item = Item::new();
        item.insert("url".to_string(), serde_json::json!(response.url));
        item.insert("title".to_string(), serde_json::json!(response.title()));

        // Extract links to follow (filter to same domain)
        let requests: Vec<Request> = response
            .links()
            .iter()
            .filter(|l| l.starts_with("https://www.rust-lang.org"))
            .map(Request::get)
            .collect();

        SpiderOutput::Both(vec![item], requests)
    }
}

fn main() {
    let runtime = TokioRuntime::default();
    runtime.block_on(async {
        let runner = scrapio_classic::spider::SpiderRunner::new().with_max_depth(2);
        let spider = MySpider;

        let items = runner.run(&spider).await;
        println!("Extracted {} items", items.len());

        for item in &items {
            println!("Item: {}", serde_json::to_string(item).unwrap());
        }
    });
}
