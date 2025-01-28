use crate::error::Error;
use notify::{Event, RecommendedWatcher, Watcher};
use tokio::sync::mpsc::{channel, Receiver};
use tracing::error;

pub(crate) fn create_watcher(
) -> Result<(RecommendedWatcher, Receiver<notify::Result<Event>>), Error> {
    let (tx, rx) = channel(1);
    let watcher = RecommendedWatcher::new(
        move |res| {
            if let Err(e) = tx.blocking_send(res) {
                error!("failed to tx event: {}", e)
            }
        },
        notify::Config::default(),
    )?;

    Ok((watcher, rx))
}
