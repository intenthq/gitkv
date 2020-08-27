use assert_cmd::Command;

#[test]
fn fails_to_start_with_invalid_host() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("--host=")
        .assert();

    assert.failure();
}

#[test]
fn fails_to_start_with_invalid_port() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("--port=")
        .assert();

    assert.failure();
}

#[test]
fn fails_to_start_with_invalid_repo_root() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let assert = cmd
        .arg("--repo-root=")
        .assert();

    assert.failure();
}

// FIXME: How to test with a process that never ends unless terminated?
// #[test]
// fn can_cat_file() {
//     let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

//     let assert = cmd
//         .arg("--repo-root=test")
//         .assert();

//     assert.success();
// }
