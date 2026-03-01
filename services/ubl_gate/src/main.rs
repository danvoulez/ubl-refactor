use tracing::error;
use ubl_gate::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ubl_config::AppConfig::from_env();
    ubl_gate::utils::init_tracing(&config.obs.rust_log);
    config
        .validate()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    if let Err(e) = run(config).await {
        error!(error = %e, "gate terminated with error");
        return Err(e);
    }

    Ok(())
}
