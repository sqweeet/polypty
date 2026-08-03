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
fn glint_slants_evenly_between_card_rows() {
    // Mid-sweep the band is fully on screen, so each row is offset by exactly one
    // column from the next — a straight, evenly spaced diagonal.
    for frame in [20, 25, 30] {
        let upper = band(frame, GlintRow::Upper).1.unwrap();
        let flat = band(frame, GlintRow::Flat).1.unwrap();
        let lower = band(frame, GlintRow::Lower).1.unwrap();
        assert!((flat - upper - 1.0).abs() < 0.05, "frame {frame}");
        assert!((lower - flat - 1.0).abs() < 0.05, "frame {frame}");
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
    // A tilted band enters corner-first, so the lower row leads the upper one at
    // the very edges. Once it is on screen both rows must carry it together, or
    // the card reads as two detached flashes instead of one diagonal.
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
fn glint_never_steps_hard_between_frames() {
    // The visible tear was a hard step, so bound every row and every frame pair
    // across the whole cycle instead of one sampled pair.
    for row in [GlintRow::Upper, GlintRow::Flat, GlintRow::Lower] {
        for frame in 0..50 {
            for column in 0..18 {
                let step = lift(frame, column, row) - lift(frame + 1, column, row);
                assert!(
                    step.abs() <= 7.0,
                    "step {step} at frame {frame} column {column}"
                );
            }
        }
    }
}
