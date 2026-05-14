use std::process::{Child, Command, Stdio};
use tracing::{info, warn};

/// Handle to a running `avahi-publish-service` subprocess. Dropping it kills the
/// child, which de-registers the service from the host's avahi-daemon.
pub struct AvahiPublish {
    child: Child,
}

impl Drop for AvahiPublish {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Publish the service through the host's avahi-daemon via DBus.
///
/// The container shares `/var/run/dbus` with the host, so `avahi-publish-service`
/// talks to the system avahi instead of opening its own UDP/5353 socket. This
/// avoids the bind conflict that silently breaks announcements when running
/// with `network_mode: host` alongside the system avahi-daemon.
pub fn announce(hostname: &str, port: u16) -> Result<AvahiPublish, Box<dyn std::error::Error>> {
    let service_type = "_knockshadow._tcp";

    let child = Command::new("avahi-publish-service")
        .arg(hostname)
        .arg(service_type)
        .arg(port.to_string())
        .arg("path=/")
        .arg("version=1.0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            warn!(
                "Failed to spawn avahi-publish-service: {}. Is avahi-utils installed in the image \
                 and /var/run/dbus mounted?",
                e
            );
            e
        })?;

    info!(
        "mDNS announce requested via avahi: {} {} port {}",
        hostname, service_type, port
    );
    Ok(AvahiPublish { child })
}
