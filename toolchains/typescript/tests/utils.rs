#![allow(dead_code)]

use moon_common::Id;
use moon_pdk::{ProjectFragment, TaskFragment};
use moon_target::Target;

pub fn create_project(id: &str) -> ProjectFragment {
    ProjectFragment {
        dependency_scope: None,
        id: Id::raw(id),
        source: id.into(),
        toolchains: vec![Id::raw("typescript"), Id::raw("javascript")],
    }
}

pub fn create_task(target: &str) -> TaskFragment {
    TaskFragment {
        target: Target::parse(target).unwrap(),
        toolchains: vec![Id::raw("typescript"), Id::raw("javascript")],
    }
}
