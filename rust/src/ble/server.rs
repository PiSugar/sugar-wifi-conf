use std::process::Stdio;
use std::sync::Arc;

use bluer::gatt::local::{Application, Service};
use tokio::process::Command;
use tokio::sync::watch;
use tokio::task;
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
    if let Err(err) = adapter.set_alias(args.name.clone()).await {
        log::warn!(
            "Failed to set Bluetooth adapter alias to '{}': {}",
            args.name,
            err
        );
    }
    if let Err(err) = adapter.set_pairable(false).await {
        log::warn!("Failed to disable Bluetooth pairing: {}", err);
    }

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
        local_name: Some(args.name.clone()),
        discoverable: Some(true),
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
                "BlueZ D-Bus advertisement failed: {}; falling back to controller advertising",
                err
            );
            if start_mgmt_advertisement(suuid::SERVICE_ID, &args.name).await {
                log::info!("Advertising via Linux MGMT fallback started");
                None
            } else if start_hci_advertisement(suuid::SERVICE_ID).await {
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
        stop_mgmt_advertisement().await;
        if let Some(task) = hci_watchdog {
            task.abort();
        }
        stop_hci_advertisement().await;
    }
    drop(adv_handle);
    drop(app_handle);

    Ok(())
}

async fn start_mgmt_advertisement(service_uuid: &str, local_name: &str) -> bool {
    stop_hci_advertisement().await;

    let service_uuid = service_uuid.to_string();
    let local_name = local_name.to_string();
    match timeout(
        Duration::from_secs(5),
        task::spawn_blocking(move || mgmt_add_advertising(&service_uuid, &local_name)),
    )
    .await
    {
        Ok(Ok(Ok(()))) => true,
        Ok(Ok(Err(err))) => {
            log::error!("Linux MGMT advertising fallback failed: {}", err);
            false
        }
        Ok(Err(err)) => {
            log::error!("Linux MGMT advertising fallback task failed: {}", err);
            false
        }
        Err(_) => {
            log::error!("Linux MGMT advertising fallback timed out");
            false
        }
    }
}

async fn stop_mgmt_advertisement() {
    let _ = timeout(
        Duration::from_secs(3),
        task::spawn_blocking(|| mgmt_remove_advertising(0)),
    )
    .await;
}

async fn start_hci_advertisement(service_uuid: &str) -> bool {
    stop_hci_advertisement().await;

    let set_params = [
        "cmd", "0x08", "0x0006", "00", "08", "00", "08", "00", "00", "00", "00", "00", "00", "00",
        "00", "07", "00",
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
    let mut command_args = vec!["-i", "hci0"];
    command_args.extend_from_slice(args);
    run_command("hcitool", &command_args).await
}

async fn run_command(program: &str, args: &[&str]) -> std::io::Result<std::process::Output> {
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::null()).kill_on_drop(true);

    match timeout(Duration::from_secs(5), command.output()).await {
        Ok(result) => result,
        Err(_) => Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            format!("{} {} timed out", program, args.join(" ")),
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

const MGMT_INDEX: u16 = 0;
const MGMT_OP_SET_BONDABLE: u16 = 0x0009;
const MGMT_OP_ADD_ADVERTISING: u16 = 0x003e;
const MGMT_OP_REMOVE_ADVERTISING: u16 = 0x003f;
const MGMT_EV_CMD_COMPLETE: u16 = 0x0001;
const MGMT_EV_CMD_STATUS: u16 = 0x0002;
const MGMT_STATUS_SUCCESS: u8 = 0x00;
const MGMT_STATUS_INVALID_PARAMS: u8 = 0x0d;
const MGMT_ADV_FLAG_CONNECTABLE: u32 = 1 << 0;
const MGMT_ADV_FLAG_MANAGED_FLAGS: u32 = 1 << 3;

const HCI_CHANNEL_CONTROL: u16 = 3;
const HCI_DEV_NONE: u16 = 0xffff;
const BTPROTO_HCI: libc::c_int = 1;

#[repr(C)]
struct SockAddrHci {
    hci_family: libc::sa_family_t,
    hci_dev: u16,
    hci_channel: u16,
}

fn mgmt_add_advertising(service_uuid: &str, local_name: &str) -> std::io::Result<()> {
    let socket = MgmtSocket::open()?;

    let _ = socket.send_cmd(MGMT_OP_REMOVE_ADVERTISING, MGMT_INDEX, &[0x00]);
    if let Err(err) = socket.send_cmd(MGMT_OP_SET_BONDABLE, MGMT_INDEX, &[0x00]) {
        log::warn!("Failed to disable Bluetooth bonding via MGMT: {}", err);
    }

    let adv_data = mgmt_service_uuid_adv_data(service_uuid);
    let scan_rsp = mgmt_local_name_scan_rsp(local_name);
    let mut payload = Vec::with_capacity(11 + adv_data.len() + scan_rsp.len());
    payload.push(1); // instance
    payload.extend_from_slice(
        &(MGMT_ADV_FLAG_CONNECTABLE | MGMT_ADV_FLAG_MANAGED_FLAGS).to_le_bytes(),
    );
    payload.extend_from_slice(&0u16.to_le_bytes()); // duration
    payload.extend_from_slice(&0u16.to_le_bytes()); // timeout
    payload.push(adv_data.len() as u8);
    payload.push(scan_rsp.len() as u8);
    payload.extend_from_slice(&adv_data);
    payload.extend_from_slice(&scan_rsp);

    socket.send_cmd(MGMT_OP_ADD_ADVERTISING, MGMT_INDEX, &payload)
}

fn mgmt_remove_advertising(instance: u8) -> std::io::Result<()> {
    MgmtSocket::open()?.send_cmd(MGMT_OP_REMOVE_ADVERTISING, MGMT_INDEX, &[instance])
}

struct MgmtSocket {
    fd: libc::c_int,
}

impl MgmtSocket {
    fn open() -> std::io::Result<Self> {
        let fd = unsafe {
            libc::socket(
                libc::AF_BLUETOOTH,
                libc::SOCK_RAW | libc::SOCK_CLOEXEC,
                BTPROTO_HCI,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let socket = Self { fd };
        let addr = SockAddrHci {
            hci_family: libc::AF_BLUETOOTH as libc::sa_family_t,
            hci_dev: HCI_DEV_NONE,
            hci_channel: HCI_CHANNEL_CONTROL,
        };

        let ret = unsafe {
            libc::bind(
                socket.fd,
                &addr as *const SockAddrHci as *const libc::sockaddr,
                std::mem::size_of::<SockAddrHci>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        let timeout = libc::timeval {
            tv_sec: 2,
            tv_usec: 0,
        };
        let ret = unsafe {
            libc::setsockopt(
                socket.fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                &timeout as *const libc::timeval as *const libc::c_void,
                std::mem::size_of::<libc::timeval>() as libc::socklen_t,
            )
        };
        if ret < 0 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(socket)
    }

    fn send_cmd(&self, opcode: u16, index: u16, payload: &[u8]) -> std::io::Result<()> {
        let mut packet = Vec::with_capacity(6 + payload.len());
        packet.extend_from_slice(&opcode.to_le_bytes());
        packet.extend_from_slice(&index.to_le_bytes());
        packet.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        packet.extend_from_slice(payload);

        let written = unsafe {
            libc::send(
                self.fd,
                packet.as_ptr() as *const libc::c_void,
                packet.len(),
                0,
            )
        };
        if written < 0 {
            return Err(std::io::Error::last_os_error());
        }

        self.wait_for_cmd_status(opcode)
    }

    fn wait_for_cmd_status(&self, opcode: u16) -> std::io::Result<()> {
        let mut buf = [0u8; 1024];
        loop {
            let len =
                unsafe { libc::recv(self.fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0) };
            if len < 0 {
                return Err(std::io::Error::last_os_error());
            }

            let len = len as usize;
            if len < 9 {
                continue;
            }

            let event = u16::from_le_bytes([buf[0], buf[1]]);
            let event_index = u16::from_le_bytes([buf[2], buf[3]]);
            let event_len = u16::from_le_bytes([buf[4], buf[5]]) as usize;
            if event_index != MGMT_INDEX || 6 + event_len > len {
                continue;
            }

            if event == MGMT_EV_CMD_COMPLETE || event == MGMT_EV_CMD_STATUS {
                let event_opcode = u16::from_le_bytes([buf[6], buf[7]]);
                let status = buf[8];
                if event_opcode != opcode {
                    continue;
                }

                if status == MGMT_STATUS_SUCCESS {
                    return Ok(());
                }

                if opcode == MGMT_OP_REMOVE_ADVERTISING && status == MGMT_STATUS_INVALID_PARAMS {
                    return Ok(());
                }

                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("MGMT opcode 0x{opcode:04x} failed with status 0x{status:02x}"),
                ));
            }
        }
    }
}

impl Drop for MgmtSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

fn mgmt_service_uuid_adv_data(service_uuid: &str) -> Vec<u8> {
    match service_uuid {
        "FD2B4448AA0F4A15A62FEB0BE77A0000" => vec![
            0x11, 0x07, 0x00, 0x00, 0x7A, 0xE7, 0x0B, 0xEB, 0x2F, 0xA6, 0x15, 0x4A, 0x0F, 0xAA,
            0x48, 0x44, 0x2B, 0xFD,
        ],
        _ => Vec::new(),
    }
}

fn mgmt_local_name_scan_rsp(local_name: &str) -> Vec<u8> {
    let local_name = truncate_utf8(local_name.trim(), 29);
    if local_name.is_empty() {
        return Vec::new();
    }

    let mut data = Vec::with_capacity(local_name.len() + 2);
    data.push((local_name.len() + 1) as u8);
    data.push(0x09); // Complete Local Name
    data.extend_from_slice(local_name.as_bytes());
    data
}

fn truncate_utf8(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }

    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}
