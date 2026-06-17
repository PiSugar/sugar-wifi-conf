use std::process::Stdio;
use std::sync::Arc;

use bluer::gatt::local::{Application, Service};
use tokio::process::Command;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{interval, timeout, Duration};

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

    // 10. SSH tunnel characteristics (ctrl + rx + tx)
    let ssh_chars = super::ssh_tunnel::build();
    characteristics.extend(ssh_chars);

    // 11. SSH Username (read)
    characteristics.push(super::ssh_username::build());

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
        service_uuids: vec![suuid::parse_uuid(suuid::SERVICE_ID)]
            .into_iter()
            .collect(),
        ..Default::default()
    };

    let mut hci_watchdog: Option<JoinHandle<()>> = None;
    let adv_handle = match adapter.advertise(le_advertisement).await {
        Ok(handle) => {
            log::info!("Advertising as '{}' started", args.name);
            Some(handle)
        }
        Err(err) => {
            log::warn!(
                "BlueZ D-Bus advertisement failed: {}; falling back to hcitool",
                err
            );
            if start_hci_advertisement(suuid::SERVICE_ID).await {
                log::info!("Advertising via hcitool fallback started");
                hci_watchdog = Some(tokio::spawn(maintain_hci_advertisement()));
                None
            } else {
                return Err(err);
            }
        }
    };

    log::info!("BLE service running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await.ok();

    log::info!("Shutting down...");
    if adv_handle.is_none() {
        if let Some(task) = hci_watchdog {
            task.abort();
        }
        stop_hci_advertisement().await;
    }
    drop(adv_handle);
    drop(app_handle);

    Ok(())
}

async fn start_hci_advertisement(service_uuid: &str) -> bool {
    stop_hci_advertisement().await;

    let set_params = [
        "cmd", "0x08", "0x0006", "00", "08", "00", "08", "00", "00", "00", "00", "00", "00", "00",
        "00", "00", "07", "00",
    ];
    if !run_hcitool_success(&set_params).await {
        return false;
    }

    let adv_data = hci_service_uuid_adv_data(service_uuid);
    if !run_hcitool_success(&adv_data).await {
        return false;
    }

    run_hcitool_success(&["cmd", "0x08", "0x000A", "01"]).await
}

async fn stop_hci_advertisement() {
    let _ = run_hcitool(&["cmd", "0x08", "0x000A", "00"]).await;
}

async fn maintain_hci_advertisement() {
    let mut ticker = interval(Duration::from_secs(3));
    loop {
        ticker.tick().await;
        let _ = run_hcitool(&["cmd", "0x08", "0x000A", "01"]).await;
    }
}

async fn run_hcitool_success(args: &[&str]) -> bool {
    match run_hcitool(args).await {
        Ok(output) if output.status.success() && hci_command_succeeded(&output.stdout) => true,
        Ok(output) => {
            log::error!(
                "hcitool {} failed: status={:?}, stdout={}, stderr={}",
                args.join(" "),
                output.status.code(),
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
            false
        }
        Err(err) => {
            log::error!("failed to run hcitool {}: {}", args.join(" "), err);
            false
        }
    }
}

async fn run_hcitool(args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut command = Command::new("hcitool");
    command
        .args(["-i", "hci0"])
        .args(args)
        .stdin(Stdio::null())
        .kill_on_drop(true);

    match timeout(Duration::from_secs(5), command.output()).await {
        Ok(result) => result,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("hcitool {} timed out", args.join(" ")),
        )),
    }
}

fn hci_service_uuid_adv_data(service_uuid: &str) -> Vec<&'static str> {
    match service_uuid {
        "FD2B4448AA0F4A15A62FEB0BE77A0000" => vec![
            "cmd", "0x08", "0x0008", "15", "02", "01", "06", "11", "07", "00", "00", "7A", "E7",
            "0B", "EB", "2F", "A6", "15", "4A", "0F", "AA", "48", "44", "2B", "FD", "00", "00",
            "00", "00", "00", "00", "00", "00", "00", "00",
        ],
        _ => vec!["cmd", "0x08", "0x0008", "03", "02", "01", "06"],
    }
}

fn hci_command_succeeded(stdout: &[u8]) -> bool {
    String::from_utf8_lossy(stdout).lines().any(|line| {
        line.trim() == "01 0A 20 00" || line.trim() == "01 08 20 00" || line.trim() == "01 06 20 00"
    })
}
