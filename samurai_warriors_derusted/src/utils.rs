use bevy::prelude::ResMut;
use bevy_rand::prelude::GlobalEntropy;
use bevy_prng::ChaCha8Rng;
use rand_core::RngCore;

pub fn rand_range(min: u32, range: u32, rng: &mut ResMut<GlobalEntropy<ChaCha8Rng>>) -> u32 {
    rng.next_u32() % range + min
}