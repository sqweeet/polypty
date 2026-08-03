use super::*;

fn lift(frame: u8, column: usize, row: GlintRow) -> f32 {
    let color = super::working_glint_bg(true, GlintFrame(frame), column, 18, row);
    let Color::Rgb { r, .. } = color else {
        panic!("expected RGB color");
    };
    f32::from(r) - 48.0
}

/// Lit mass of one row, and where along the row that mass sits.
fn band(frame: u8, row: GlintRow) -> (f32, Option<f32>) {
    let mass: f32 = (0..18).map(|column| lift(frame, column, row)).sum();
    let centroid = (mass > 0.0).then(|| {
        (0..18)
            .map(|column| column as f32 * lift(frame, column, row))
            .sum::<f32>()
            / mass
    });
    (mass, centroid)
}

#[test]
fn glint_is_vertical_across_card_rows() {
    for frame in 0..50 {
        for column in 0..18 {
            let flat = lift(frame, column, GlintRow::Flat);
            assert_eq!(lift(frame, column, GlintRow::Upper), flat);
            assert_eq!(lift(frame, column, GlintRow::Lower), flat);
        }
    }
}

#[test]
fn glint_sweep_joins_rest_without_a_seam() {
    // The tear this guards: a row still lit on the last sweep frame snapped down
    // to base when the timeline handed over to REST, and a row lit at column 0
    // popped in. Both ends of the sweep must already rest at base.
    for row in [GlintRow::Upper, GlintRow::Flat, GlintRow::Lower] {
        for frame in [0, 49] {
            assert_eq!(band(frame, row).0, 0.0, "{row:?} frame {frame}");
        }
    }
}

#[test]
fn glint_spans_both_card_rows_through_the_body_of_the_sweep() {
    // Both rows must carry the same highlight through the visible pass so the
    // card reads as one surface rather than two detached flashes.
    for frame in 8..42 {
        assert!(
            band(frame, GlintRow::Upper).0 > 0.0,
            "upper row dark at frame {frame}"
        );
        assert!(
            band(frame, GlintRow::Lower).0 > 0.0,
            "lower row dark at frame {frame}"
        );
    }
}

#[test]
fn glint_has_a_broad_soft_gradient() {
    for row in [GlintRow::Upper, GlintRow::Flat, GlintRow::Lower] {
        let lifts: Vec<_> = (0..18).map(|column| lift(25, column, row)).collect();
        let lit = lifts.iter().filter(|lift| **lift > 0.0).count();
        let bright = lifts.iter().filter(|lift| **lift >= 10.0).count();
        assert!(lit >= 8, "{row:?} lights only {lit} columns");
        assert!(bright >= 4, "{row:?} strongly lights only {bright} columns");
        for pair in lifts.windows(2) {
            assert!((pair[1] - pair[0]).abs() <= 6.0, "{row:?}: {lifts:?}");
        }
    }
}

#[test]
fn glint_never_steps_hard_between_frames() {
    // The visible tear was a hard step, so bound every row and every frame pair
    // across the whole cycle instead of one sampled pair.
    for row in [GlintRow::Upper, GlintRow::Flat, GlintRow::Lower] {
        for frame in 0..50 {
            for column in 0..18 {
                let step = lift(frame, column, row) - lift(frame + 1, column, row);
                assert!(
                    step.abs() <= 5.0,
                    "step {step} at frame {frame} column {column}"
                );
            }
        }
    }
}
