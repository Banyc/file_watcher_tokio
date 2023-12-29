use std::{path::Path, sync::Arc};

use notify::{RecommendedWatcher, Watcher};
use tokio::sync::mpsc::Receiver;
use tracing::{instrument, trace};

#[instrument]
fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<notify::Event>>)>
{
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        notify::Config::default(),
    )?;

    Ok((watcher, rx))
}

pub trait EventActor {
    fn notify(&self, event: notify::Event) -> impl std::future::Future<Output = ()> + Send;
}

#[instrument(skip(actor))]
pub async fn watch_file<P, EA>(path: P, actor: Arc<EA>) -> notify::Result<()>
where
    P: AsRef<Path> + std::fmt::Debug,
    EA: EventActor,
{
    trace!(path = ?path, "Watching file");

    let (mut watcher, mut rx) = async_watcher()?;

    watcher.watch(path.as_ref(), notify::RecursiveMode::Recursive)?;

    loop {
        let res = rx.recv().await;
        let Some(res) = res else {
            break;
        };
        match res {
            Ok(event) => {
                trace!("File changed: {:?}", event);
                actor.notify(event).await;
            }
            Err(e) => {
                trace!("Error watching config file: {:?}", e);
            }
        }
    }

    Ok(())
}
