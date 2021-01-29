use anyhow::Result;
use tokio::sync::{Mutex, broadcast::Sender, broadcast::Receiver};
use once_cell::sync::OnceCell;
use typemap::SendMap;

struct Mailbox<T> {
    tx: Sender<T>,
}

impl<T: Clone> Mailbox<T> {
    fn new() -> Mailbox<T> {
        let (tx, _) = tokio::sync::broadcast::channel(32);
        Mailbox { tx }
    }
}

impl<T: 'static> typemap::Key for Mailbox<T> {
    type Value = Mailbox<T>;
}

static POST_OFFICE: OnceCell<Mutex<SendMap>> = OnceCell::new();

pub fn open() -> Result<()> {
    POST_OFFICE
        .set(Mutex::new(SendMap::custom()))
        .map_err(|_| ())
        .expect("postoffice is already open");

    Ok(())
}

async fn with_mailbox<T: Clone + Send + 'static, F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut Mailbox<T>) -> Result<R>,
{
    let mut postoffice = POST_OFFICE
        .get()
        .expect("postoffice is not open yet")
        .lock()
        .await;

    let mailbox = postoffice
        .entry::<Mailbox<T>>()
        .or_insert_with(|| Mailbox::<T>::new());

    f(mailbox)
}

pub async fn receive_mail<T: Clone + Send + 'static>() -> Result<Receiver<T>> {
    with_mailbox(|mailbox| Ok(mailbox.tx.subscribe())).await
}

pub async fn post_mail<T: Clone + Send + 'static>() -> Result<Sender<T>> {
    with_mailbox(|mailbox| Ok(mailbox.tx.clone())).await
}
