use assert_cmd::cmd::Command;

#[test]
fn invalid_config_will_cause_commands_to_fail() {
    let mut cmd = Command::cargo_bin("spl-token").unwrap();
    cmd.args(["address", "--config", "~/nonexistent/config.yml"]);
    cmd.assert()
        .stderr("error: Could not find config file `~/nonexistent/config.yml`\n");
    cmd.assert().code(1).failure();
}
