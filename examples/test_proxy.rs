fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("HTTP_PROXY: {:?}", std::env::var("HTTP_PROXY"));
    println!("HTTPS_PROXY: {:?}", std::env::var("HTTPS_PROXY"));

    let client = reqwest::blocking::ClientBuilder::new().build()?;

    println!("\nAttempting request to api.github.com...");
    let resp = client
        .get("https://api.github.com/repos/maxfirman/dbtl/releases")
        .send()?;

    println!("Status: {}", resp.status());
    Ok(())
}
