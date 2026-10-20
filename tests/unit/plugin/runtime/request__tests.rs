use super::*;

fn context() -> RequestContext {
    RequestContext {
        plugin_id: "org.example.runtime".into(),
        plugin_version: "1.2.3".into(),
        plugin_sha256: "a".repeat(64),
        numeric_game_id: 42,
        assignment_id: "b".repeat(32),
        game_folder: PathBuf::from(r"C:\Games With Spaces\Trail\"),
        selected_target: PathBuf::from(r#"C:\Games With Spaces\Trail\say "go".exe"#),
        resolved_launcher: Some(PathBuf::from(
            r"C:\Games With Spaces\Trail\plugin launcher.exe",
        )),
    }
}

fn operation_id() -> String {
    "d".repeat(32)
}

#[test]
fn the_request_file_round_trips_and_names_the_switch_and_the_environment() {
    let root = tempfile::tempdir().unwrap();
    let id = operation_id();
    let file = RequestFile::create(
        root.path(),
        Operation::Prepare,
        &context(),
        &id,
        &super::super::channel::pipe_name(&id),
    )
    .unwrap();
    let parsed: Request = serde_json::from_slice(&fs::read(&file.path).unwrap()).unwrap();
    assert_eq!(parsed, file.request);
    assert_eq!(parsed.protocol, PROTOCOL);
    assert_eq!(parsed.operation, Operation::Prepare);
    assert!(parsed.game.folder.ends_with('\\'));
    assert!(parsed.game.executable.contains('"'));
    assert_eq!(
        parsed.game.launcher.as_deref(),
        Some(r"C:\Games With Spaces\Trail\plugin launcher.exe")
    );
    assert_eq!(parsed.plugin.id, "org.example.runtime");
    let argv = file.argv();
    assert_eq!(argv.len(), 2);
    assert_eq!(argv[0], SWITCH);
    assert_eq!(argv[1], file.path.as_os_str());
    assert_eq!(
        file.environment(),
        vec![
            (PIPE_VARIABLE.into(), OsString::from(&parsed.pipe)),
            (OPERATION_ID_VARIABLE.into(), OsString::from(&id)),
        ]
    );
    assert!(fs::metadata(&file.path).unwrap().len() as usize <= REQUEST_LIMIT);
    let path = file.path.clone();
    drop(file);
    assert!(!path.exists(), "the request is removed with its operation");
}

#[test]
fn the_launcher_is_null_without_a_resolved_launcher_and_unsafe_paths_are_rejected() {
    let mut no_launcher = context();
    no_launcher.resolved_launcher = None;
    let id = operation_id();
    let pipe = super::super::channel::pipe_name(&id);
    let request = Request::new(Operation::Launch, &no_launcher, id.clone(), pipe.clone()).unwrap();
    assert_eq!(request.game.launcher, None);
    let mut unsafe_context = context();
    unsafe_context.selected_target = PathBuf::from(r"C:\Games\..\Windows\game.exe");
    assert!(Request::new(
        Operation::Prepare,
        &unsafe_context,
        id.clone(),
        pipe.clone()
    )
    .is_err());
    let mut wrong_pipe = request.clone();
    wrong_pipe.pipe = r"\\.\pipe\somewhere-else".into();
    assert!(wrong_pipe.validate().is_err());
    let mut wrong_protocol = request;
    wrong_protocol.protocol = 1;
    assert!(wrong_protocol.validate().is_err());
}

#[test]
fn game_ids_are_stable_and_distinct_and_operation_ids_random() {
    assert_eq!(protocol_game_id(7), protocol_game_id(7));
    assert_ne!(protocol_game_id(7), protocol_game_id(8));
    assert_eq!(protocol_game_id(7).len(), 32);
    assert_ne!(new_operation_id().unwrap(), new_operation_id().unwrap());
    assert_eq!(new_operation_id().unwrap().len(), 32);
}

#[test]
fn request_argv_strips_only_the_local_verbatim_prefix() {
    assert_eq!(
        normalized_local_argument(Path::new(r"\\?\D:\State\session\request.json")).unwrap(),
        PathBuf::from(r"D:\State\session\request.json")
    );
    assert!(normalized_local_argument(Path::new(r"\\?\UNC\server\share\request.json")).is_err());
    assert!(normalized_local_argument(Path::new(r"relative\request.json")).is_err());
}
