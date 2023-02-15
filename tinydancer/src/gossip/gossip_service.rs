use crate::tinydancer::ClientService;
use async_trait::async_trait;
use tokio::task::JoinHandle;

pub struct GossipService {
    pub gossip_handle: JoinHandle<()>,
}
pub struct GossipConfig {}

#[async_trait]
impl ClientService<GossipConfig> for GossipService {
    type ServiceError = tokio::task::JoinError;
    fn new(_config: GossipConfig) -> Self {
        todo!()
    }
    async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        self.gossip_handle.await
    }
}
