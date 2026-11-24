use crate::player::Player;
use crate::sim::{Encounter, Sim, Tally};
use rayon::prelude::*;

#[derive(Clone, Debug)]
pub struct Outcome {
    pub dps: f64,
    pub stderr: f64,
    pub iterations: u64,
    pub tally: Tally,
}

const CHUNK: u64 = 250;

fn chunk_seed(seed: u64, chunk: u64) -> u64 {
    seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ chunk.wrapping_mul(0xD1B5_4A32_D192_ED03).rotate_left(17)
}

pub fn simulate(player: &Player, enc: &Encounter, iterations: u64, seed: u64) -> Outcome {
    let chunks = iterations.div_ceil(CHUNK).max(1);
    let parts: Vec<(f64, f64, u64, Tally)> = (0..chunks)
        .into_par_iter()
        .map(|c| {
            let n = CHUNK.min(iterations - c * CHUNK);
            let mut sim = Sim::new(player, chunk_seed(seed, c));
            let mut sum = 0.0;
            let mut sq = 0.0;
            for _ in 0..n {
                let dps = sim.run(enc);
                sum += dps;
                sq += dps * dps;
            }
            (sum, sq, n, sim.tally.clone())
        })
        .collect();
    let mut tally = Tally::new(player);
    let (mut sum, mut sq, mut n) = (0.0, 0.0, 0u64);
    for (s, q, k, t) in &parts {
        sum += s;
        sq += q;
        n += k;
        tally.merge(t);
    }
    let mean = sum / n as f64;
    let var = (sq / n as f64 - mean * mean).max(0.0);
    Outcome {
        dps: mean,
        stderr: (var / n as f64).sqrt(),
        iterations: n,
        tally,
    }
}
