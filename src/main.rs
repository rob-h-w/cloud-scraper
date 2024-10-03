use cloud_scraper::{main_impl, CoreInterface};

#[tokio::main]
async fn main() -> Result<(), String> {
    main_impl::<CoreInterface>().await
}
