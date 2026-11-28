use super::super::publication::CodexAuthorityView;
use super::super::router::CodexCommittedRoute;
use super::route::ProjectionRoute;
use std::collections::HashSet;

pub(super) enum ProjectionInput {
    View {
        route: ProjectionRoute,
        view: CodexAuthorityView,
    },
    Closed {
        route: ProjectionRoute,
    },
}

pub(super) struct ProjectionForwarders {
    installed: HashSet<u64>,
    pub(super) tasks: tokio::task::JoinSet<()>,
}

impl ProjectionForwarders {
    pub(super) fn new() -> Self {
        Self {
            installed: HashSet::new(),
            tasks: tokio::task::JoinSet::new(),
        }
    }

    pub(super) fn install(
        &mut self,
        route: CodexCommittedRoute,
        sender: tokio::sync::mpsc::Sender<ProjectionInput>,
    ) {
        if !self.installed.insert(route.revision) {
            return;
        }
        self.tasks.spawn(forward(route, sender));
    }
    pub(super) fn install_seeded(
        &mut self,
        route: CodexCommittedRoute,
        sender: tokio::sync::mpsc::Sender<ProjectionInput>,
    ) -> Option<ProjectionInput> {
        if !self.installed.insert(route.revision) {
            return None;
        }
        let mut authority = route.entry.runtime().subscribe_authority();
        let projection_route = ProjectionRoute::capture(&route);
        drop(route);
        let initial = authority.borrow_and_update().clone();
        self.tasks
            .spawn(forward_changes(projection_route.clone(), authority, sender));
        Some(ProjectionInput::View {
            route: projection_route,
            view: initial,
        })
    }
}

async fn forward(route: CodexCommittedRoute, sender: tokio::sync::mpsc::Sender<ProjectionInput>) {
    let mut authority = route.entry.runtime().subscribe_authority();
    let projection_route = ProjectionRoute::capture(&route);
    drop(route);
    let initial = authority.borrow_and_update().clone();
    if sender
        .send(ProjectionInput::View {
            route: projection_route.clone(),
            view: initial,
        })
        .await
        .is_err()
    {
        return;
    }
    forward_changes(projection_route, authority, sender).await;
}

async fn forward_changes(
    projection_route: ProjectionRoute,
    mut authority: tokio::sync::watch::Receiver<CodexAuthorityView>,
    sender: tokio::sync::mpsc::Sender<ProjectionInput>,
) {
    loop {
        if authority.changed().await.is_err() {
            let _ = sender
                .send(ProjectionInput::Closed {
                    route: projection_route,
                })
                .await;
            return;
        }
        let view = authority.borrow_and_update().clone();
        if sender
            .send(ProjectionInput::View {
                route: projection_route.clone(),
                view,
            })
            .await
            .is_err()
        {
            return;
        }
    }
}
