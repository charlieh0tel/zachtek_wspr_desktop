use ascii::AsciiStr;
use clap::Parser;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serialport::{ClearBuffer, SerialPort};
use std::io::prelude::*;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    port: String,
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum Mode {
    Sig = b'S',
    Wspr = b'W',
    Idle = b'N',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum FilterBank {
    A = b'A',
    B = b'B',
    C = b'C',
    D = b'D',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum Reference {
    External = b'E',
    Internal = b'I',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum LocationSource {
    Gps = b'G',
    Manual = b'M',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum LocationPrecision {
    Maidenhead4 = b'4',
    Maidenhead6 = b'6',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum PowerEncoding {
    Normal = b'N',
    Altitude = b'A',
}

#[derive(Debug, Clone, Copy)]
enum TimeSlot {
    TenMinute,
    TwentyMinute,
    BandCoordinated,
    NoSchedule,
    Tracker,
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum PrefixSuffix {
    Prefix = b'P',
    Suffix = b'S',
    None = b'N',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum Constellation {
    GPSOnly = b'G',
    BeiDouOnly = b'B',
    All = b'A',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum Band {
    B2190m = 0,
    B630m = 1,
    B160m = 2,
    B80m = 3,
    B40m = 4,
    B30m = 5,
    B20m = 6,
    B17m = 7,
    B15m = 8,
    B12m = 9,
    B10m = 10,
    B6m = 11,
    B4m = 12,
    B2m = 13,
    B70Cm = 14,
    B23Cm = 15,
    NoFilter = 98,
    Open = 99,
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
enum GpsLock {
    Locked = b'T',
    Unlocked = b'F',
}

fn ascii_bytes_to_string(bytes: &[u8]) -> String {
    AsciiStr::from_ascii(bytes)
        .unwrap_or_else(|_| panic!("could not interpret bytes: {:?}", bytes))
        .to_string()
}

fn parse_enum<T: TryFrom<u8>>(command_string: &String, args: &[u8]) -> Result<T, ()> {
    if args.len() != 1 {
        println!("Wrong length {command_string} response.");
        return Err(());
    }
    let first_byte = args[0];
    T::try_from(first_byte).map_err(|_| ())
}

fn parse_enum_from_number<T: TryFrom<u8>>(command_string: &String, args: &[u8]) -> Result<T, ()> {
    if args.is_empty() || args.len() > 3 {
        println!("Wrong length {command_string} response.");
        return Err(());
    }
    let n: u8 = parse_number(command_string, args).map_err(|_| ())?;
    T::try_from(n).map_err(|_| ())
}

fn parse_number<T: FromStr>(command_string: &String, args: &[u8]) -> Result<T, ()> {
    if args.is_empty() {
        println!("Too short {command_string} response.");
        return Err(());
    }
    ascii_bytes_to_string(args).parse::<T>().map_err(|_| ())
}

fn process_line(mut s: Vec<u8>) {
    s.retain_mut(|c| c != &b'\n' && c != &b'\r');

    if s.is_empty() {
        return;
    }

    if s.len() < 5 {
        println!("Line is too short (s='{:?}')", s);
        return;
    }

    let command = &s[..5];
    let command_string = ascii_bytes_to_string(command);
    let args = &s[6..];

    /*
    println!(
        "nread: '{}' '{}'", command,
        ascii_bytes_to_string(args));
    );
    */

    match command {
        b"{CCM}" => {
            // Current Mode {CCM} Text 1 S=Sig, W=WSPR, N=None
            let mode: Mode = parse_enum(&command_string, args).unwrap();
            println!("{} current_mode={:?}", command_string, mode);
        }
        b"{CCR}" => {
            // Command CurrentReference [CCR] G Text 1 E=External, I=Internal
            let reference: Reference = parse_enum(&command_string, args).unwrap();
            println!("{} current_mode={:?}", command_string, reference);
        }
        b"{OTP}" => {
            // Option TX Pause {OTP} Text 5 0-99999 Minutes
            let minutes: u32 = parse_number(&command_string, args).unwrap();
            println!("{} tx_pause={} [m]", command_string, minutes);
        }
        b"{OSM}" => {
            // Option StartMode {OSM} Text 1 S=Sig, W=WSPR, N=None
            let mode: Mode = parse_enum(&command_string, args).unwrap();
            println!("{} start_mode={:?}", command_string, mode);
        }
        b"{OBD}" => {
            // Option Band TX Enable {OBD} Text 2 Text 1. Band number *, E=Enable, D=Disable
            if args.len() != 4 {
                panic!("OBD length wrong: {:?}", args);
            }
            let band_arg = &args[0..2];
            let enabled_arg = &args[3];
            let band: Band = parse_enum_from_number(&command_string, band_arg)
                .unwrap_or_else(|_| panic!("Failed to parse band from args: {:?}", args));
            let enabled = match enabled_arg {
                b'E' => true,
                b'D' => false,
                _ => {
                    panic!("Bad args for OBD {:?}", args);
                }
            };
            println!("{} band_tx_enable={:?} {}", command_string, band, enabled);
        }
        b"{OLC}" => {
            // Option Location {OLC} Text 1. G=GPS calculated, M=Manual (DL4 data)
            let location_source: LocationSource = parse_enum(&command_string, args).unwrap();
            println!("{} location_source={:?}", command_string, location_source);
        }
        b"{OLP}" => {
            // Option Locator Precision [OLP] S/G Text 1. 4 or 6 = Number of
            // character used in the Maidenhead report.
            let locator_precision: LocationPrecision = parse_enum(&command_string, args).unwrap();
            println!(
                "{} locator_precision={:?}",
                command_string, locator_precision
            );
        }
        b"{OPW}" => {
            let power_encoding: PowerEncoding = parse_enum(&command_string, args).unwrap();
            println!("{} power_encoding={:?}", command_string, power_encoding);
        }
        b"{OTS}" => {
            // Option Time Slot [OTS] S/G Text 2 Time Slot Code 0 to
            // 16. 0-4=10 min. schedule , 5-14=20min schedule, 15=Band
            // coordinated Schedule, 16=No schedule, 17=Tracker (only TX when
            // on the move or at top of hour)
            let number: u16 = parse_number(&command_string, args).unwrap();
            let time_slot = match number {
                0..=4 => TimeSlot::TenMinute,
                5..=14 => TimeSlot::TwentyMinute,
                15 => TimeSlot::BandCoordinated,
                16 => TimeSlot::NoSchedule,
                17 => TimeSlot::Tracker,
                _ => {
                    panic!("Bad time slot {:?}", args);
                }
            };
            println!("{} time_slot={:?}", command_string, time_slot);
        }
        b"{OPS}" => {
            // Option PreFix/Suffix [OPS] S/G Test1 P=Use Prefix. S=Use Suffix
            // N=None
            let prefix_suffix: PrefixSuffix = parse_enum(&command_string, args).unwrap();
            println!("{} prefix_suffix={:?}", command_string, prefix_suffix);
        }
        b"{OSC}" => {
            // Option set GPS Constellations {OSC} Text 1. G=GPS Only
            // B=BeiDou Only, A= GPS And BeiDou
            let constellation: Constellation = parse_enum(&command_string, args).unwrap();
            println!("{} gps_constellation={:?}", command_string, constellation);
        }
        b"{DCS}" => {
            // Data CallSign {DCS} Text 6
            let call_sign = ascii_bytes_to_string(args);
            println!("{} call_sing={}", command_string, call_sign);
        }
        b"{DSF}" => {
            // Data Suffix [DSF] S/G Text 3 Suffix code 000-125. 000-009= 0 to
            // 9. 010-035=A to Z suffix.  Call Sign suffix code. A / will be
            // automatically appended after the Call Sign followed by the
            let data_suffix = ascii_bytes_to_string(args);
            println!("{} data_suffix={}", command_string, data_suffix);
        }
        b"{DPF}" => {
            // Data Prefix [DPF] S/G Text 3 Prefix padded with leading spaces
            // if less than three characters. A-Z and 0-9 allowed Call Sign
            // prefix chars. A / will be automatically added between the
            // Prefix and the Call Sign
            let data_prefix = ascii_bytes_to_string(args);
            println!("{} data_prefix={}", command_string, data_prefix);
        }
        b"{DL4}" => {
            // Data Locator 4 {DL4} Text 4
            let locator_4 = ascii_bytes_to_string(args);
            println!("{} data_locator_4={}", command_string, locator_4);
        }
        b"{DL6}" => {
            // Data Locator 6 {DL6} Text 6
            let locator_6 = ascii_bytes_to_string(args);
            println!("{} data_locator_6={}", command_string, locator_6);
        }
        b"{DPD}" => {
            // Data PowerData {DPD} Text 2 (00 to 60) dBm
            let dbm: u8 = parse_number(&command_string, args).unwrap();
            println!("{} power_data={} dBm", command_string, dbm);
        }
        b"{DNM}" => {
            // Data Name {DNM} Text 40
            let name = ascii_bytes_to_string(args);
            println!("{} name={}", command_string, name);
        }
        b"{DGF}" => {
            // Data Generator Freq {DGF} Text 12 Frequency in
            // CentiHertz. Padded with leading zeros to 12 characters
            let centihertz: u32 = parse_number(&command_string, args).unwrap();
            let hertz: f32 = centihertz as f32 / 100.;
            println!("{} generator_freq={} Hz", command_string, hertz);
        }
        b"{DER}" => {
            // Data External Reference Frequency [DER] S/G Text 9 Frequency in
            // Hertz. Padded with leading zeros to 9 characters Normally
            // 010000000
            let hertz: u32 = parse_number(&command_string, args).unwrap();
            println!("{} generator_freq={} Hz", command_string, hertz);
        }
        b"{FPN}" => {
            // Factory Product model Number [FPN] G Text 5 0-65534
            // 1011=WSPR-TX_LP1, 1012=WSPR Desktop, 1017=WSPR Mini
            let model: u16 = parse_number(&command_string, args).unwrap();
            println!("{} model={}", command_string, model);
        }
        b"{FHV}" => {
            // Factory Hardware Version [FHV] S/G Text 3 0-255
            let hardware_version = ascii_bytes_to_string(args);
            println!("{} hardware_version={}", command_string, hardware_version);
        }
        b"{FHR}" => {
            // Factory Hardware Revision [FHR] S/G Text 3 0-255
            let hardware_revision = ascii_bytes_to_string(args);
            println!("{} hardware_revision={}", command_string, hardware_revision);
        }
        b"{FSV}" => {
            // Factory Software Version [FSV] G Text 3 0-255
            let software_version = ascii_bytes_to_string(args);
            println!("{} software_version={}", command_string, software_version);
        }
        b"{FSR}" => {
            // Factory Software Revision [FSR] G Text 3 0-255
            let software_revision = ascii_bytes_to_string(args);
            println!("{} software_revision={}", command_string, software_revision);
        }
        b"{FRF}" => {
            // Factory Reference Oscillator Frequency [FRF] S/G Text 9
            // Frequency in Hertz. Padded with leading zeros to 9 characters
            // Normally 026000000
            let hertz: u32 = parse_number(&command_string, args).unwrap();
            println!(
                "{} factory_reference_oscillator_frequency={} Hz",
                command_string, hertz
            );
        }
        b"{FLP}" => {
            // Factory Low Pass Filter installed [FLP] S/G Text 1 A,B,C or D
            // for indicating or setting bank of low pass filter A to D.  Text
            // 2 00 to 15 for band. 98=just a link between input and output -
            // the firmware will use this if no other filter is a good match,
            // 99=Nothing fitted (open circuit) the firmware will never use
            // this as a filter TODO(ch): fix this
            if args.len() != 4 {
                panic!("FLP length wrong: {:?}", args);
            }
            let bank_arg = &args[0..1];
            let band_arg = &args[2..];
            let filter_bank: FilterBank = parse_enum(&command_string, bank_arg).unwrap();
            let band: Band = parse_enum_from_number(&command_string, band_arg)
                .unwrap_or_else(|_| panic!("Failed to parse band from args: {:?}", args));
            println!(
                "{} factory_low_pass_filter={:?} {:?}",
                command_string, filter_bank, band
            );
        }
        b"{GL4}" => {
            // GPS locator 4 char Maidenhead {GL4} Text 4
            let maidenhead_4 = ascii_bytes_to_string(args);
            println!("{} maidenhead_4={}", command_string, maidenhead_4);
        }
        b"{GL6}" => {
            // GPS Locator 6 char Maidenhead {GL6} Test 6
            let maidenhead_6 = ascii_bytes_to_string(args);
            println!("{} maidenhead_6={}", command_string, maidenhead_6);
        }
        b"{GTM}" => {
            // GPS Time {GTM} Text 8 HH:MM:SS
            let _time = ascii_bytes_to_string(args);
            //println!("{} time={}", command_string, time);
        }
        b"{GLC}" => {
            // GPS Lock {GLC} Text 1 T=True F=False
            let gps_lock: GpsLock = parse_enum(&command_string, args).unwrap();
            println!("{} gps_lock={:?}", command_string, gps_lock);
        }
        b"{GSI}" => {
            // GPS Satellite data {GSI} Text2 Text3 Text2 Text2 - ID Az El SNR
            let _gps_sat_data = ascii_bytes_to_string(args);
            //println!("{} gps_sat_data={}", command_string, gps_sat_data);
        }
        b"{TFQ}" => {
            // Transmitter Frequency {TFQ} Text 5-12 Frequency in centiHz, no
            // leading zeros
            println!("transmitter_freq={:?}", args);
        }
        b"{TON}" => {
            // Transmitter On {TON} Text 1 T=True F=False
            if args.len() != 1 {
                println!("Wrong length TON response.");
                return;
            }
            let first_byte = args[0];
            let on = match first_byte {
                b'T' => true,
                b'F' => false,
                _ => {
                    panic!("bad char {}", first_byte);
                }
            };
            println!("{} transmitter_on={:?}", command_string, on);
        }
        b"{MPS}" => {
            // Microcontroller Pause {MPS} Text 7 0-4,000,000Seconds
            println!("{} microcontroller_pause={:?}", command_string, args);
        }
        b"{MIN}" => {
            // Microcontroller Information {MIN} Text
            println!(
                "{} mirocontroller_info='{}'",
                command_string,
                ascii_bytes_to_string(args)
            );
        }
        b"{LPI}" => {
            // Low Pass filter set {LPI} Text 1 A-D
            let filter_bank: FilterBank = parse_enum(&command_string, args).unwrap();
            println!("{} filter_bank_set={:?}", command_string, filter_bank);
        }
        b"{MVC}" => {
            // MicroController VCC Voltage {MVC} Text 4 0-9999mV (Normally
            // 3300)
            let millivolts: u32 = ascii_bytes_to_string(args).parse().unwrap();
            let voltage: f32 = millivolts as f32 / 1000.;
            println!("{} microcontroller_vcc={} [V]", command_string, voltage);
        }
        b"{TBN}" => {
            // Transmitter Current Band {TBN} Text 2=Band number *
            let band: Band = parse_enum_from_number(&command_string, args)
                .unwrap_or_else(|_| panic!("Failed to parse band from args: {:?}", args));
            println!("{} transmitter_current_band={:?}", command_string, band);
        }
        b"{TWS}" => {
            // Transmitter WSPR Symbol {TWS} Text 2 Text3 Band number *, WSPR
            // symbol count 0-161
            println!("{} transmitter_wspr_symbol={:?}", command_string, args);
        }
        b"{TCC}" => {
            // Transmitter WSPR Band Cycle Complete {TCC}
            println!(
                "{} transmitter_wspr_cycle_complete={:?}",
                command_string, args
            );
        }
        _ => {
            println!("unknown response {:?} '{}'", command, command_string);
        }
    }
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
    }
}

#[allow(dead_code)]
fn reset_device(port: &mut Box<dyn SerialPort>) {
    // To reset the device:
    //   Set RTS to HIGH
    //   Wait a while (100ms)
    //   Set RTS to LOW
    port.write_request_to_send(true).expect("Failed to set RTS");
    std::thread::sleep(Duration::from_millis(100));
    port.write_request_to_send(false)
        .expect("Failed to set RTS");
}

fn set_run(port: &mut Box<dyn SerialPort>) {
    // To set device to run:
    //   Set DTR LOW
    //   Wait a while (100ms)
    port.write_data_terminal_ready(false)
        .expect("Failed to set DTR");
    std::thread::sleep(Duration::from_millis(100));
    port.write_request_to_send(false)
        .expect("Failed to set RTS");
    std::thread::sleep(Duration::from_millis(100));
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
    let mut first_line = true;
    loop {
        let mut buf = vec![];
        match reader.read_until(b'\n', &mut buf) {
            Ok(_) => {
                if !first_line {
                    process_line(buf);
                }
                first_line = false;
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
