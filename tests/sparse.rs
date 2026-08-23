use blasphem::SparseModel;

#[test]
fn sparse_model_rejects_a_truncated_artifact() {
    let error = SparseModel::from_bytes(b"TOXSPRS1").expect_err("truncated artifact");

    assert!(error.to_string().contains("length"));
}
