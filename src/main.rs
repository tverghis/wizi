mod device;
mod network_manager;
mod wireless;

use std::collections::HashMap;

use device::DeviceProxy;
use futures_util::stream::StreamExt;
use wireless::WirelessProxy;
use zbus::{zvariant::OwnedObjectPath, Connection, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let connection = Connection::system().await?;
    let proxy = network_manager::NetworkManagerProxy::new(&connection).await?;
    let x = proxy.get_devices().await?;

    for dev in x.iter() {
        let device = device_props(&connection, &dev).await?;

        if let Device::Wireless(wireless_device) = device {
            println!("Device {}", wireless_device.inner().path());
            wireless_device.request_scan(HashMap::new()).await?;
            let mut scan_changed_notif = wireless_device.receive_last_scan_changed().await;

            let mut last_scan_time: Option<i64> = None;

            while let Some(signal) = scan_changed_notif.next().await {
                let signal = signal.get().await?;
                if let Some(prev) = last_scan_time {
                    if prev != signal {
                        break;
                    }
                }
                last_scan_time = Some(signal);
            }

            let access_pts = wireless_device.get_access_points().await?;
            dbg!(access_pts);
        }
    }

    Ok(())
}

#[derive(Debug)]
enum Device<'a> {
    Wireless(WirelessProxy<'a>),
    Unrecognized,
}

async fn device_props<'a>(
    conn: &'a Connection,
    device_path: &'a OwnedObjectPath,
) -> Result<Device<'a>> {
    let proxy = DeviceProxy::builder(conn)
        .path(device_path)?
        .build()
        .await?;

    let dev_ty = proxy.device_type().await?;

    let device = match dev_ty {
        2 => Device::Wireless(WirelessProxy::new(conn, device_path).await?),
        _ => Device::Unrecognized,
    };

    Ok(device)
}
