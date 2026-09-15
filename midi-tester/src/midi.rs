use anyhow::Result;
use crc::{CRC_32_ISO_HDLC, Crc};
use midir::{MidiInput, MidiInputPort, MidiOutputConnection};
use rand::Rng;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

/// Sends `message_count` random channel messages `interval_ms` apart.
/// Returns the CRC32 of the sent bytes and their count.
pub fn send_messages(
    rng: &mut impl Rng,
    connection: &mut MidiOutputConnection,
    message_count: u32,
    interval_ms: u64,
) -> (u32, u32) {
    let mut digest = CRC32.digest();
    let mut byte_count: u32 = 0;
    for _ in 0..message_count {
        let msg = crate::utils::random_midi_message(rng);
        digest.update(&msg);
        byte_count += msg.len() as u32;
        connection.send(&msg).ok();
        thread::sleep(Duration::from_millis(interval_ms));
    }
    (digest.finalize(), byte_count)
}

/// Receives until `watchdog_ms` after connecting (forever when `None`).
/// Returns the CRC32 of the received channel message bytes and their count.
///
/// System Real-Time, System Common and SysEx bytes are dropped, the same filter as the
/// firmware's receive path. This matters on macOS: CoreMIDI sends a MIDI-CI Discovery
/// Inquiry SysEx out the adapter's MIDI OUT shortly after a client opens it, and in the
/// THRU test the board copies it back into the adapter's MIDI IN. The SysEx state is kept
/// across callbacks because CoreMIDI can split one SysEx over several of them.
pub fn receive_messages(
    midi_in: MidiInput,
    in_port: &MidiInputPort,
    watchdog_ms: Option<u64>,
) -> Result<(u32, u32)> {
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let _conn_in = midi_in
        .connect(
            in_port,
            "input connection",
            move |_ts, msg, _| {
                tx.send(msg.to_vec()).ok();
            },
            (),
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut digest = CRC32.digest();
    let mut byte_count: u32 = 0;
    let mut sysex_active = false;
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
        for &byte in &msg {
            if byte >= 0xF8 {
                // System Real-Time: ignore
            } else if byte == 0xF0 {
                sysex_active = true;
            } else if byte == 0xF7 {
                sysex_active = false;
            } else if byte >= 0xF1 {
                // Other System Common: ignore
            } else if !sysex_active {
                digest.update(&[byte]);
                byte_count += 1;
            }
        }
    }

    Ok((digest.finalize(), byte_count))
}
