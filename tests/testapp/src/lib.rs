use anyhow::Result;
use bytes::Bytes;
use lol2::client::*;
use tonic::transport::Channel;

mod proto {
    tonic::include_proto!("testapp");
}
pub use proto::ping_client::PingClient;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum AppWriteRequest {
    FetchAdd { bytes: Vec<u8> },
}
impl AppWriteRequest {
    pub fn serialize(self) -> Bytes {
        bincode::serialize(&self).unwrap().into()
    }
    pub fn deserialize(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum AppReadRequest {
    Read,
    MakeSnapshot,
}
impl AppReadRequest {
    pub fn serialize(self) -> Bytes {
        bincode::serialize(&self).unwrap().into()
    }
    pub fn deserialize(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct AppState(pub u64);
impl AppState {
    pub fn serialize(&self) -> Bytes {
        bincode::serialize(&self).unwrap().into()
    }
    pub fn deserialize(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

pub struct Client {
    cli: RaftClient,
}
impl Client {
    pub fn new(conn: Channel) -> Self {
        let cli = RaftClient::new(conn);
        Self { cli }
    }
    pub async fn fetch_add(&mut self, n: u64) -> Result<u64> {
        let req = Request {
            message: AppWriteRequest::FetchAdd {
                bytes: vec![1u8; n as usize].into(),
            }
            .serialize(),
            mutation: true,
        };
        let resp = self.cli.process(req).await?.into_inner();
        let resp = AppState::deserialize(&resp.message);
        Ok(resp.0)
    }
    pub async fn read(&self) -> Result<u64> {
        let req = Request {
            message: AppReadRequest::Read.serialize(),
            mutation: false,
        };
        let resp = self.cli.clone().process(req).await?.into_inner();
        let resp = AppState::deserialize(&resp.message);
        Ok(resp.0)
    }
    pub async fn make_snapshot(&self) -> Result<u64> {
        let req = Request {
            message: AppReadRequest::MakeSnapshot.serialize(),
            mutation: false,
        };
        let resp = self.cli.clone().process(req).await?.into_inner();
        let resp = AppState::deserialize(&resp.message);
        Ok(resp.0)
    }
}