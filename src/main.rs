mod access_point;
mod device;
mod network_manager;
mod wireless;

use device::Device;
use zbus::{Connection, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let connection = Connection::system().await?;
    do_wifi_scan(&connection).await?;

    Ok(())
}

async fn do_wifi_scan(conn: &Connection) -> Result<()> {
    let proxy = network_manager::NetworkManagerProxy::new(conn).await?;
    let x = proxy.get_devices().await?;

    for dev in x.iter() {
        let device = Device::from_object_path(conn, dev).await?;

        if let Device::Wireless(wifi_device) = device {
            wifi_device.scan().await?;
        }
    }

    Ok(())
}
