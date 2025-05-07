mod access_point;
mod device;
mod network_manager;
mod wireless;

use std::collections::HashMap;

use access_point::AccessPointProxy;
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
        let device = device_props(&connection, dev).await?;

        if let Device::Wireless(wifi_device) = device {
            wifi_device.scan().await?;
        }
    }

    Ok(())
}

#[derive(Debug)]
enum Device<'a> {
    Wireless(WifiDevice<'a>),
    Unrecognized,
}

#[derive(Debug)]
struct WifiDevice<'a> {
    proxy: WirelessProxy<'a>,
}

impl<'a> WifiDevice<'a> {
    async fn scan(&self) -> Result<()> {
        // Request a scan; this will send a PropertyChanged signal for the "LastScan" property once completed.
        self.proxy.request_scan(HashMap::new()).await?;
        let mut scan_changed_notif = self.proxy.receive_last_scan_changed().await;

        let mut last_scan_time: Option<i64> = None;

        // Wait for last_scan_time to change before proceeding
        while let Some(signal) = scan_changed_notif.next().await {
            let signal = signal.get().await?;
            if let Some(prev) = last_scan_time {
                if prev != signal {
                    break;
                }
            }
            last_scan_time = Some(signal);
        }

        // At this point, we can query the updated list of access points that were retrieved from this scan.
        let access_pts = self.proxy.get_access_points().await?;
        for ap in access_pts.iter() {
            let ap_proxy = AccessPointProxy::builder(self.proxy.inner().connection())
                .path(ap)?
                .build()
                .await?;
            let ssid = unsafe { String::from_utf8_unchecked(ap_proxy.ssid().await?) };
            let freq = ap_proxy.frequency().await? as f32 / 1000.0;
            println!("{ssid} ({freq:.1}GHz)");
        }

        Ok(())
    }
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
        2 => {
            let wifi_device = WifiDevice {
                proxy: WirelessProxy::new(conn, device_path).await?,
            };
            Device::Wireless(wifi_device)
        }
        _ => Device::Unrecognized,
    };

    Ok(device)
}
