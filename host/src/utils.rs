use anyhow::{Result, anyhow};
use midir::MidiIO;
use rand::{Rng, RngExt};
use std::io::{Write, stdin, stdout};

pub fn select_port<T: MidiIO>(midi_io: &T, descr: &str) -> Result<T::Port> {
    println!("Available {} ports:", descr);
    let midi_ports = midi_io.ports();
    for (i, p) in midi_ports.iter().enumerate() {
        println!("{}: {}", i, midi_io.port_name(p)?);
    }
    print!("Please select {} port: ", descr);
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let port = midi_ports
        .get(input.trim().parse::<usize>()?)
        .ok_or_else(|| anyhow!("Invalid port number"))?;
    Ok(port.clone())
}

pub fn port_by_index<T: MidiIO>(midi_io: &T, index: usize) -> Result<T::Port> {
    let ports = midi_io.ports();
    ports
        .into_iter()
        .nth(index)
        .ok_or_else(|| anyhow!("MIDI port index {} not found", index))
}

pub fn random_midi_message(rng: &mut impl Rng) -> [u8; 3] {
    let channel: u8 = rng.random_range(0..=15);
    let note: u8 = rng.random_range(0..=127);
    let velocity: u8 = rng.random_range(1..=127);
    [0x90 | channel, note, velocity]
}
