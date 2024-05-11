use anyhow::Result;
use dotenv::dotenv;
use log::LevelFilter;
use net_loginer::{Authenticator, Classifier};
use simple_logger::SimpleLogger;
use std::env;

fn main() -> Result<()> {
    dotenv().ok();

    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("net_loginer", LevelFilter::Info)
        .init()?;

    let user_id = env::var("EGATE_ID").expect("EGATE_ID is not set!");
    let password = env::var("EGATE_PASSWORD").expect("EGATE_PASSWORD is not set!");

    log::info!("User ID: {}", user_id);
    log::info!("Password: {}", password);

    let model = include_bytes!("../model/shtu_captcha.onnx");
    let charset = serde_json::from_slice(include_bytes!("../model/charset.json"))?;
    let classifier = Classifier::new(model, charset, [-1, 64], 1)?;

    let authenticator = Authenticator::new(user_id, password, classifier)?;
    authenticator.perform_login()?;

    Ok(())
}
