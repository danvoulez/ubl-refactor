use tracing::error;
use ubl_gate::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ubl_gate::utils::init_tracing();
    let config = ubl_config::GateConfig::from_env();
    config
        .validate()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

    if let Err(e) = run(config).await {
        error!(error = %e, "gate terminated with error");
        return Err(e);
    }

    Ok(())
}
