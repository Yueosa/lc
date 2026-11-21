use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

fn cli(root: &Path, args: &[&OsStr], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lc"))
        .current_dir(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn run(root: &Path, args: &[&str], input: &str) -> String {
    let args: Vec<_> = args.iter().map(OsStr::new).collect();
    let output = cli(root, &args, input);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn sample() -> TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("tools/converter/plugins/csv/samples")).unwrap();
    fs::write(
        root.path().join("config.toml"),
        include_bytes!("project/config.toml"),
    )
    .unwrap();
    fs::write(
        root.path()
            .join("tools/converter/plugins/csv/samples/devices.csv"),
        include_bytes!("project/tools/converter/plugins/csv/samples/devices.csv"),
    )
    .unwrap();
    run(root.path(), &["init"], "");
    root
}

#[test]
fn nested_projects_round_trip_and_survive_outer_clear() {
    let source = sample();
    let target = sample();
    let source = source.path();
    let target = target.path();
    let nested = "tools/converter";
    let plugin = "tools/converter/plugins/csv";
    let item = "tools/converter/plugins/csv/samples/devices.csv";
    run(source, &["set", item, "outer record"], "");
    run(&source.join(nested), &["init"], "");
    run(&source.join(plugin), &["init"], "");
    run(source, &["set", item, "CSV sample"], "");
    run(source, &["set", "config.toml", "配置"], "");

    let tree = run(source, &["-R"], "");
    assert!(tree.contains("converter/ [project]"));
    assert!(!tree.contains("devices.csv"));

    let work = tempfile::tempdir().unwrap();
    let bundle = work.path().join("bundle.json");
    let bundle = bundle.to_str().unwrap();
    run(source, &["export", ".", "-o", bundle], "");
    assert!(run(target, &["import", bundle, "--root", "."], "y\n").contains("Imported 2 entries"));
    assert_eq!(run(target, &["get", item], ""), "CSV sample\n");
    assert!(target.join(plugin).join(".lc.json").is_file());
    assert!(run(target, &["import", bundle, "--root", "."], "").contains("Conflicts:        2"));
    assert!(run(source, &["clear", "--all"], "confirm\n").contains("Cleared 1"));
    assert_eq!(run(source, &["get", item], ""), "CSV sample\n");
    assert!(
        fs::read_to_string(source.join(".lc.json"))
            .unwrap()
            .contains("outer record")
    );
}

#[test]
fn invalid_names_and_missing_records_are_handled_without_losing_data() {
    let root = sample();
    let root = root.path();
    let raw = OsString::from_vec(b"raw-\xff".to_vec());
    fs::write(root.join(&raw), b"preserve this").unwrap();
    let output = cli(root, &["set".as_ref(), &raw, "raw comment".as_ref()], "");
    assert!(output.status.success());
    assert!(run(root, &[], "").contains(r"raw-\xff"));
    assert_eq!(
        cli(root, &["get".as_ref(), &raw], "").stdout,
        b"raw comment\n"
    );

    run(root, &["set", "config.toml", "config comment"], "");
    fs::remove_file(root.join("config.toml")).unwrap();
    assert!(run(root, &["clean", "."], "y\n").contains("Removed 1 invalid record"));
    assert_eq!(fs::read(root.join(&raw)).unwrap(), b"preserve this");
    assert_eq!(
        cli(root, &["get".as_ref(), &raw], "").stdout,
        b"raw comment\n"
    );
}

#[test]
fn hidden_project_is_only_opened_when_visible() {
    let root = sample();
    let root = root.path();
    fs::create_dir(root.join(".broken")).unwrap();
    fs::write(root.join(".broken/.lc.json"), b"invalid JSON").unwrap();
    assert!(run(root, &["-R"], "").contains("config.toml"));
    assert!(!cli(root, &["-a".as_ref()], "").status.success());
}
