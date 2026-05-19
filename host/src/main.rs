mod transmitter;
use anyhow::Result;
use mseq::{MidiInParam, run};
use std::env;

const SEED: u64 = 42;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let port = if args.len() == 3 {
        (
            Some(args[1].parse::<u32>().unwrap()),
            Some(args[2].parse::<u32>().unwrap()),
        )
    } else {
        (None, None)
    };
    let params = MidiInParam {
        ignore: mseq::Ignore::None,
        port: port.1,
        slave: false,
    };
    let conductor = transmitter::Conductor::default();
    run(conductor, port.0, Some(params)).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}
