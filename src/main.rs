use dotenv::dotenv;
use net_loginer::Authenticator;
use std::env;
use std::error::Error;
use log::LevelFilter;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level("net_loginer", LevelFilter::Info)
        .init()?;

    let user_id = env::var("EGATE_ID")?;
    let password = env::var("EGATE_PASSWORD")?;
    let interface = env::var("INTERFACE")?;

    log::info!("User ID: {}", user_id);
    log::info!("Password: {}", password);
    log::info!("Interface: {}", interface);

    let mut authenticator = Authenticator::new(user_id, password, interface)?;
    authenticator.perform_login().await?;
    log::info!("Successfully logged in!");

    Ok(())
}
