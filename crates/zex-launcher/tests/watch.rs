use notify::event::{CreateKind, DataChange, ModifyKind, RemoveKind};
use notify::{Event, EventKind};
use std::path::PathBuf;
use zex_launcher::apps::{translate, Change};

fn sample_event(kind: EventKind, path: PathBuf) -> Event {
    Event {
        kind,
        paths: vec![path],
        ..Event::default()
    }
}

#[test]
fn maps_desktop_changes() {
    let path = PathBuf::from("/tmp/apps/firefox.desktop");

    let added = translate(sample_event(EventKind::Create(CreateKind::File), path.clone()));
    assert_eq!(added, vec![Change::Installed(path.clone())]);

    let removed = translate(sample_event(EventKind::Remove(RemoveKind::File), path.clone()));
    assert_eq!(removed, vec![Change::Removed(path.clone())]);

    let modified = translate(sample_event(
        EventKind::Modify(ModifyKind::Data(DataChange::Any)),
        path.clone(),
    ));
    assert_eq!(modified, vec![Change::Touched(path)]);
}

#[test]
fn maps_folder_events_to_rebuild() {
    let dir = PathBuf::from("/tmp/apps");
    let created = translate(sample_event(EventKind::Create(CreateKind::Folder), dir.clone()));
    assert_eq!(created, vec![Change::Rebuild(dir)]);
}

#[test]
fn ignores_foreign_paths() {
    let path = PathBuf::from("/tmp/apps/notes.txt");
    let created = translate(sample_event(EventKind::Create(CreateKind::File), path.clone()));
    assert!(created.is_empty());

    let modified = translate(sample_event(
        EventKind::Modify(ModifyKind::Data(DataChange::Any)),
        path,
    ));
    assert!(modified.is_empty());
}