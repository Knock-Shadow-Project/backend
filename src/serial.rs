use std::io::{self, Read};
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use serialport::{SerialPortInfo, SerialPortType};

const ST_VID: u16 = 0x0483;

#[derive(Debug, Parser)]
#[command(author, version, about = "List or read serial ports")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    List,
    Read {
        #[arg(long)]
        port: Option<String>,
        #[arg(long, default_value_t = 115_200)]
        baud: u32,
        #[arg(long, default_value_t = 1000)]
        timeout_ms: u64,
        #[arg(long, value_enum, default_value_t = OutputFormat::Hex)]
        format: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Hex,
    Utf8,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command.unwrap_or(Command::List) {
        Command::List => list_ports(),
        Command::Read {
            port,
            baud,
            timeout_ms,
            format,
        } => read_port(port, baud, timeout_ms, format),
    };

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn list_ports() -> Result<(), Box<dyn std::error::Error>> {
    let mut ports = serialport::available_ports()?;
    ports.sort_by(|left, right| left.port_name.cmp(&right.port_name));

    if ports.is_empty() {
        println!("No serial devices found.");
        return Ok(());
    }

    for port in ports {
        println!("{}{}", port.port_name, format_port_metadata(&port));
    }

    Ok(())
}

fn read_port(
    requested_port: Option<String>,
    baud: u32,
    timeout_ms: u64,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let port_name = match requested_port {
        Some(port) => port,
        None => auto_select_port()?,
    };

    let mut port = serialport::new(&port_name, baud)
        .timeout(Duration::from_millis(timeout_ms))
        .open()?;

    println!("Reading from {port_name} at {baud} baud");

    let mut buffer = [0_u8; 4096];
    loop {
        match port.read(&mut buffer) {
            Ok(read_len) if read_len > 0 => print_payload(&buffer[..read_len], format),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => return Err(Box::new(error)),
        }
    }
}

fn auto_select_port() -> Result<String, Box<dyn std::error::Error>> {
    let ports = serialport::available_ports()?;

    if let Some(port) = ports.iter().find(|port| is_st_port(port)) {
        return Ok(port.port_name.clone());
    }

    match ports.as_slice() {
        [port] => Ok(port.port_name.clone()),
        [] => Err("No serial ports are available. The SensorTile is visible on USB, but Linux has not created a /dev/ttyACM* device yet.".into()),
        _ => Err("Multiple serial ports found. Pass --port <device> explicitly.".into()),
    }
}

fn print_payload(bytes: &[u8], format: OutputFormat) {
    match format {
        OutputFormat::Hex => {
            let line = bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            println!("{line}");
        }
        OutputFormat::Utf8 => print!("{}", String::from_utf8_lossy(bytes)),
    }
}

fn is_st_port(port: &SerialPortInfo) -> bool {
    match &port.port_type {
        SerialPortType::UsbPort(info) => info.vid == ST_VID,
        _ => false,
    }
}

fn format_port_metadata(port: &SerialPortInfo) -> String {
    let st_marker = if is_st_port(port) { " [ST]" } else { "" };

    match &port.port_type {
        SerialPortType::UsbPort(info) => format!(
            "{st_marker} [USB vid={:#06x} pid={:#06x} manufacturer={} product={} serial={}]",
            info.vid,
            info.pid,
            info.manufacturer.as_deref().unwrap_or("unknown"),
            info.product.as_deref().unwrap_or("unknown"),
            info.serial_number.as_deref().unwrap_or("unknown")
        ),
        SerialPortType::BluetoothPort => format!("{st_marker} [Bluetooth]"),
        SerialPortType::PciPort => format!("{st_marker} [PCI]"),
        SerialPortType::Unknown => st_marker.to_string(),
    }
}
