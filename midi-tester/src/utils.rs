use anyhow::{Result, anyhow};
use midir::MidiIO;
use rand::{Rng, RngExt};

/// Prints the ports of `midi_io` as "index: name" under a "<descr> ports:" header.
pub fn list_ports<T: MidiIO>(midi_io: &T, descr: &str) -> Result<()> {
    println!("{descr} ports:");
    for (i, p) in midi_io.ports().iter().enumerate() {
        println!("{i}: {}", midi_io.port_name(p)?);
    }
    Ok(())
}

/// Finds a port by exact name, or by index when `wanted` is a number.
pub fn find_port<T: MidiIO>(midi_io: &T, wanted: &str) -> Result<T::Port> {
    let ports = midi_io.ports();
    if let Ok(index) = wanted.parse::<usize>() {
        return ports
            .into_iter()
            .nth(index)
            .ok_or_else(|| anyhow!("MIDI port index {index} not found, see `midi-tester list`"));
    }
    ports
        .into_iter()
        .find(|p| midi_io.port_name(p).is_ok_and(|name| name == wanted))
        .ok_or_else(|| anyhow!("MIDI port {wanted:?} not found, see `midi-tester list`"))
}

pub fn random_midi_message(rng: &mut impl Rng) -> [u8; 3] {
    let channel: u8 = rng.random_range(0..=15);
    let note: u8 = rng.random_range(0..=127);
    let velocity: u8 = rng.random_range(1..=127);
    [0x90 | channel, note, velocity]
}
