#![allow(dead_code)]

use moon_common::Id;
use moon_pdk_api::{ProjectFragment, SyncOutput, TaskFragment};
use moon_target::Target;

pub fn create_project(id: &str) -> ProjectFragment {
    ProjectFragment {
        alias: None,
        dependency_scope: None,
        id: Id::raw(id),
        source: id.into(),
        toolchains: vec![Id::raw("typescript"), Id::raw("javascript")],
    }
}

pub fn create_project_dependencies() -> Vec<ProjectFragment> {
    vec![
        create_project("a"),
        create_project("b"),
        create_project("c"),
    ]
}

pub fn create_task(target: &str) -> TaskFragment {
    TaskFragment {
        target: Target::parse(target).unwrap(),
        toolchains: vec![Id::raw("typescript"), Id::raw("javascript")],
    }
}

pub fn has_changed_file(output: &SyncOutput, name: &str) -> bool {
    output
        .changed_files
        .iter()
        .any(|file| file.any_path().as_os_str() == name)
}
