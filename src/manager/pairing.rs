use std::net::{Ipv4Addr, SocketAddr};
use tokio::{net::UdpSocket, sync::oneshot};

const SOCK_REV_ADDR: &str = "0.0.0.0:9487";
const SOCK_SEND_ADDR: &str = "0.0.0.0:0";
const MULTICAST_ADDR: &str = "239.255.94.87";
const MULTICAST_ADDR_SEND: &str = "239.255.94.87:9487";
const INTERFACE_ADDR: &str = "0.0.0.0";

const BUFFER_SIZE: usize = 14;
const VERIFY_PASS: [u8; BUFFER_SIZE] = [69, 108, 32, 80, 115, 121, 32, 67, 111, 110, 103, 114, 111, 111];

pub struct PairingManager {
    pub sender: PairingSender,
    pub listener: PairingListener,
}

impl PairingManager {
    pub async fn new() -> anyhow::Result<Self> {
        let socket_rev = UdpSocket::bind(SOCK_REV_ADDR).await?;
        let multicast_address = MULTICAST_ADDR.parse::<Ipv4Addr>()?;
        let interface = INTERFACE_ADDR.parse::<Ipv4Addr>()?;
        socket_rev.join_multicast_v4(multicast_address, interface)?;

        let socket_send = UdpSocket::bind(SOCK_SEND_ADDR).await?;
        socket_send.set_broadcast(true)?;

        Ok(Self {
            sender: PairingSender { socket: socket_send },
            listener: PairingListener::new(socket_rev),
        })
    }

    pub fn split(self) -> (PairingSender, PairingListener) {
        (self.sender, self.listener)
    }
}

pub struct PairingSender {
    socket: UdpSocket,
}

impl PairingSender {
    pub async fn try_pairing(&mut self) -> anyhow::Result<()> {
        self.socket.send_to(&VERIFY_PASS, MULTICAST_ADDR_SEND).await?;
        Ok(())
    }
}

pub struct PairingListener
{
    socket: UdpSocket,
    quit_tx: Option<oneshot::Sender<()>>,
    buf: [u8; BUFFER_SIZE],
}

impl PairingListener
{
    pub fn new(socket: UdpSocket) -> Self {
        Self {
            socket,
            quit_tx: None,
            buf: [0; BUFFER_SIZE],
        }
    }

    fn verify(&self) -> bool {
        self.buf == VERIFY_PASS
    }

    async fn listen(&mut self, on_pair: Box<dyn Fn(SocketAddr) + Send>) -> anyhow::Result<()> {
        loop {
            let (amt, src) = self.socket.recv_from(&mut self.buf).await?;
            if amt < BUFFER_SIZE {
                continue;
            }

            if self.verify() && src != self.socket.local_addr()? {
                (on_pair)(src);
            }
        }
    }

    pub async fn event_loop(&mut self, on_pair: Box<dyn Fn(SocketAddr) + Send>) -> anyhow::Result<()> {
        if self.quit_tx.is_some() {
            return Ok(());
        }

        let (quit_tx, quit_rx) = oneshot::channel::<()>();
        self.quit_tx = Some(quit_tx);

        tokio::select! {
            _ = quit_rx => {},
            _ = async move { self.listen(on_pair).await } => {}
        }

        Ok(())
    }

    pub fn quit(&mut self) {
        self.quit_tx.take().map(|tx| tx.send(()));
    }
}
