use candid::Principal;
use rand::{
    SeedableRng,
    distr::uniform::{UniformChar, UniformSampler},
    seq::IndexedRandom,
};
use rand_xoshiro::Xoshiro256StarStar;

use crate::data::{adjectives::ADJECTIVES, nouns::NOUNS};

mod data;

const RAND_DIGIT_COUNT: usize = 3;

pub fn random_username_from_principal(principal: Principal, max_len: usize) -> String {
    let mut seed = [0u8; 32];
    let princ_bytes = principal.as_slice();
    seed[0..princ_bytes.len()].copy_from_slice(princ_bytes);
    let mut rng = Xoshiro256StarStar::from_seed(seed);

    let noun = *NOUNS.choose(&mut rng).unwrap();
    let adjective = *ADJECTIVES.choose(&mut rng).unwrap();

    let mut base = String::new();
    base.push_str(noun);
    base.push_str(adjective);
    base.truncate(max_len - RAND_DIGIT_COUNT);

    let digit_dist = UniformChar::new_inclusive('0', '9').unwrap();

    for _ in 0..RAND_DIGIT_COUNT {
        base.push(digit_dist.sample(&mut rng));
    }
    base.shrink_to_fit();

    base
}

#[cfg(test)]
mod test {
    use super::random_username_from_principal;
    use candid::Principal;

    #[test]
    fn test_rng_len() {
        let princ = Principal::anonymous();
        let res = random_username_from_principal(princ, 15);
        println!("{res}");
        assert!(res.len() <= 15);
    }

    #[test]
    fn test_rng_reproducible() {
        let princ = Principal::anonymous();
        let res1 = random_username_from_principal(princ, 15);
        let res2 = random_username_from_principal(princ, 15);
        assert_eq!(res1, res2);
    }
}
