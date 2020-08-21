use anyhow::Result;
use async_std::sync::{Mutex, Receiver, Sender};
use once_cell::sync::OnceCell;
use typemap::SendMap;

struct Mailbox<T> {
    tx: Sender<T>,
    rx: Option<Receiver<T>>,
}

impl<T> Mailbox<T> {
    fn new() -> Mailbox<T> {
        let (tx, rx) = async_std::sync::channel(32);
        Mailbox { tx, rx: Some(rx) }
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

async fn with_mailbox<T: Send + 'static, F, R>(f: F) -> Result<R>
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

pub async fn receive_mail<T: Send + 'static>() -> Result<Receiver<T>> {
    let rx = with_mailbox(|mailbox| match mailbox.rx.take() {
        Some(rx) => Ok(rx),
        None => panic!("someone has already claimed this mailbox"),
    })
    .await?;

    Ok(rx)
}

pub async fn post_mail<T: Send + 'static>() -> Result<Sender<T>> {
    let tx = with_mailbox(|mailbox| Ok(mailbox.tx.clone())).await?;

    Ok(tx)
}
