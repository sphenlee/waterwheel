use anyhow::Result;
use once_cell::sync::OnceCell;
use postage::dispatch::{Receiver, Sender};
use tokio::sync::Mutex;
use typemap::SendMap;

struct Mailbox<T> {
    tx: Sender<T>,
}

impl<T: Clone> Mailbox<T> {
    fn new() -> Mailbox<T> {
        let (tx, _) = postage::dispatch::channel(128);
        Mailbox { tx }
    }
}

impl<T: 'static> typemap::Key for Mailbox<T> {
    type Value = Mailbox<T>;
}

static POST_OFFICE: OnceCell<Mutex<SendMap>> = OnceCell::new();

async fn with_mailbox<T: Clone + Send + 'static, F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut Mailbox<T>) -> Result<R>,
{
    let mut postoffice = POST_OFFICE
        .get_or_init(|| Mutex::new(SendMap::custom()))
        .lock()
        .await;

    let mailbox = postoffice
        .entry::<Mailbox<T>>()
        .or_insert_with(Mailbox::<T>::new);

    f(mailbox)
}

pub async fn receive_mail<T: Clone + Send + 'static>() -> Result<Receiver<T>> {
    with_mailbox(|mailbox| Ok(mailbox.tx.subscribe())).await
}

pub async fn post_mail<T: Clone + Send + 'static>() -> Result<Sender<T>> {
    with_mailbox(|mailbox| Ok(mailbox.tx.clone())).await
}
