use blasphem::ConfusionMatrix;

#[test]
fn calculates_binary_metrics_from_literal_counts() {
    let matrix = ConfusionMatrix {
        true_positive: 8,
        true_negative: 9,
        false_positive: 1,
        false_negative: 2,
    };

    let metrics = matrix.metrics();

    assert!((metrics.accuracy.expect("defined") - 0.85).abs() < f64::EPSILON);
    assert!((metrics.precision.expect("defined") - (8.0 / 9.0)).abs() < f64::EPSILON);
    assert!((metrics.recall.expect("defined") - 0.8).abs() < f64::EPSILON);
    assert!((metrics.specificity.expect("defined") - 0.9).abs() < f64::EPSILON);
    assert!((metrics.f1.expect("defined") - (16.0 / 19.0)).abs() < f64::EPSILON);
}

#[test]
fn returns_none_for_undefined_metrics() {
    let metrics = ConfusionMatrix::default().metrics();

    assert_eq!(metrics.accuracy, None);
    assert_eq!(metrics.precision, None);
    assert_eq!(metrics.recall, None);
    assert_eq!(metrics.specificity, None);
    assert_eq!(metrics.f1, None);
}
