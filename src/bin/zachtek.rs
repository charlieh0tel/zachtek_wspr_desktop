use clap::Parser;
use serialport::{ClearBuffer, SerialPort};
use std::io::{BufRead, BufReader};
use std::time::Duration;
use zacktek::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    port: String,
}

fn poll_thread(mut port: Box<dyn SerialPort>) {
    const GET_COMMANDS: &[&[u8]] = &[
        b"[CCM]", b"[CCR]", b"[OTP]", b"[OSM]", b"[OBD]", b"[OLC]", b"[OLP]", b"[OPW]", b"[OTS]",
        b"[OPS]", b"[OSC]", b"[DSC]", b"[DSF]", b"[DPF]", b"[DL4]", b"[DL6]", b"[DPD]", b"[DNM]",
        b"[DGF]", b"[DER]", b"[FPN]", b"[FHV]", b"[FHR]", b"[FSV]", b"[FSR]", b"[FRF]", b"[FLP]",
    ];
    const LF: &[u8] = b"\n";
    loop {
        for command in GET_COMMANDS {
            //println!("writing '{}'", ascii_bytes_to_string(command));
            port.write_all(LF).expect("Failed to write.");
            port.write_all(command).expect("Failed to write.");
            port.write_all(LF).expect("Failed to write.");
            port.flush().expect("Failed to write.");
            std::thread::sleep(Duration::from_millis(500));
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn main() {
    let args = Args::parse();
    let port_path = args.port;
    let baud_rate = 9_600;

    let mut port = serialport::new(&port_path, baud_rate)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_secs(10))
        .open()
        .unwrap_or_else(|_| panic!("Failed to open serial port at {}", port_path));

    set_run(&mut port);
    //reset_device(&mut port);

    let _ = std::thread::spawn({
        let port = port.try_clone().expect("Failed to clone port.");
        move || {
            poll_thread(port);
        }
    });

    port.clear(ClearBuffer::Input)
        .expect("Failed to clear input.");
    let mut reader = BufReader::new(port);
    loop {
        let mut buf = vec![];
        match reader.read_until(b'\n', &mut buf) {
            Ok(_) => {
                let response = process_line(buf);
                if response.is_some() {
                    println!("{:?}", response.unwrap());
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                panic!("Error: Timeout on serial port");
            }
            Err(e) => {
                panic!("Error: Failed to read from serial port: {}", e);
            }
        }
    }
}
