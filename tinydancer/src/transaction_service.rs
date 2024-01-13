use crate::tinydancer::{ClientService, Cluster};
use async_trait::async_trait;
use tokio::task::JoinHandle;

pub struct TransactionService {
    pub handler: JoinHandle<()>,
}

pub struct TransactionServiceConfig {
    pub cluster: Cluster,
}

#[async_trait]
impl ClientService<TransactionServiceConfig> for TransactionService {
    type ServiceError = tokio::task::JoinError;

    fn new(config: TransactionServiceConfig) -> Self {
        let handler = tokio::spawn(async move {});
        Self { handler }
    }
    async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        self.handler.await
    }
}
