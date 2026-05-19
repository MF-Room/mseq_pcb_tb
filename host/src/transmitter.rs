use crate::SEED;
use mseq::{Context, Instruction};
use rand::rngs::SmallRng;
use rand::{RngExt, SeedableRng};

#[derive(Clone)]
pub struct Conductor {
    rng: SmallRng,
}

impl Default for Conductor {
    fn default() -> Self {
        Self {
            rng: SmallRng::seed_from_u64(SEED),
        }
    }
}

impl mseq::Conductor for Conductor {
    fn init(&mut self, context: &mut Context) -> Vec<Instruction> {
        context.set_bpm(130);
        context.start();
        vec![]
    }

    fn update(&mut self, context: &mut Context) -> Vec<Instruction> {
        let step = context.get_step();
        if step == 383 {
            context.quit();
            return vec![];
        }

        let mut ins = vec![];

        let n = self.rng.random_range(0u8..=4);
        for _ in 0..n {
            ins.push(Instruction::SendCC {
                channel_id: self.rng.random_range(1u8..=16),
                parameter: self.rng.random_range(0u8..=127),
                value: self.rng.random_range(0u8..=127),
            });
        }

        ins
    }
}
