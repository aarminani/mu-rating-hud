use std::f64::consts::PI;

pub const SCALE: f64 = 173.7178;

pub const PRIOR_RD: f64 = 65.0;

pub const VOLATILITY_INFLATION: f64 = 0.06;

pub const GAP_STEP: i32 = 10;

pub const GAP_LIMIT: i32 = 600;

fn g(phi: f64) -> f64 {
    1.0 / (1.0 + 3.0 * phi * phi / (PI * PI)).sqrt()
}

pub fn change(phi_self: f64, phi_opp: f64, mu_self: f64, mu_opp: f64, s: f64) -> f64 {
    let gg = g(phi_opp);
    let e = 1.0 / (1.0 + (-gg * (mu_self - mu_opp)).exp());
    let v = 1.0 / (gg * gg * e * (1.0 - e));
    let phi_post_sq = 1.0 / (1.0 / (phi_self * phi_self) + 1.0 / v);
    SCALE * phi_post_sq * gg * (s - e)
}

pub fn est(gap: i32) -> (i32, i32) {
    let prior = PRIOR_RD / SCALE;
    let phi_self = (prior * prior + VOLATILITY_INFLATION * VOLATILITY_INFLATION).sqrt();
    let mu_opp = f64::from(gap) / SCALE;
    let round = |x: f64| (x + 0.5).floor() as i32;
    (
        round(change(phi_self, prior, 0.0, mu_opp, 1.0)),
        round(change(phi_self, prior, 0.0, mu_opp, 0.0)),
    )
}

pub fn est_with_rd(gap: i32, phi_self: f64, phi_opp: f64) -> (i32, i32) {
    let mu_opp = f64::from(gap) / SCALE;
    let round = |x: f64| (x + 0.5).floor() as i32;
    (
        round(change(phi_self, phi_opp, 0.0, mu_opp, 1.0)),
        round(change(phi_self, phi_opp, 0.0, mu_opp, 0.0)),
    )
}

pub fn phi_after(phi_self: f64, phi_opp: f64, mu_self: f64, mu_opp: f64) -> f64 {
    let gg = g(phi_opp);
    let e = 1.0 / (1.0 + (-gg * (mu_self - mu_opp)).exp());
    let v = 1.0 / (gg * gg * e * (1.0 - e));
    (1.0 / (1.0 / (phi_self * phi_self) + 1.0 / v)).sqrt()
}

pub fn propagate(phi: f64, phi_opp: f64, mu_self: f64, mu_opp: f64) -> f64 {
    let inflated = (phi * phi + VOLATILITY_INFLATION * VOLATILITY_INFLATION).sqrt();
    phi_after(inflated, phi_opp, mu_self, mu_opp)
}

pub fn mu_of(rating: i32) -> f64 {
    (f64::from(rating) - 1500.0) / SCALE
}

pub fn rd_of(phi: f64) -> f64 {
    phi * SCALE
}

pub fn phi_of(rd: f64) -> f64 {
    rd / SCALE
}

pub const RD_BUCKET: i32 = 1;

pub const RD_BUCKET_MIN: i32 = 54;
pub const RD_BUCKET_MAX: i32 = 84;

pub fn bucket_rd(rd: f64) -> i32 {
    let b = ((rd / f64::from(RD_BUCKET)).round() as i32) * RD_BUCKET;
    b.clamp(RD_BUCKET_MIN, RD_BUCKET_MAX)
}

pub fn buckets() -> impl Iterator<Item = i32> {
    (RD_BUCKET_MIN..=RD_BUCKET_MAX).step_by(RD_BUCKET as usize)
}

pub fn table(phi_self: f64, phi_opp: f64) -> Vec<(i32, (i32, i32))> {
    let inflated = (phi_self * phi_self + VOLATILITY_INFLATION * VOLATILITY_INFLATION).sqrt();
    (-GAP_LIMIT..=GAP_LIMIT)
        .step_by(GAP_STEP as usize)
        .map(|gap| (gap, est_with_rd(gap, inflated, phi_opp)))
        .collect()
}

pub fn solve_phi(target_change: f64, phi_opp: f64, mu_self: f64, mu_opp: f64, s: f64) -> f64 {
    let (mut lo, mut hi) = (0.02_f64, 3.2_f64);
    for _ in 0..45 {
        let mid = (lo + hi) / 2.0;
        if change(mid, phi_opp, mu_self, mu_opp, s).abs() < target_change.abs() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

pub fn snap(gap: i32) -> i32 {
    let step = f64::from(gap) / f64::from(GAP_STEP);
    let snapped = step.round() as i32 * GAP_STEP;
    snapped.clamp(-GAP_LIMIT, GAP_LIMIT)
}

pub fn settles_at(base: i32, opponent_mr: i32, won: bool) -> i32 {
    let (w, l) = est(snap(opponent_mr - base));
    base + if won { w } else { l }
}

pub fn settles_at_with(base: i32, opponent_mr: i32, won: bool, phi_self: f64, phi_opp: f64) -> i32 {
    let gap = snap(opponent_mr - base);
    let (w, l) = table(phi_self, phi_opp)
        .into_iter()
        .find(|(g, _)| *g == gap)
        .map(|(_, e)| e)
        .unwrap_or_else(|| est(gap));
    base + if won { w } else { l }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settles_at_with_reads_the_badges_own_table() {
        let prior = phi_of(PRIOR_RD);
        for (base, opp, won) in [(2057, 1893, true), (2042, 2131, true), (2063, 2083, false)] {
            assert_eq!(settles_at_with(base, opp, won, prior, prior), settles_at(base, opp, won));
        }
        let ours = phi_of(64.0);
        let expected = table(ours, prior).into_iter().find(|(g, _)| *g == snap(2131 - 2042)).unwrap().1 .0;
        assert_eq!(settles_at_with(2042, 2131, true, ours, prior), 2042 + expected);
    }

    #[test]
    fn bucketed_tables_reproduce_the_rows_that_are_two_apart() {
        let ours = phi_of(f64::from(bucket_rd(66.0)));
        let theirs = phi_of(f64::from(bucket_rd(61.0)));
        let look = |t: &[(i32, (i32, i32))], gap: i32, win: bool| {
            let e = t.iter().find(|(k, _)| *k == snap(gap)).expect("gap in table").1;
            if win { e.0 } else { e.1 }
        };
        let mine = table(ours, theirs);
        let opp = table(theirs, ours);
        for (me, them, real_me, real_opp) in [(2067, 1906, -17, 15), (2069, 2020, -14, 12)] {
            assert_eq!(look(&mine, them - me, false), real_me, "our side of {me} vs {them}");
            assert_eq!(look(&opp, me - them, true), real_opp, "their side of {me} vs {them}");
        }
    }

    #[test]
    fn the_bucket_width_is_fine_enough_to_separate_real_players() {
        assert_ne!(
            bucket_rd(61.0),
            bucket_rd(66.0),
            "the two deviations measured in one real session must land in different buckets"
        );
        assert_eq!(bucket_rd(10.0), RD_BUCKET_MIN, "clamped low");
        assert_eq!(bucket_rd(500.0), RD_BUCKET_MAX, "clamped high");
        assert!(buckets().all(|b| bucket_rd(f64::from(b)) == b), "buckets are their own snap");
    }

    #[test]
    fn est_matches_the_shipped_table() {
        let cases: &[(i32, i32, i32)] = &[
            (-600, 1, -24),
            (-260, 4, -19),
            (-250, 5, -19),
            (-240, 5, -19),
            (-100, 9, -15),
            (-60, 10, -14),
            (-50, 10, -13),
            (-40, 10, -13),
            (-10, 11, -12),
            (0, 12, -12),
            (10, 12, -11),
            (100, 15, -9),
        ];
        for &(gap, w, l) in cases {
            assert_eq!(est(gap), (w, l), "gap {gap}");
        }
    }

    #[test]
    fn the_table_is_monotonic_across_its_whole_range() {
        let mut prev = est(-GAP_LIMIT);
        let mut gap = -GAP_LIMIT + GAP_STEP;
        while gap <= GAP_LIMIT {
            let cur = est(gap);
            assert!(cur.0 >= prev.0, "win fell at gap {gap}: {prev:?} -> {cur:?}");
            assert!(cur.1 >= prev.1, "loss fell at gap {gap}: {prev:?} -> {cur:?}");
            prev = cur;
            gap += GAP_STEP;
        }
        assert!(est(GAP_LIMIT).0 > 20 && est(GAP_LIMIT).1 > -5);
        assert!(est(-GAP_LIMIT).0 < 5 && est(-GAP_LIMIT).1 < -20);
    }

    #[test]
    fn snap_rounds_to_the_grid_and_clamps() {
        assert_eq!(snap(0), 0);
        assert_eq!(snap(-54), -50);
        assert_eq!(snap(-56), -60);
        assert_eq!(snap(41), 40);
        assert_eq!(snap(9_999), GAP_LIMIT);
        assert_eq!(snap(-9_999), -GAP_LIMIT);
    }

    #[test]
    fn it_is_within_one_of_the_live_set() {
        for (base, opp, real, label) in
            [(2055, 1835, 5, "05:33"), (2060, 1830, 4, "05:36")]
        {
            let got = settles_at(base, opp, true) - base;
            assert!(
                (got - real).abs() <= 1,
                "{label}: model {got:+}, Wavu {real:+} - outside the +-1 the backtest promises"
            );
        }
        assert_eq!(settles_at(2055, 1835, true), 2060, "05:33 should be exact");
    }

    #[test]
    fn solve_phi_round_trips() {
        for &rd in &[35.0, 65.0, 110.0, 180.0] {
            for &gap in &[-300, -50, 0, 50, 300] {
                for &s in &[1.0, 0.0] {
                    let phi = rd / SCALE;
                    let opp = PRIOR_RD / SCALE;
                    let mu_opp = f64::from(gap) / SCALE;
                    let d = change(phi, opp, 0.0, mu_opp, s);
                    let back = solve_phi(d, opp, 0.0, mu_opp, s);
                    assert!(
                        (back - phi).abs() < 1e-3,
                        "rd {rd} gap {gap} s {s}: solved {} want {}",
                        back * SCALE,
                        rd
                    );
                }
            }
        }
    }

    #[test]
    fn the_opponents_missing_point_is_explained_by_a_bigger_rd() {
        let prior = PRIOR_RD / SCALE;
        let gap = 2055 - 1835;
        assert_eq!(est(gap).1, -5, "the prior-RD model is one short of Wavu's -6");

        let mu_opp = f64::from(gap) / SCALE;
        let phi = solve_phi(-6.0, prior, 0.0, mu_opp, 0.0);
        assert!(
            phi > prior,
            "a bigger swing must mean a bigger RD: solved {} vs prior {PRIOR_RD}",
            phi * SCALE
        );
        assert_eq!(
            est_with_rd(gap, phi, prior).1,
            -6,
            "with their own RD the estimate should land on Wavu's number"
        );
    }

    #[test]
    fn a_loss_settles_below_the_base() {
        let base = 2055;
        let lost = settles_at(base, 1835, false);
        assert!(lost < base, "a loss must go down: {lost} vs {base}");
        assert!(lost < settles_at(base, 1835, true));
    }
}
