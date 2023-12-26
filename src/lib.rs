#![allow(dead_code)]

use ascii::AsciiStr;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serialport::SerialPort;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Mode {
    Sig = b'S',
    Wspr = b'W',
    Idle = b'N',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum FilterBank {
    A = b'A',
    B = b'B',
    C = b'C',
    D = b'D',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Reference {
    External = b'E',
    Internal = b'I',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum LocationSource {
    Gps = b'G',
    Manual = b'M',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum LocatorPrecision {
    Maidenhead4 = b'4',
    Maidenhead6 = b'6',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum PowerEncoding {
    Normal = b'N',
    Altitude = b'A',
}

#[derive(Debug, Clone, Copy)]
pub enum TimeSlot {
    TenMinute,
    TwentyMinute,
    BandCoordinated,
    NoSchedule,
    Tracker,
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum PrefixSuffix {
    Prefix = b'P',
    Suffix = b'S',
    None = b'N',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Constellation {
    GPSOnly = b'G',
    BeiDouOnly = b'B',
    All = b'A',
}

#[derive(Debug, Clone, Copy, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Band {
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
pub enum GpsLock {
    Locked = b'T',
    Unlocked = b'F',
}

////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct CurrentModeCommand {
    pub mode: Mode,
}

impl CurrentModeCommand {
    // Current Mode {CCM} Text 1 S=Sig, W=WSPR, N=None
    pub const CODE: &'static [u8] = b"CCM";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::CurrentModeCommand(CurrentModeCommand {
            mode: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CurrentReferenceCommand {
    pub reference: Reference,
}

impl CurrentReferenceCommand {
    // Command CurrentReference [CCR] G Text 1 E=External, I=Internal
    pub const CODE: &'static [u8] = b"CCR";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::CurrentReferenceCommand(CurrentReferenceCommand {
            reference: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TxPauseOption {
    pub duration: Duration,
}

impl TxPauseOption {
    // Option TX Pause {OTP} Text 5 0-99999 Minutes
    pub const CODE: &'static [u8] = b"OTP";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        let minutes: u32 = parse_number(command_string, args).unwrap();
        let seconds: u64 = 60 * minutes as u64;
        Response::TxPauseOption(TxPauseOption {
            duration: Duration::from_secs(seconds),
        })
    }
}

#[derive(Debug, Clone)]
pub struct StartModeOption {
    pub mode: Mode,
}

impl StartModeOption {
    // Option StartMode {OSM} Text 1 S=Sig, W=WSPR, N=None
    pub const CODE: &'static [u8] = b"OSM";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::StartModeOption(StartModeOption {
            mode: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct BandTxEnable {
    pub band: Band,
    pub enabled: bool,
}

impl BandTxEnable {
    // Option Band TX Enable {OBD} Text 2 Text 1. Band number *, E=Enable, D=Disable
    pub const CODE: &'static [u8] = b"OBD";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        if args.len() != 4 {
            panic!("OBD length wrong: {:?}", args);
        }
        let band_arg = &args[0..2];
        let enabled_arg = &args[3];
        let band: Band = parse_enum_from_number(command_string, band_arg)
            .unwrap_or_else(|_| panic!("Failed to parse band from args: {:?}", args));
        let enabled = match enabled_arg {
            b'E' => true,
            b'D' => false,
            _ => {
                panic!("Bad args for OBD {:?}", args);
            }
        };
        Response::BandTxEnable(BandTxEnable { band, enabled })
    }
}

#[derive(Debug, Clone)]
pub struct LocationSourceOption {
    pub location_source: LocationSource,
}

impl LocationSourceOption {
    // Option Location {OLC} Text 1. G=GPS calculated, M=Manual (DL4 data)
    pub const CODE: &'static [u8] = b"OLC";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::LocationSourceOption(LocationSourceOption {
            location_source: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct LocatorPrecisionOption {
    pub locator_precision: LocatorPrecision,
}

impl LocatorPrecisionOption {
    // Option Locator Precision [OLP] S/G Text 1. 4 or 6 = Number of
    // character used in the Maidenhead report.
    pub const CODE: &'static [u8] = b"OLP";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::LocatorPrecisionOption(LocatorPrecisionOption {
            locator_precision: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PowerEncodingOption {
    pub power_encoding: PowerEncoding,
}

impl PowerEncodingOption {
    // Option ....
    pub const CODE: &'static [u8] = b"OPW";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::PowerEncodingOption(PowerEncodingOption {
            power_encoding: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TimeSlotOption {
    pub time_slot: TimeSlot,
}

impl TimeSlotOption {
    // Option Time Slot [OTS] S/G Text 2 Time Slot Code 0/ 1;36. 0*-4=10 min. schedule , 5-14=20min schedule, 15=Band
    // coordinated Schedule, 16=No schedule, 17=Tracker (only TX when
    // on the move or at top of hour)
    pub const CODE: &'static [u8] = b"OTS";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        let number: u16 = parse_number(command_string, args).unwrap();
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
        Response::TimeSlotOption(TimeSlotOption { time_slot })
    }
}

#[derive(Debug, Clone)]
pub struct PrefixSuffixOption {
    pub prefix_suffix: PrefixSuffix,
}

impl PrefixSuffixOption {
    // Option PreFix/Suffix [OPS] S/G Test1 P=Use Prefix. S=Use Suffix
    // N=None
    pub const CODE: &'static [u8] = b"OPS";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::PrefixSuffixOption(PrefixSuffixOption {
            prefix_suffix: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ConstellationOption {
    pub constellation: Constellation,
}

impl ConstellationOption {
    // Option set GPS Constellations {OSC} Text 1. G=GPS Only
    // B=BeiDou Only, A= GPS And BeiDou
    pub const CODE: &'static [u8] = b"OSC";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::ConstellationOption(ConstellationOption {
            constellation: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct CallSignData {
    pub call_sign: String,
}

impl CallSignData {
    // Data CallSign {DCS} Text 6
    pub const CODE: &'static [u8] = b"DCS";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        // Data CallSign {DCS} Text 6
        Response::CallSignData(CallSignData {
            call_sign: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SuffixData {
    pub data_suffix: String,
}

impl SuffixData {
    // Data Suffix [DSF] S/G Text 3 Suffix code 000-;3009=* 0 to
    // 9. 010-035=A to Z suffix.  Call Sign suffix code. be
    // automatically appended after the Call Sign followed by the
    pub const CODE: &'static [u8] = b"DSF";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::SuffixData(SuffixData {
            data_suffix: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PrefixData {
    pub data_prefix: String,
}

impl PrefixData {
    // Data Prefix [DPF] S/G Text 3 Prefix padded with leading spaces
    // if less than three characters. A-Z and 0-9 allowed Call Sign
    // prefix chars. A / will be automatically added between the
    // Prefix and the Call Sign
    pub const CODE: &'static [u8] = b"DPF";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::PrefixData(PrefixData {
            data_prefix: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Locator4Data {
    pub locator_4: String,
}

impl Locator4Data {
    // Data Locator 4 {DL4} Text 4
    pub const CODE: &'static [u8] = b"DL4";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::Locator4Data(Locator4Data {
            locator_4: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Locator6Data {
    pub locator_6: String,
}

impl Locator6Data {
    // Data Locator 6 {DL6} Text 6
    pub const CODE: &'static [u8] = b"DL6";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::Locator6Data(Locator6Data {
            locator_6: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PowerData {
    pub dbm: u8,
}

impl PowerData {
    // Data PowerData {DPD} Text 2 (00 to 60) dBm
    pub const CODE: &'static [u8] = b"DPD";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::PowerData(PowerData {
            dbm: parse_number(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct NameData {
    pub name: String,
}

impl NameData {
    // Data Name {DNM} Text 40
    pub const CODE: &'static [u8] = b"DNM";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::NameData(NameData {
            name: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct GeneratorFrequencyData {
    pub hertz: f32,
}

impl GeneratorFrequencyData {
    // Data Generator Freq {DGF} Text 12 Frequency in
    // CentiHertz. Padded with leading zeros to 12 characters
    pub const CODE: &'static [u8] = b"DGF";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        let centihertz: u32 = parse_number(command_string, args).unwrap();
        let hertz: f32 = centihertz as f32 / 100.;
        Response::GeneratorFrequencyData(GeneratorFrequencyData { hertz })
    }
}

#[derive(Debug, Clone)]
pub struct ExternalReferenceFrequencyData {
    pub hertz: u32,
}

impl ExternalReferenceFrequencyData {
    // Data External Reference Frequency [DER] S/G Text 9 Frequency in
    // Hertz. Padded with leading zeros to 9 characters Normally
    // 010000000
    pub const CODE: &'static [u8] = b"DER";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::ExternalReferenceFrequencyData(ExternalReferenceFrequencyData {
            hertz: parse_number(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ProductModelNumberFactory {
    pub model: u16, // TODO(ch): switch to enum
}

impl ProductModelNumberFactory {
    // Factory Product model Number [FPN] G Text 5 0-65534
    // 1011=WSPR-TX_LP1, 1012=WSPR Desktop, 1017=WSPR Mini
    pub const CODE: &'static [u8] = b"FPN";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::ProductModelNumberFactory(ProductModelNumberFactory {
            model: parse_number(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct HardwareVersionFactory {
    pub hardware_version: String,
}

impl HardwareVersionFactory {
    // Factory Hardware Version [FHV] S/G Text 3 0-255
    pub const CODE: &'static [u8] = b"FHV";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::HardwareVersionFactory(HardwareVersionFactory {
            hardware_version: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct HardwareRevisionFactory {
    pub hardware_version: String,
}

impl HardwareRevisionFactory {
    // Factory Hardware Revision [FHR] S/G Text 3 0-255
    pub const CODE: &'static [u8] = b"FHR";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::HardwareRevisionFactory(HardwareRevisionFactory {
            hardware_version: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SoftwareVersionFactory {
    pub software_version: String,
}

impl SoftwareVersionFactory {
    // Factory Software Version [FSV] G Text 3 0-255
    pub const CODE: &'static [u8] = b"FSV";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::SoftwareVersionFactory(SoftwareVersionFactory {
            software_version: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SoftwareRevisionFactory {
    pub software_revision: String,
}

impl SoftwareRevisionFactory {
    // Factory Software Revision [FSR] G Text 3 0-255
    pub const CODE: &'static [u8] = b"FSR";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::SoftwareRevisionFactory(SoftwareRevisionFactory {
            software_revision: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReferenceOscillatorFrequencyFactory {
    pub hertz: u32,
}

impl ReferenceOscillatorFrequencyFactory {
    // Factory Reference Oscillator Frequency [FRF] S/G Text 9
    // Frequency in Hertz. Padded with leading zeros to 9 characters
    // Normally 026000000
    pub const CODE: &'static [u8] = b"FRF";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::ReferenceOscillatorFrequencyFactory(ReferenceOscillatorFrequencyFactory {
            hertz: parse_number(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct LowPassFilterFactory {
    pub filter_bank: FilterBank,
    pub band: Band,
}

impl LowPassFilterFactory {
    // Factory Low Pass Filter installed [FLP] S/G Text 1 A,B,C or D
    // for indicating or setting bank of low pass filter A to D.  Text
    // 2 00 to 15 for band. 98=just a link between input and output -
    // the firmware will use this if no other filter is a good match,
    // 99=Nothing fitted (open circuit) the firmware will never use
    // this as a filter
    pub const CODE: &'static [u8] = b"FLP";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        // TODO(ch): fix this
        if args.len() != 4 {
            panic!("FLP length wrong: {:?}", args);
        }
        let bank_arg = &args[0..1];
        let band_arg = &args[2..];
        let filter_bank: FilterBank = parse_enum(command_string, bank_arg).unwrap();
        let band: Band = parse_enum_from_number(command_string, band_arg)
            .unwrap_or_else(|_| panic!("Failed to parse band from args: {:?}", args));
        Response::LowPassFilterFactory(LowPassFilterFactory { filter_bank, band })
    }
}

#[derive(Debug, Clone)]
pub struct Locator4GPS {
    pub maidenhead_4: String,
}

impl Locator4GPS {
    // GPS locator 4 char Maidenhead {GL4} Text 4
    pub const CODE: &'static [u8] = b"GL4";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::Locator4GPS(Locator4GPS {
            maidenhead_4: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Locator6GPS {
    pub maidenhead_6: String,
}

impl Locator6GPS {
    // GPS locator 6 char Maidenhead {GL6} Text 6
    pub const CODE: &'static [u8] = b"GL6";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::Locator6GPS(Locator6GPS {
            maidenhead_6: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TimeGPS {
    pub hhmmss: String,
}

impl TimeGPS {
    // GPS Time {GTM} Text 8 HH:MM:SS
    pub const CODE: &'static [u8] = b"GTM";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        // TODO(ch): parse this.
        Response::TimeGPS(TimeGPS {
            hhmmss: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct LockStatusGPS {
    pub lock: GpsLock,
}

impl LockStatusGPS {
    // GPS Lock {GLC} Text 1 T=True F=False
    pub const CODE: &'static [u8] = b"GLC";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        // TODO(ch): parse this.
        Response::LockStatusGPS(LockStatusGPS {
            lock: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SatelliteInfoGPS {
    pub satellite_info: String,
}

impl SatelliteInfoGPS {
    // GPS Satellite data {GSI} Text2 Text3 Text2 Text2 - ID Az El SNR
    pub const CODE: &'static [u8] = b"GSI";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        // TODO(ch): parse this.
        Response::SatelliteInfoGPS(SatelliteInfoGPS {
            satellite_info: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TransmitterFrequency {
    hertz: f32,
}

impl TransmitterFrequency {
    // Transmitter Frequency {TFQ} Text 5-12 Frequency in centiHz, no
    // leading zeros
    pub const CODE: &'static [u8] = b"TFQ";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        let centihertz: u64 = parse_number(command_string, args).unwrap();
        let hertz = centihertz as f32 / 100.;
        Response::TransmitterFrequency(TransmitterFrequency { hertz })
    }
}

#[derive(Debug, Clone)]
pub struct TransmitterStatus {
    pub on: bool,
}

impl TransmitterStatus {
    // Transmitter On {TON} Text 1 T=True F=False
    pub const CODE: &'static [u8] = b"TON";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        if args.len() != 1 {
            panic!("Wrong length TON response.");
        }
        let first_byte = args[0];
        let on = match first_byte {
            b'T' => true,
            b'F' => false,
            _ => {
                panic!("bad char {}", first_byte);
            }
        };
        Response::TransmitterStatus(TransmitterStatus { on })
    }
}

#[derive(Debug, Clone)]
pub struct MicrocontrollerPause {}

impl MicrocontrollerPause {
    // Microcontroller Pause {MPS} Text 7 0-4,000,000Seconds
    pub const CODE: &'static [u8] = b"MPS";

    fn parse(_command_string: &str, _args: &[u8]) -> Response {
        // TODO(ch): implement
        Response::MicrocontrollerPause(MicrocontrollerPause {})
    }
}

#[derive(Debug, Clone)]
pub struct MicrocontrollerInfo {
    pub info: String,
}

impl MicrocontrollerInfo {
    // Microcontroller Information {MIN} Text
    pub const CODE: &'static [u8] = b"MIN";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        Response::MicrocontrollerInfo(MicrocontrollerInfo {
            info: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct LowPassFilterSet {
    pub filter_bank: FilterBank,
}

impl LowPassFilterSet {
    // Low Pass filter set {LPI} Text 1 A-D
    pub const CODE: &'static [u8] = b"LPI";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        Response::LowPassFilterSet(LowPassFilterSet {
            filter_bank: parse_enum(command_string, args).unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct MicrocontrollerVoltage {
    pub voltage: f32,
}

impl MicrocontrollerVoltage {
    // MicroController VCC Voltage {MVC} Text 4 0-9999mV (Normally
    // 3300)
    pub const CODE: &'static [u8] = b"MVC";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        let millivolts: u32 = ascii_bytes_to_string(args).parse().unwrap();
        let voltage: f32 = millivolts as f32 / 1000.;
        Response::MicrocontrollerVoltage(MicrocontrollerVoltage { voltage })
    }
}

#[derive(Debug, Clone)]
pub struct TransmitterCurrentBand {
    pub band: Band,
}

impl TransmitterCurrentBand {
    // Transmitter Current Band {TBN} Text 2=Band number *
    pub const CODE: &'static [u8] = b"TBN";

    fn parse(command_string: &str, args: &[u8]) -> Response {
        let band: Band = parse_enum_from_number(command_string, args)
            .unwrap_or_else(|_| panic!("Failed to parse band from args: {:?}", args));
        Response::TransmitterCurrentBand(TransmitterCurrentBand { band })
    }
}

#[derive(Debug, Clone)]
pub struct TransmitterWSPRSymbol {
    pub something: String,
}

impl TransmitterWSPRSymbol {
    // Transmitter WSPR Symbol {TWS} Text 2 Text3 Band number *, WSPR
    // symbol count 0-161
    pub const CODE: &'static [u8] = b"TWS";

    fn parse(_command_string: &str, args: &[u8]) -> Response {
        // TODO(ch): figure this out
        Response::TransmitterWSPRSymbol(TransmitterWSPRSymbol {
            something: ascii_bytes_to_string(args),
        })
    }
}

#[derive(Debug, Clone)]
pub struct TransmitterBandCycleComplete {}

impl TransmitterBandCycleComplete {
    // Transmitter WSPR Band Cycle Complete {TCC}
    pub const CODE: &'static [u8] = b"TCC";

    fn parse(_command_string: &str, _args: &[u8]) -> Response {
        Response::TransmitterBandCycleComplete(TransmitterBandCycleComplete {})
    }
}

#[derive(Debug, Clone)]
pub enum Response {
    CurrentModeCommand(CurrentModeCommand),
    CurrentReferenceCommand(CurrentReferenceCommand),
    TxPauseOption(TxPauseOption),
    StartModeOption(StartModeOption),
    BandTxEnable(BandTxEnable),
    LocationSourceOption(LocationSourceOption),
    LocatorPrecisionOption(LocatorPrecisionOption),
    PowerEncodingOption(PowerEncodingOption),
    TimeSlotOption(TimeSlotOption),
    PrefixSuffixOption(PrefixSuffixOption),
    ConstellationOption(ConstellationOption),
    CallSignData(CallSignData),
    SuffixData(SuffixData),
    PrefixData(PrefixData),
    Locator4Data(Locator4Data),
    Locator6Data(Locator6Data),
    PowerData(PowerData),
    NameData(NameData),
    GeneratorFrequencyData(GeneratorFrequencyData),
    ExternalReferenceFrequencyData(ExternalReferenceFrequencyData),
    ProductModelNumberFactory(ProductModelNumberFactory),
    HardwareVersionFactory(HardwareVersionFactory),
    HardwareRevisionFactory(HardwareRevisionFactory),
    SoftwareVersionFactory(SoftwareVersionFactory),
    SoftwareRevisionFactory(SoftwareRevisionFactory),
    ReferenceOscillatorFrequencyFactory(ReferenceOscillatorFrequencyFactory),
    LowPassFilterFactory(LowPassFilterFactory),
    Locator4GPS(Locator4GPS),
    Locator6GPS(Locator6GPS),
    TimeGPS(TimeGPS),
    LockStatusGPS(LockStatusGPS),
    SatelliteInfoGPS(SatelliteInfoGPS),
    TransmitterFrequency(TransmitterFrequency),
    TransmitterStatus(TransmitterStatus),
    MicrocontrollerPause(MicrocontrollerPause),
    MicrocontrollerInfo(MicrocontrollerInfo),
    LowPassFilterSet(LowPassFilterSet),
    MicrocontrollerVoltage(MicrocontrollerVoltage),
    TransmitterCurrentBand(TransmitterCurrentBand),
    TransmitterWSPRSymbol(TransmitterWSPRSymbol),
    TransmitterBandCycleComplete(TransmitterBandCycleComplete),
}

fn ascii_bytes_to_string(bytes: &[u8]) -> String {
    AsciiStr::from_ascii(bytes)
        .unwrap_or_else(|_| panic!("could not interpret bytes: {:?}", bytes))
        .to_string()
}

fn parse_enum<T: TryFrom<u8>>(command_string: &str, args: &[u8]) -> Result<T, ()> {
    if args.len() != 1 {
        println!("Wrong length {command_string} response.");
        return Err(());
    }
    let first_byte = args[0];
    T::try_from(first_byte).map_err(|_| ())
}

fn parse_enum_from_number<T: TryFrom<u8>>(command_string: &str, args: &[u8]) -> Result<T, ()> {
    if args.is_empty() || args.len() > 3 {
        println!("Wrong length {command_string} response.");
        return Err(());
    }
    let n: u8 = parse_number(command_string, args).map_err(|_| ())?;
    T::try_from(n).map_err(|_| ())
}

fn parse_number<T: FromStr>(command_string: &str, args: &[u8]) -> Result<T, ()> {
    if args.is_empty() {
        println!("Too short {command_string} response.");
        return Err(());
    }
    ascii_bytes_to_string(args).parse::<T>().map_err(|_| ())
}

pub fn process_line(mut s: Vec<u8>) -> Option<Response> {
    s.retain_mut(|c| c != &b'\n' && c != &b'\r');

    if s.is_empty() {
        return None;
    }

    if s.len() < 5 {
        println!("Line is too short (s='{:?}')", s);
        return None;
    }

    let command = &s[..5];
    let code = &s[1..4];
    let command_string = ascii_bytes_to_string(command);
    let args = &s[6..];

    /*
    println!(
        "read: '{:?}' '{}'", command,
        ascii_bytes_to_string(args));
    */

    let response: Response = match code {
        CurrentModeCommand::CODE => CurrentModeCommand::parse(&command_string, args),
        CurrentReferenceCommand::CODE => CurrentReferenceCommand::parse(&command_string, args),
        TxPauseOption::CODE => TxPauseOption::parse(&command_string, args),
        StartModeOption::CODE => StartModeOption::parse(&command_string, args),
        BandTxEnable::CODE => BandTxEnable::parse(&command_string, args),
        LocationSourceOption::CODE => LocationSourceOption::parse(&command_string, args),
        LocatorPrecisionOption::CODE => LocatorPrecisionOption::parse(&command_string, args),
        PowerEncodingOption::CODE => PowerEncodingOption::parse(&command_string, args),
        TimeSlotOption::CODE => TimeSlotOption::parse(&command_string, args),
        PrefixSuffixOption::CODE => PrefixSuffixOption::parse(&command_string, args),
        ConstellationOption::CODE => ConstellationOption::parse(&command_string, args),
        CallSignData::CODE => CallSignData::parse(&command_string, args),
        SuffixData::CODE => SuffixData::parse(&command_string, args),
        PrefixData::CODE => PrefixData::parse(&command_string, args),
        Locator4Data::CODE => Locator4Data::parse(&command_string, args),
        Locator6Data::CODE => Locator6Data::parse(&command_string, args),
        PowerData::CODE => PowerData::parse(&command_string, args),
        NameData::CODE => NameData::parse(&command_string, args),
        GeneratorFrequencyData::CODE => GeneratorFrequencyData::parse(&command_string, args),
        ExternalReferenceFrequencyData::CODE => {
            ExternalReferenceFrequencyData::parse(&command_string, args)
        }
        ProductModelNumberFactory::CODE => ProductModelNumberFactory::parse(&command_string, args),
        HardwareVersionFactory::CODE => HardwareVersionFactory::parse(&command_string, args),
        HardwareRevisionFactory::CODE => HardwareRevisionFactory::parse(&command_string, args),
        SoftwareVersionFactory::CODE => SoftwareVersionFactory::parse(&command_string, args),
        ReferenceOscillatorFrequencyFactory::CODE => {
            ReferenceOscillatorFrequencyFactory::parse(&command_string, args)
        }
        LowPassFilterFactory::CODE => LowPassFilterFactory::parse(&command_string, args),
        Locator4GPS::CODE => Locator4GPS::parse(&command_string, args),
        Locator6GPS::CODE => Locator6GPS::parse(&command_string, args),
        TimeGPS::CODE => TimeGPS::parse(&command_string, args),
        LockStatusGPS::CODE => LockStatusGPS::parse(&command_string, args),
        SatelliteInfoGPS::CODE => SatelliteInfoGPS::parse(&command_string, args),
        TransmitterFrequency::CODE => TransmitterFrequency::parse(&command_string, args),
        TransmitterStatus::CODE => TransmitterStatus::parse(&command_string, args),
        MicrocontrollerPause::CODE => MicrocontrollerPause::parse(&command_string, args),
        MicrocontrollerInfo::CODE => MicrocontrollerInfo::parse(&command_string, args),
        LowPassFilterSet::CODE => LowPassFilterSet::parse(&command_string, args),
        MicrocontrollerVoltage::CODE => MicrocontrollerVoltage::parse(&command_string, args),
        TransmitterCurrentBand::CODE => TransmitterCurrentBand::parse(&command_string, args),
        TransmitterWSPRSymbol::CODE => TransmitterWSPRSymbol::parse(&command_string, args),
        TransmitterBandCycleComplete::CODE => {
            TransmitterBandCycleComplete::parse(&command_string, args)
        }
        _ => {
            panic!("unknown response {:?} '{}'", command, command_string);
        }
    };
    Some(response)
}

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

pub fn set_run(port: &mut Box<dyn SerialPort>) {
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
