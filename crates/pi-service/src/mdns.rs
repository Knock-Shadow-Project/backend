use mdns_sd::ServiceDaemon;
use tracing::info;

pub fn announce(hostname: &str, port: u16) -> Result<ServiceDaemon, Box<dyn std::error::Error>> {
    let mdns = ServiceDaemon::new()?;
    let service_type = "_knockshadow._tcp.local.";
    let instance_name = format!("{}.{}", hostname, service_type);

    let my_addrs: Vec<std::net::IpAddr> = get_if_addrs::get_if_addrs()?
        .into_iter()
        .filter(|iface| !iface.is_loopback() && iface.ip().is_ipv4())
        .map(|iface| iface.ip())
        .collect();

    let service_info = mdns_sd::ServiceInfo::new(
        service_type,
        &instance_name,
        &format!("{}.local.", hostname),
        &my_addrs[..],
        port,
        &[("path", "/"), ("version", "1.0")][..],
    )?
    .to_owned();

    mdns.register(service_info)?;
    info!("mDNS service registered: {} on port {}", instance_name, port);
    Ok(mdns)
}
