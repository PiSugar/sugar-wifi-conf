use std::sync::Arc;

use bluer::gatt::local::{Application, Service};
use tokio::sync::watch;

use crate::config::{Args, CustomConfig};
use crate::uuid as suuid;

/// Build and register the BLE GATT application, then advertise.
pub async fn run_ble_server(args: Args, custom_config: CustomConfig) -> bluer::Result<()> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    log::info!(
        "Advertising on adapter {} with address {}",
        adapter.name(),
        adapter.address().await?
    );

    // Channel for wifi config notify messages (input_notify → notify_message)
    let (notify_tx, notify_rx) = watch::channel::<String>(String::new());
    let notify_tx = Arc::new(notify_tx);

    // Channel for custom command notify messages
    let (cmd_notify_tx, cmd_notify_rx) = watch::channel::<String>(String::new());
    let cmd_notify_tx = Arc::new(cmd_notify_tx);

    // Build characteristics
    let mut characteristics = Vec::new();

    // 1. Service Name (read)
    characteristics.push(super::service_name::build());

    // 2. Device Model (read)
    characteristics.push(super::device_model::build());

    // 3. WiFi Name (notify)
    characteristics.push(super::wifi_name::build());

    // 4. IP Address (notify)
    characteristics.push(super::ip_address::build());

    // 5. Input (write, deprecated)
    let key_for_input = args.key.clone();
    let notify_tx_for_input = notify_tx.clone();
    characteristics.push(super::input_notify::build_input(
        key_for_input,
        notify_tx_for_input,
    ));

    // 6. InputSep (write, chunked)
    let key_for_sep = args.key.clone();
    let notify_tx_for_sep = notify_tx.clone();
    characteristics.push(super::input_notify::build_input_sep(
        key_for_sep,
        notify_tx_for_sep,
    ));

    // 7. Notify Message (notify)
    characteristics.push(super::input_notify::build_notify_message(notify_rx.clone()));

    // 8. Custom Info characteristics (dynamic from config)
    let info_chars = super::custom_info::build(&custom_config.info);
    characteristics.extend(info_chars);

    // 9. Custom Command characteristics (dynamic from config)
    let cmd_chars = super::custom_command::build(
        &custom_config.commands,
        args.key.clone(),
        cmd_notify_tx.clone(),
        cmd_notify_rx.clone(),
    );
    characteristics.extend(cmd_chars);

    // Build the GATT application
    let app = Application {
        services: vec![Service {
            uuid: suuid::parse_uuid(suuid::SERVICE_ID),
            primary: true,
            characteristics,
            ..Default::default()
        }],
        ..Default::default()
    };

    let app_handle = adapter.serve_gatt_application(app).await?;
    log::info!("GATT application registered");

    // Set up advertising
    let le_advertisement = bluer::adv::Advertisement {
        advertisement_type: bluer::adv::Type::Peripheral,
        service_uuids: vec![suuid::parse_uuid(suuid::SERVICE_ID)].into_iter().collect(),
        local_name: Some(args.name.clone()),
        discoverable: Some(true),
        ..Default::default()
    };

    let adv_handle = adapter.advertise(le_advertisement).await?;
    log::info!("Advertising as '{}' started", args.name);

    log::info!("BLE service running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await.ok();

    log::info!("Shutting down...");
    drop(adv_handle);
    drop(app_handle);

    Ok(())
}
