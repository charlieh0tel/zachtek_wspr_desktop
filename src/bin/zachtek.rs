use anyhow::{Context, Result};
use clap::Parser;
use std::num::ParseIntError;
use std::time::Duration;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use zachtek::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Serial port.
    #[arg(short, long)]
    port: String,

    /// Tracing level.
    #[arg(short, long, default_value_t=Level::INFO)]
    level: tracing::Level,

    /// Timeout (seconds).
    #[arg(short, long, value_parser = parse_duration_in_seconds, default_value="10")]
    timeout: Duration,

    /// Poll sleep interval (seconds).
    #[arg(long, value_parser = parse_duration_in_seconds, default_value="10")]
    poll_sleep_interval: Duration,
}

fn parse_duration_in_seconds(arg: &str) -> Result<Duration, ParseIntError> {
    Ok(Duration::from_secs(arg.parse()?))
}

fn main() -> Result<()> {
    let args = Args::parse();

    let subscriber = FmtSubscriber::builder().with_max_level(args.level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let port_path = args.port;
    let baud_rate = 9_600;
    let mut port = serialport::new(&port_path, baud_rate)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(args.timeout)
        .open()
        .with_context(|| format!("Failed to open serial port at {}", port_path))?;

    let mut device = ZachtekDevice::new(&mut port);

    device.set_run()?;
    device.start_poll_thread(args.poll_sleep_interval);
    device.clear_input()?;
    loop {
        match device.read_response() {
            Ok(response) => {
                println!("{response:?}");
            }
            Err(err) => {
                println!("Err: {err:?}");
            }
        }
    }
}
