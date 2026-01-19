use std::cmp;

use crate::{
    any::{HitResultGenerator, hitresult_generator::Fast},
    osu::{OsuHitResults, performance::hitresult_generator::OsuHitResultParams},
};

impl HitResultGenerator<OsuHitResultParams> for Fast {
    fn generate_hitresults(params: &OsuHitResultParams) -> OsuHitResults {
        let large_tick_hits = params.large_tick_hits.unwrap_or(0);
        let small_tick_hits = params.small_tick_hits.unwrap_or(0);
        let slider_end_hits = params.slider_end_hits.unwrap_or(0);

        let misses = cmp::min(params.misses, params.total_hits);
        let remain = params.total_hits - misses;

        if remain == 0 {
            return OsuHitResults {
                large_tick_hits,
                small_tick_hits,
                slider_end_hits,
                n300: 0,
                n100: 0,
                n50: 0,
                misses,
            };
        }

        let (tick_score, tick_max) =
            params
                .origin
                .tick_scores(large_tick_hits, small_tick_hits, slider_end_hits);

        // acc = (300*n300 + 100*n100 + 50*n50 + tick_score) / (300*total_hits + tick_max)
        // Simplify by dividing by 50: (reducing risk of overflow)
        // acc = (6*n300 + 2*n100 + n50 + tick_score/50) / (6*total_hits + tick_max/50)

        let target_total = f64::round(
            params.acc * (f64::from(6 * params.total_hits) + f64::from(tick_max) / 50.0),
        ) as u32;

        // Start by assuming every non-miss is an n50
        // delta is how much we need to increase from the baseline (all n50s)
        let baseline = remain + tick_score / 50;
        let delta = target_total.saturating_sub(baseline);

        // Each n300 increases by 5 (6-1), each n100 increases by 1 (2-1)
        // delta = 5*n300 + 1*n100

        let n300 = cmp::min(remain, delta / 5);
        let n100 = cmp::min(remain - n300, delta % 5);
        let n50 = remain.saturating_sub(n300 + n100);

        OsuHitResults {
            large_tick_hits,
            small_tick_hits,
            slider_end_hits,
            n300,
            n100,
            n50,
            misses,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::osu::OsuScoreOrigin;

    use super::*;

    #[test]
    fn perfect_accuracy_no_misses() {
        let params = OsuHitResultParams {
            total_hits: 100,
            acc: 1.0,
            n300: None,
            n100: None,
            n50: None,
            misses: 0,
            large_tick_hits: None,
            small_tick_hits: None,
            slider_end_hits: None,
            origin: OsuScoreOrigin::Stable,
        };

        let result = Fast::generate_hitresults(&params);

        assert_eq!(result.n300, 100);
        assert_eq!(result.n100, 0);
        assert_eq!(result.n50, 0);
        assert_eq!(result.misses, 0);
        assert_eq!(result.accuracy(OsuScoreOrigin::Stable), 1.0);
    }

    #[test]
    fn high_accuracy_stable() {
        let params = OsuHitResultParams {
            total_hits: 1000,
            acc: 0.95,
            n300: None,
            n100: None,
            n50: None,
            misses: 10,
            large_tick_hits: None,
            small_tick_hits: None,
            slider_end_hits: None,
            origin: OsuScoreOrigin::Stable,
        };

        let result = Fast::generate_hitresults(&params);

        // Verify total adds up
        assert_eq!(result.n300 + result.n100 + result.n50 + result.misses, 1000);
        assert_eq!(result.misses, 10);

        // Verify accuracy is close to target
        let actual_acc = result.accuracy(OsuScoreOrigin::Stable);
        assert!(
            (actual_acc - 0.95).abs() < 0.001,
            "Expected ~0.95, got {actual_acc}",
        );
    }

    #[test]
    fn medium_accuracy_stable() {
        let params = OsuHitResultParams {
            total_hits: 500,
            acc: 0.85,
            n300: None,
            n100: None,
            n50: None,
            misses: 25,
            large_tick_hits: None,
            small_tick_hits: None,
            slider_end_hits: None,
            origin: OsuScoreOrigin::Stable,
        };

        let result = Fast::generate_hitresults(&params);

        assert_eq!(result.n300 + result.n100 + result.n50 + result.misses, 500);
        assert_eq!(result.misses, 25);

        let actual_acc = result.accuracy(OsuScoreOrigin::Stable);
        assert!(
            (actual_acc - 0.85).abs() < 0.001,
            "Expected ~0.85, got {actual_acc}",
        );
    }

    #[test]
    fn with_slider_acc() {
        let params = OsuHitResultParams {
            total_hits: 200,
            acc: 0.98,
            n300: None,
            n100: None,
            n50: None,
            misses: 2,
            large_tick_hits: Some(50),
            small_tick_hits: None,
            slider_end_hits: Some(40),
            origin: OsuScoreOrigin::WithSliderAcc {
                max_large_ticks: 50,
                max_slider_ends: 40,
            },
        };

        let result = Fast::generate_hitresults(&params);

        assert_eq!(result.n300 + result.n100 + result.n50 + result.misses, 200);
        assert_eq!(result.misses, 2);
        assert_eq!(result.large_tick_hits, 50);
        assert_eq!(result.slider_end_hits, 40);

        let actual_acc = result.accuracy(params.origin);
        assert!(
            (actual_acc - 0.98).abs() < 0.002,
            "Expected ~0.98, got {actual_acc}",
        );
    }

    #[test]
    fn without_slider_acc() {
        let params = OsuHitResultParams {
            total_hits: 300,
            acc: 0.92,
            n300: None,
            n100: None,
            n50: None,
            misses: 5,
            large_tick_hits: Some(60),
            small_tick_hits: Some(100),
            slider_end_hits: None,
            origin: OsuScoreOrigin::WithoutSliderAcc {
                max_large_ticks: 60,
                max_small_ticks: 100,
            },
        };

        let result = Fast::generate_hitresults(&params);

        assert_eq!(result.n300 + result.n100 + result.n50 + result.misses, 300);
        assert_eq!(result.misses, 5);
        assert_eq!(result.large_tick_hits, 60);
        assert_eq!(result.small_tick_hits, 100);

        let actual_acc = result.accuracy(params.origin);
        assert!(
            (actual_acc - 0.92).abs() < 0.002,
            "Expected ~0.92, got {actual_acc}",
        );
    }

    #[test]
    fn all_misses() {
        let params = OsuHitResultParams {
            total_hits: 50,
            acc: 0.0,
            n300: None,
            n100: None,
            n50: None,
            misses: 50,
            large_tick_hits: None,
            small_tick_hits: None,
            slider_end_hits: None,
            origin: OsuScoreOrigin::Stable,
        };

        let result = Fast::generate_hitresults(&params);

        assert_eq!(result.n300, 0);
        assert_eq!(result.n100, 0);
        assert_eq!(result.n50, 0);
        assert_eq!(result.misses, 50);
        assert_eq!(result.accuracy(OsuScoreOrigin::Stable), 0.0);
    }

    #[test]
    fn low_accuracy_many_50s() {
        let params = OsuHitResultParams {
            total_hits: 400,
            acc: 0.60,
            n300: None,
            n100: None,
            n50: None,
            misses: 50,
            large_tick_hits: None,
            small_tick_hits: None,
            slider_end_hits: None,
            origin: OsuScoreOrigin::Stable,
        };

        let result = Fast::generate_hitresults(&params);

        assert_eq!(result.n300 + result.n100 + result.n50 + result.misses, 400);
        assert_eq!(result.misses, 50);
        // At 60% accuracy with many misses, we should have a lot of 50s
        assert!(result.n50 > 0, "Expected some n50s at low accuracy");

        let actual_acc = result.accuracy(OsuScoreOrigin::Stable);
        assert!(
            (actual_acc - 0.60).abs() < 0.002,
            "Expected ~0.60, got {actual_acc}",
        );
    }

    #[test]
    fn edge_case_more_misses_than_hits() {
        let params = OsuHitResultParams {
            total_hits: 100,
            acc: 0.5,
            n300: None,
            n100: None,
            n50: None,
            misses: 150, // More misses than total hits
            large_tick_hits: None,
            small_tick_hits: None,
            slider_end_hits: None,
            origin: OsuScoreOrigin::Stable,
        };

        let result = Fast::generate_hitresults(&params);

        // Should clamp misses to total_hits
        assert_eq!(result.misses, 100);
        assert_eq!(result.n300, 0);
        assert_eq!(result.n100, 0);
        assert_eq!(result.n50, 0);
    }
}
