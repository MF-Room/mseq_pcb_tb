use anyhow::Result;
use crc::{CRC_32_ISO_HDLC, Crc};
use midir::{MidiInput, MidiInputPort, MidiOutputConnection};
use rand::Rng;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

pub fn send_messages(
    rng: &mut impl Rng,
    connection: &mut MidiOutputConnection,
    message_count: u32,
    interval_ms: u64,
) -> u32 {
    let mut digest = CRC32.digest();
    for _ in 0..message_count {
        let msg = crate::utils::random_midi_message(rng);
        digest.update(&msg);
        connection.send(&msg).ok();
        thread::sleep(Duration::from_millis(interval_ms));
    }
    digest.finalize()
}

pub fn receive_messages(
    midi_in: MidiInput,
    in_port: &MidiInputPort,
    watchdog_ms: Option<u64>,
) -> Result<u32> {
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
