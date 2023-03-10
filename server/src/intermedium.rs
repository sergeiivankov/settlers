use log::debug;
use std::sync::Arc;
use tokio::{ sync::{ oneshot::Receiver as OneshotReceiver, Mutex }, select };
use crate::{ communicator::{ Communicator, Data, Receiver } };

pub struct Intermedium {
  communicator: Arc<Mutex<Communicator>>,
  receiver: Receiver
}

impl Intermedium {
  pub fn new(communicator: Arc<Mutex<Communicator>>, receiver: Receiver) -> Self {
    Self {
      communicator,
      receiver
    }
  }

  async fn send(&self, id: u32, data: String) -> bool {
    let communicator_lock = self.communicator.lock().await;
    communicator_lock.send(id, data)
  }

  async fn receive(&mut self) -> Data {
    let data_option = self.receiver.recv().await;
    // SAFETY: `recv` method return None variant if all senders have been dropped or closed,
    //         but sender live in Communicator, one of which references store in Intermedium
    //         and dropped with it
    unsafe { data_option.unwrap_unchecked() }
  }

  pub async fn run(&mut self, mut stop_receiver: OneshotReceiver<()>) {
    loop {
      select! {
        (id, data) = self.receive() => {
          println!("Data received {id}: {data}");
          let result = self.send(id, data).await;
          println!("Send result {id}: {result}");
        },
        _ = &mut stop_receiver => {
          debug!("Graceful intermedium shutdown");
          break
        }
      }
    }
  }
}