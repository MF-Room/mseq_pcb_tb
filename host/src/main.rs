mod config;
mod utils;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use crc::{CRC_32_ISO_HDLC, Crc};
use midir::{MidiInput, MidiInputPort, MidiOutput, MidiOutputConnection};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use utils::{port_by_index, random_midi_message, select_port};

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Send MIDI messages and print the CRC32 of sent bytes
    Send {
        /// Number of messages to send
        #[arg(long)]
        count: u32,
        /// MIDI output port index (overrides config)
        #[arg(long)]
        port: Option<usize>,
    },
    /// Receive MIDI messages and print the CRC32 of received bytes
    Receive {
        /// MIDI input port index (overrides config)
        #[arg(long)]
        port: Option<usize>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut cfg: Config = toml::from_str(&std::fs::read_to_string("config.toml")?)?;

    let crc = match cli.command {
        Command::Send { count, port } => {
            if let Some(p) = port { cfg.midi_port = Some(p); }
            let midi_out = MidiOutput::new("host output")?;
            let out_port = match cfg.midi_port {
                Some(idx) => port_by_index(&midi_out, idx)?,
                None => select_port(&midi_out, "output")?,
            };
            let mut conn_out = midi_out.connect(&out_port, "output connection")?;
            send_messages(
                &mut SmallRng::seed_from_u64(cfg.seed),
                &mut conn_out,
                count,
                cfg.interval_ms,
            )
        }
        Command::Receive { port } => {
            if let Some(p) = port { cfg.midi_port = Some(p); }
            let midi_in = MidiInput::new("host input")?;
            let in_port = match cfg.midi_port {
                Some(idx) => port_by_index(&midi_in, idx)?,
                None => select_port(&midi_in, "input")?,
            };
            receive_messages(midi_in, &in_port, cfg.watchdog_ms)?
        }
    };

    println!("CRC32: {:#010X}", crc);
    Ok(())
}

fn send_messages(
    rng: &mut impl Rng,
    connection: &mut MidiOutputConnection,
    message_count: u32,
    interval_ms: u64,
) -> u32 {
    let mut digest = CRC32.digest();
    for _ in 0..message_count {
        let msg = random_midi_message(rng);
        digest.update(&msg);
        connection.send(&msg).ok();
        thread::sleep(Duration::from_millis(interval_ms));
    }
    digest.finalize()
}

fn receive_messages(
    midi_in: MidiInput,
    in_port: &MidiInputPort,
    watchdog_ms: Option<u64>,
) -> Result<u32> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let _conn_in = midi_in.connect(
        in_port,
        "input connection",
        move |_ts, msg, _| {
            tx.send(msg.to_vec()).ok();
        },
        (),
    )?;

    let mut digest = CRC32.digest();
    let deadline = watchdog_ms.map(|ms| Instant::now() + Duration::from_millis(ms));

    loop {
        let msg = match deadline {
            Some(dl) => {
                let remaining = dl.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match rx.recv_timeout(remaining) {
                    Ok(m) => m,
                    Err(_) => break,
                }
            }
            None => match rx.recv() {
                Ok(m) => m,
                Err(_) => break,
            },
        };
        digest.update(&msg);
    }

    Ok(digest.finalize())
}
