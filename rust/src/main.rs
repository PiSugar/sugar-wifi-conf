mod ble;
mod cli;
mod config;
mod uuid;
mod wifi;

use clap::Parser;
use config::{Args, Command, CustomConfig};

#[tokio::main]
async fn main() -> bluer::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    match args.command {
        Some(Command::Config) => {
            cli::run_config_editor(&args.config);
            Ok(())
        }
        Some(Command::Serve) | None => {
            log::info!("sugar-wifi-conf starting");
            log::info!("  BLE name: {}", args.name);
            log::info!("  Config: {}", args.config);

            let custom_config = CustomConfig::load(&args.config);
            log::info!(
                "  Custom info items: {}, commands: {}",
                custom_config.info.len(),
                custom_config.commands.len()
            );

            // Wait 10 seconds for Bluetooth to stabilize (matches JS behavior)
            log::info!("Waiting 10 seconds for Bluetooth to stabilize...");
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;

            log::info!("Starting BLE server...");
            ble::server::run_ble_server(args, custom_config).await
        }
    }
}
