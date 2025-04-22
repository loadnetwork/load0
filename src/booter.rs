use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;

pub struct Booter {
    pub port: u16,
    tcp_listener: TcpListener,
}

impl Booter {
    pub async fn new(port: Option<u16>) -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(port.unwrap_or(3000));

        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        let listener = TcpListener::bind(addr).await.unwrap();

        Self {
            port,
            tcp_listener: listener,
        }
    }

    pub async fn start(self, router: Router) {
        axum::serve(
            self.tcp_listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap()
    }
}
