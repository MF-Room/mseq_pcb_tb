mod config;
mod midi;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use midir::{MidiInput, MidiOutput};
use rand::{SeedableRng, rngs::SmallRng};
use utils::{find_port, list_ports};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the host MIDI ports as "index: name"
    List {
        /// Only the input ports
        #[arg(long)]
        input: bool,
        /// Only the output ports
        #[arg(long)]
        output: bool,
    },
    /// Send MIDI messages and print the CRC32 of sent bytes
    Send {
        /// Number of messages to send
        #[arg(long)]
        count: u32,
        /// MIDI output port, by name or index (see `list`)
        #[arg(long)]
        port: String,
    },
    /// Receive MIDI messages and print the CRC32 of the received channel message bytes
    /// (System Real-Time, System Common and SysEx are ignored, as on the firmware)
    Receive {
        /// MIDI input port, by name or index (see `list`)
        #[arg(long)]
        port: String,
    },
}

fn load_config() -> Result<Config> {
    Ok(toml::from_str(&std::fs::read_to_string("config.toml")?)?)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let (crc, bytes) = match cli.command {
        Command::List { input, output } => {
            let both = input == output;
            if output || both {
                list_ports(&MidiOutput::new("host output")?, "Output")?;
            }
            if input || both {
                list_ports(&MidiInput::new("host input")?, "Input")?;
            }
            return Ok(());
        }
        Command::Send { count, port } => {
            let cfg = load_config()?;
            let midi_out = MidiOutput::new("host output")?;
            let out_port = find_port(&midi_out, &port)?;
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
            let cfg = load_config()?;
            let midi_in = MidiInput::new("host input")?;
            let in_port = find_port(&midi_in, &port)?;
            midi::receive_messages(midi_in, &in_port, cfg.watchdog_ms)?
        }
    };

    println!("CRC32: {crc:#010X} ({bytes} bytes)");
    Ok(())
}
