use std::path::Path;

use notify::{RecommendedWatcher, Watcher};
use tokio::sync::mpsc;
use tracing::{instrument, trace};

pub use notify::Event;

#[instrument]
fn async_watcher() -> notify::Result<(
    RecommendedWatcher,
    mpsc::Receiver<notify::Result<notify::Event>>,
)> {
    let (tx, rx) = mpsc::channel(1);

    let watcher = RecommendedWatcher::new(
        move |res| {
            tx.blocking_send(res).unwrap();
        },
        notify::Config::default(),
    )?;

    Ok((watcher, rx))
}

pub trait HandleEvent {
    fn handle_event(&mut self, event: notify::Event) -> impl Future<Output = ()> + Send;
}

#[instrument(skip(handler))]
pub async fn watch_file<P, EventHandler>(path: P, mut handler: EventHandler) -> notify::Result<()>
where
    P: AsRef<Path> + std::fmt::Debug,
    EventHandler: HandleEvent,
{
    trace!(path = ?path, "Watching file");

    let (mut watcher, mut rx) = async_watcher()?;

    watcher.watch(path.as_ref(), notify::RecursiveMode::Recursive)?;

    while let Some(res) = rx.recv().await {
        match res {
            Ok(event) => {
                trace!("File changed: {event:?}");
                handler.handle_event(event).await;
            }
            Err(e) => {
                trace!("Error watching config file: {e:?}");
            }
        }
    }

    Ok(())
}
