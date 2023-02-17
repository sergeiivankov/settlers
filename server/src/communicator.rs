use fastrand::Rng;
use log::debug;
use std::{ collections::HashMap, sync::Arc };
use tokio::sync::{ mpsc::{ UnboundedReceiver, UnboundedSender, unbounded_channel }, Mutex };

pub type Data = (u32, String);
pub type Sender = UnboundedSender<Data>;
pub type Receiver = UnboundedReceiver<Data>;

pub struct Communicator {
  rng: Rng,
  peers: HashMap<u32, UnboundedSender<String>>,
  sender: Sender
}

impl Communicator {
  pub fn new() -> (Arc<Mutex<Self>>, Receiver) {
    let (sender, receiver) = unbounded_channel();

    let this = Self {
      rng: Rng::new(),
      peers: HashMap::new(),
      sender
    };

    (Arc::new(Mutex::new(this)), receiver)
  }

  pub fn add(&mut self) -> (u32, Sender, UnboundedReceiver<String>) {
    let (peer_sender, peer_receiver) = unbounded_channel();

    let id = self.generate_id();

    self.peers.insert(id, peer_sender);

    (id, self.sender.clone(), peer_receiver)
  }

  pub fn remove(&mut self, id: &u32) {
    self.peers.remove(&id);
  }

  pub fn send(&self, id: &u32, data: String) -> bool {
    let sender = match self.peers.get(id) {
      Some(sender) => sender,
      None => return false
    };

    match sender.send(data) {
      Ok(_) => true,
      Err(err) => {
        debug!("Send data to peer {} error: {}", id, err);
        false
      }
    }
  }

  fn generate_id(&self) -> u32 {
    loop {
      let random = self.rng.u32(..);
      if self.peers.contains_key(&random) {
        continue
      }
      return random
    }
  }
}