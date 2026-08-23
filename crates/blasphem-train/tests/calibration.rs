use blasphem::{ConfusionMatrix, EvalLabel, Language};
use blasphem_train::calibration::{
    BoundaryEvaluation, CalibrationError, CalibrationRow, calibrate, calibrate_at_or_above,
    candidate_boundaries, gates, select_best,
};

#[test]
fn gate_boundaries_use_exact_integer_math() {
    let nine_of_300 = ConfusionMatrix {
        true_positive: 9,
        true_negative: 291,
        false_positive: 9,
        false_negative: 0,
    };
    let ten_of_300 = ConfusionMatrix {
        false_positive: 10,
        true_negative: 290,
        ..nine_of_300
    };
    let ninety_percent = ConfusionMatrix {
        true_positive: 9,
        true_negative: 299,
        false_positive: 1,
        false_negative: 0,
    };
    let below_ninety_percent = ConfusionMatrix {
        true_positive: 8,
        ..ninety_percent
    };
    let no_predictions = ConfusionMatrix {
        true_positive: 0,
        true_negative: 300,
        false_positive: 0,
        false_negative: 9,
    };
    let low_recall = ConfusionMatrix {
        true_positive: 1,
        true_negative: 300,
        false_positive: 0,
        false_negative: 299,
    };

    assert!(gates(nine_of_300).false_warning_passed);
    assert!(!gates(ten_of_300).false_warning_passed);
    assert!(gates(ninety_percent).precision_passed);
    assert!(!gates(below_ninety_percent).precision_passed);
    assert!(!gates(no_predictions).precision_passed);
    assert!(!gates(no_predictions).passed());
    assert!(gates(low_recall).passed());
}

#[test]
fn calibration_maximizes_true_positives_then_minimizes_false_positives() {
    let result = calibrate(Language::En, &fixture_rows()).expect("calibrate");
    assert_eq!(result.boundary, 11);
    assert_eq!(result.matrix.true_positive, 9);
    assert_eq!(result.matrix.false_positive, 1);
}

#[test]
fn rule_channel_gate_failure_cannot_be_hidden_by_sparse_threshold() {
    let error = calibrate(Language::En, &rule_only_false_warnings()).expect_err("gate failure");
    assert!(matches!(
        error,
        CalibrationError::RuleChannelGateFailure(Language::En)
    ));
}

#[test]
fn searches_all_distinct_and_adjacent_boundaries() {
    let rows = [toxic(-2), clean(0), toxic(0), clean(7)];
    assert_eq!(candidate_boundaries(&rows), vec![-2, -1, 0, 1, 7, 8]);
}

#[test]
fn rejects_no_admissible_boundary() {
    let mut rows = Vec::new();
    rows.extend((0..300).map(|_| toxic(5)));
    rows.extend((0..300).map(|_| clean(5)));
    assert!(matches!(
        calibrate(Language::En, &rows),
        Err(CalibrationError::NoAdmissibleBoundary(Language::En))
    ));
}

#[test]
fn breaks_ties_with_false_warnings_then_boundary() {
    let matrix = |false_positive| ConfusionMatrix {
        true_positive: 99,
        true_negative: 300 - false_positive,
        false_positive,
        false_negative: 1,
    };
    let fewer_false_warnings = select_best(
        Language::En,
        &[
            BoundaryEvaluation {
                boundary: 20,
                matrix: matrix(1),
            },
            BoundaryEvaluation {
                boundary: 10,
                matrix: matrix(0),
            },
        ],
    )
    .expect("candidate");
    assert_eq!(fewer_false_warnings.boundary, 10);

    let higher_boundary = select_best(
        Language::En,
        &[
            BoundaryEvaluation {
                boundary: 20,
                matrix: matrix(0),
            },
            BoundaryEvaluation {
                boundary: 21,
                matrix: matrix(0),
            },
        ],
    )
    .expect("candidate");
    assert_eq!(higher_boundary.boundary, 21);
}

fn fixture_rows() -> Vec<CalibrationRow> {
    let mut rows = Vec::new();
    rows.extend((0..9).map(|_| toxic(11)));
    rows.push(toxic(10));
    rows.push(clean(11));
    rows.extend((0..4).map(|_| clean(10)));
    rows.extend((0..95).map(|_| clean(0)));
    rows
}

fn rule_only_false_warnings() -> Vec<CalibrationRow> {
    let mut rows = vec![toxic(0)];
    rows.extend((0..10).map(|_| rule_clean(0)));
    rows.extend((0..290).map(|_| clean(0)));
    rows
}

fn toxic(sparse_raw_score: i32) -> CalibrationRow {
    CalibrationRow {
        label: EvalLabel::Toxic,
        sparse_raw_score,
        rule_should_nudge: false,
        suppress_sparse: false,
    }
}

fn clean(sparse_raw_score: i32) -> CalibrationRow {
    CalibrationRow {
        label: EvalLabel::Clean,
        sparse_raw_score,
        rule_should_nudge: false,
        suppress_sparse: false,
    }
}

fn rule_clean(sparse_raw_score: i32) -> CalibrationRow {
    CalibrationRow {
        label: EvalLabel::Clean,
        sparse_raw_score,
        rule_should_nudge: true,
        suppress_sparse: false,
    }
}

#[test]
fn contextual_suppression_disables_only_the_sparse_decision() {
    let mut rows = Vec::new();
    rows.extend((0..9).map(|_| toxic(20)));
    rows.push(CalibrationRow {
        label: EvalLabel::Clean,
        sparse_raw_score: 100,
        rule_should_nudge: false,
        suppress_sparse: true,
    });
    rows.extend((0..99).map(|_| clean(0)));

    let result = calibrate(Language::En, &rows).expect("suppressed clean row");
    assert_eq!(result.boundary, 20);
    assert_eq!(result.matrix.false_positive, 0);
    assert_eq!(result.matrix.true_positive, 9);
}

#[test]
fn a_clean_control_floor_can_trade_recall_for_zero_control_warnings() {
    let mut rows = Vec::new();
    rows.extend((0..10).map(|_| toxic(20)));
    rows.push(toxic(30));
    rows.extend((0..100).map(|_| clean(0)));

    let unconstrained = calibrate(Language::En, &rows).expect("unconstrained calibration");
    let guarded = calibrate_at_or_above(Language::En, &rows, 26).expect("guarded calibration");

    assert_eq!(unconstrained.boundary, 20);
    assert!(guarded.boundary >= 26);
    assert_eq!(guarded.matrix.false_positive, 0);
    assert_eq!(guarded.matrix.true_positive, 1);
}
