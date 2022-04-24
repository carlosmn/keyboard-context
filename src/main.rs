use std::collections::HashSet;
use std::ffi::OsStr;
use std::io::Result;
use std::process::ExitStatus;

use tokio::process::Command;
use tokio::sync::mpsc::{self, Receiver};
use tokio_stream::StreamExt;
use tokio_udev::{AsyncMonitorSocket, Enumerator, MonitorBuilder, MonitorSocket};

// An interesting device was added or removed. The identifier is arbitrary as
// long as it's unique
#[derive(Debug)]
enum Event {
    // A device was added with the given identifier
    Add(String),
    // A device was removed with the given identifier
    Remove(String),
}

#[tokio::main]
async fn main() {
    // Set up the listener and enumerator
    let mut enumerator = ergodox_enumerator().unwrap();

    let socket = usb_monitor().unwrap();
    let mut stream = AsyncMonitorSocket::new(socket).unwrap();

    let (tx, rx) = mpsc::channel(4);
    tokio::spawn(async move {
        management_thread(rx).await;
    });

    // The listener is set up so now we can scan for the devices that already
    // exist
    if let Some(device) = enumerator.scan_devices().unwrap().next() {
        let path = device.syspath().to_string_lossy().to_string();
        tx.send(Event::Add(path)).await.unwrap();
    } else if let Some(status) = change_setting(false, true).await {
        match status {
            Ok(status) => println!("non-success exit: {:?}", status.code()),
            Err(err) => println!("error with dconf: {}", err),
        }
    }

    // And now finally just keep waiting for events
    while let Some(v) = stream.next().await {
        let v = v.unwrap();
        let device = v.device();
        let action = device.action();

        if action == Some(OsStr::new("unbind")) {
            let path = device.syspath().to_string_lossy().to_string();
            tx.send(Event::Remove(path)).await.unwrap();
            continue;
        }

        if action != Some(OsStr::new("bind")) {
            continue;
        }
        if device.attribute_value("idVendor") != Some(OsStr::new("3297")) {
            continue;
        }
        if device.attribute_value("idProduct") != Some(OsStr::new("4976")) {
            continue;
        }

        let path = device.syspath().to_string_lossy().to_string();
        tx.send(Event::Add(path)).await.unwrap();
    }
}

async fn management_thread(mut rx: Receiver<Event>) {
    let mut devices = HashSet::new();

    while let Some(event) = rx.recv().await {
        let was_empty = devices.is_empty();
        match event {
            Event::Add(path) => devices.insert(path),
            Event::Remove(path) => devices.remove(&path),
        };

        let exit_code = change_setting(was_empty, devices.is_empty()).await;
        if let Some(status) = exit_code {
            match status {
                Ok(status) => println!("non-success exit: {:?}", status.code()),
                Err(err) => println!("error with dconf: {}", err),
            }
        }
    }
}

async fn change_setting(was_empty: bool, is_empty: bool) -> Option<Result<ExitStatus>> {
    let mut child = match (was_empty, is_empty) {
        (true, true) | (false, false) => return None,
        (true, false) => Command::new("dconf")
            .arg("write")
            .arg("/org/gnome/desktop/input-sources/xkb-options")
            .arg("@as []")
            .spawn()
            .expect("failed to spawn reset"),
        (false, true) => Command::new("dconf")
            .arg("write")
            .arg("/org/gnome/desktop/input-sources/xkb-options")
            .arg("['ctrl:swapcaps']")
            .spawn()
            .expect("failed to spawn set"),
    };

    match child.wait().await {
        Ok(status) if status.success() => None,
        Ok(status) => Some(Ok(status)),
        Err(err) => Some(Err(err)),
    }
}

// Returns an enumerator that just looks for ErgoDox EZ Glow
fn ergodox_enumerator() -> Result<Enumerator> {
    let mut enumerator = Enumerator::new()?;
    enumerator.match_subsystem("usb")?;
    enumerator.match_attribute("idVendor", "3297")?;
    enumerator.match_attribute("idProduct", "4976")?;

    Ok(enumerator)
}

fn usb_monitor() -> Result<MonitorSocket> {
    MonitorBuilder::new()?
        .match_subsystem_devtype("usb", "usb_device")?
        .listen()
}
