use anyhow::Result;
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

pub struct PostOffice(Mutex<SendMap>);

impl PostOffice {
    pub fn open() -> Self {
        Self(Mutex::new(SendMap::custom()))
    }

    async fn with_mailbox<T: Clone + Send + 'static, F, R>(&self, f: F) -> Result<R>
        where
            F: FnOnce(&mut Mailbox<T>) -> Result<R>,
    {
        let mut postoffice = self.0.lock().await;

        let mailbox = postoffice
            .entry::<Mailbox<T>>()
            .or_insert_with(Mailbox::<T>::new);

        f(mailbox)
    }

    pub async fn receive_mail<T: Clone + Send + 'static>(&self) -> Result<Receiver<T>> {
        self.with_mailbox(|mailbox| Ok(mailbox.tx.subscribe())).await
    }

    pub async fn post_mail<T: Clone + Send + 'static>(&self) -> Result<Sender<T>> {
        self.with_mailbox(|mailbox| Ok(mailbox.tx.clone())).await
    }
}
