use std::process::Command;
use tau_client::LinkConfig;
use tau_client::store::Secrets;

#[test]
fn standard_config_isolated_from_process_environment() {
    if let Ok(case) = std::env::var("TAU_TEST_CONFIG_CASE") {
        let config = LinkConfig::standard();
        let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap());
        let expected = if case == "root" || case == "both" {
            home.join("client")
        } else {
            home.join(".config/tau")
        };
        assert_eq!(config.dir, Some(expected.clone()));
        match config.secrets {
            Secrets::Files(path) if case == "tokens" || case == "both" => {
                assert_eq!(path, home.join("secrets"));
                let secrets = Secrets::Files(path);
                let token = tau_client::wire::Token::new("test-only-token");
                secrets.save("http://localhost:1", &token).unwrap();
                assert_eq!(secrets.load("http://localhost:1"), Some(token));
                secrets.delete("http://localhost:1").unwrap();
                assert_eq!(secrets.load("http://localhost:1"), None);
            }
            Secrets::Keychain
                if cfg!(target_os = "macos") && case != "both" && case != "tokens" => {}
            Secrets::Files(path) if !cfg!(target_os = "macos") => {
                assert_eq!(path, expected.join("tokens"));
            }
            _ => panic!("wrong secret store for {case}"),
        }
        return;
    }
    for case in ["default", "root", "tokens", "both"] {
        let home = tempfile::tempdir().unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "standard_config_isolated_from_process_environment",
                "--nocapture",
            ])
            .env("TAU_TEST_CONFIG_CASE", case)
            .env("HOME", home.path())
            .env_remove("TAU_CLIENT_DIR")
            .env_remove("TAU_TOKEN_DIR");
        if case == "root" || case == "both" {
            child.env("TAU_CLIENT_DIR", home.path().join("client"));
        }
        if case == "tokens" || case == "both" {
            child.env("TAU_TOKEN_DIR", home.path().join("secrets"));
        }
        assert!(child.status().unwrap().success(), "config case {case}");
        assert!(!home.path().join(".config").exists());
    }
}
