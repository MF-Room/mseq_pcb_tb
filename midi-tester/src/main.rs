mod config;
mod midi;
mod utils;
use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use midir::{MidiInput, MidiOutput};
use rand::{SeedableRng, rngs::SmallRng};
use utils::{port_by_index, select_port};

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
            if let Some(p) = port {
                cfg.midi_port = Some(p);
            }
            let midi_out = MidiOutput::new("host output")?;
            let out_port = match cfg.midi_port {
                Some(idx) => port_by_index(&midi_out, idx)?,
                None => select_port(&midi_out, "output")?,
            };
            let mut conn_out = midi_out
                .connect(&out_port, "output connection")
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            midi::send_messages(
                &mut SmallRng::seed_from_u64(cfg.seed),
                &mut conn_out,
                count,
                cfg.interval_ms,
            )
        }
        Command::Receive { port } => {
            if let Some(p) = port {
                cfg.midi_port = Some(p);
            }
            let midi_in = MidiInput::new("host input")?;
            let in_port = match cfg.midi_port {
                Some(idx) => port_by_index(&midi_in, idx)?,
                None => select_port(&midi_in, "input")?,
            };
            midi::receive_messages(midi_in, &in_port, cfg.watchdog_ms)?
        }
    };

    println!("CRC32: {:#010X}", crc);
    Ok(())
}
