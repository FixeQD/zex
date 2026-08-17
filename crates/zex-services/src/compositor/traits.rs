//! Shared types and the [`Compositor`] trait

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub address: String,
    pub title: String,
    pub class: String,
    pub workspace: i32,
    pub focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceInfo {
    pub id: i32,
    pub name: String,
    pub active: bool,
    pub focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompositorEvent {
    WorkspacesChanged,
    WorkspaceChanged { id: i32 },
    ActiveWindowChanged,
    WindowOpened,
    WindowClosed,
}

pub trait Compositor: Send + Sync {
    fn name(&self) -> &'static str;
    fn active_window(&self) -> Result<Option<WindowInfo>>;
    fn windows(&self) -> Result<Vec<WindowInfo>>;
    fn workspaces(&self) -> Result<Vec<WorkspaceInfo>>;
    fn switch_to_workspace(&self, id: i32) -> Result<()>;
    fn focus_window(&self, address: &str) -> Result<()>;
    fn quit(&self) -> Result<()>;
    fn events(&self) -> flume::Receiver<CompositorEvent>;
}
