use eos_plugin_runtime::{publish::*, verify_tuf_target};
use std::fs;
#[tokio::test]
async fn isolated_threshold_root_signs_targets_and_rejects_tampering() {
    let temp = tempfile::tempdir().unwrap();
    let keys = temp.path().join("keys");
    init_development_repository(&keys).await.unwrap();
    let targets = temp.path().join("targets");
    let metadata = temp.path().join("metadata");
    fs::create_dir(&targets).unwrap();
    fs::write(targets.join("fixture"), b"trusted-content").unwrap();
    let signing = vec![
        keys.join("targets.pk8"),
        keys.join("snapshot.pk8"),
        keys.join("timestamp.pk8"),
    ];
    sign_repository(&keys.join("root.json"), &targets, &metadata, 1, &signing)
        .await
        .unwrap();
    let root = fs::read(keys.join("root.json")).unwrap();
    assert_eq!(
        verify_tuf_target(
            &root,
            &metadata,
            &targets,
            &temp.path().join("cache"),
            "fixture"
        )
        .await
        .unwrap(),
        b"trusted-content"
    );
    fs::write(targets.join("fixture"), b"tampered-content").unwrap();
    assert!(verify_tuf_target(
        &root,
        &metadata,
        &targets,
        &temp.path().join("cache"),
        "fixture"
    )
    .await
    .is_err());
}
