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
        let device = Device::from_object_path(&connection, dev).await?;

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

impl<'a> Device<'a> {
    async fn from_object_path(conn: &'a Connection, path: &'a OwnedObjectPath) -> Result<Self> {
        let proxy = DeviceProxy::builder(conn).path(path)?.build().await?;
        let dev_ty = proxy.device_type().await?;

        let device = match dev_ty {
            2 => {
                let wifi_device = WifiDevice::from_object_path(conn, path).await?;
                Device::Wireless(wifi_device)
            }
            _ => Device::Unrecognized,
        };

        Ok(device)
    }
}

#[derive(Debug)]
struct WifiDevice<'a> {
    proxy: WirelessProxy<'a>,
}

impl<'a> WifiDevice<'a> {
    async fn from_object_path(conn: &'a Connection, path: &'a OwnedObjectPath) -> Result<Self> {
        let proxy = WirelessProxy::new(conn, path).await?;

        Ok(Self { proxy })
    }

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
            let access_point =
                AccessPoint::from_object_path(self.proxy.inner().connection(), ap).await?;
            let ssid = access_point.ssid().await?;
            let freq = access_point.freq().await? as f32 / 1000.0;
            println!("{ssid} ({freq:.1}GHz)");
        }

        Ok(())
    }
}

#[derive(Debug)]
struct AccessPoint<'a> {
    proxy: AccessPointProxy<'a>,
}

impl<'a> AccessPoint<'a> {
    async fn from_object_path(conn: &'a Connection, path: &'a OwnedObjectPath) -> Result<Self> {
        let proxy = AccessPointProxy::builder(conn).path(path)?.build().await?;

        Ok(Self { proxy })
    }

    async fn ssid(&self) -> Result<String> {
        let ssid_bytes = match self.proxy.cached_ssid()? {
            Some(b) => b,
            None => self.proxy.ssid().await?,
        };

        let ssid_string = unsafe { String::from_utf8_unchecked(ssid_bytes) };

        Ok(ssid_string)
    }

    async fn freq(&self) -> Result<u32> {
        match self.proxy.cached_frequency()? {
            Some(f) => Ok(f),
            None => self.proxy.frequency().await,
        }
    }
}
