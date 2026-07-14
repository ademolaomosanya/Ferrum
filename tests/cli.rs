use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn ferrum() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ferrum"))
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

#[test]
fn help_describes_the_pipeline_commands() {
    let output = ferrum().arg("--help").output().expect("Ferrum should run");
    let stdout = String::from_utf8(output.stdout).expect("help should be UTF-8");

    assert!(output.status.success());
    assert!(stdout.contains("educational browser engine"));
    assert!(stdout.contains("ferrum paint"));
    assert!(stdout.contains("ferrum browse"));
}

#[test]
fn version_matches_the_package_version() {
    let output = ferrum()
        .arg("--version")
        .output()
        .expect("Ferrum should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("version should be UTF-8"),
        format!("ferrum {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn inspects_an_html_fixture() {
    let output = ferrum()
        .arg(fixture("hello.html"))
        .output()
        .expect("Ferrum should run");
    let stdout = String::from_utf8(output.stdout).expect("tree should be UTF-8");

    assert!(output.status.success());
    assert!(stdout.contains("#document"));
    assert!(stdout.contains("<main id=\"app\">"));
}

#[test]
fn reports_invalid_html_without_panicking() {
    let output = ferrum()
        .args(["-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .expect("stdin should be piped")
                .write_all(b"<main></p>")?;
            child.wait_with_output()
        })
        .expect("Ferrum should run");
    let stderr = String::from_utf8(output.stderr).expect("error should be UTF-8");

    assert!(!output.status.success());
    assert!(stderr.contains("HTML parse error"));
    assert!(stderr.contains("expected </main>"));
}

#[test]
fn paints_a_valid_ppm_from_example_inputs() {
    let output_path = std::env::temp_dir().join(format!(
        "ferrum-cli-test-{}-{}.ppm",
        std::process::id(),
        env!("CARGO_PKG_VERSION")
    ));
    let output = ferrum()
        .args([
            "paint".into(),
            fixture("hello.html").into_os_string(),
            fixture("theme.css").into_os_string(),
            output_path.clone().into_os_string(),
        ])
        .output()
        .expect("Ferrum should run");
    let bytes = fs::read(&output_path).expect("paint should create an image");
    fs::remove_file(&output_path).expect("test image should be removable");

    assert!(output.status.success());
    assert!(bytes.starts_with(b"P6\n800 600\n255\n"));
    assert_eq!(bytes.len(), b"P6\n800 600\n255\n".len() + 800 * 600 * 3);
}
