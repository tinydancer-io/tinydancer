use async_trait::async_trait;

use crate::tinydancer::{GenericService, TinyDancer};

pub struct UiService {
    pub views: Vec<String>, // placeholder
}
pub struct UiConfig {}
#[async_trait]
impl GenericService<UiConfig> for UiService {
    fn new(config: UiConfig) -> Self {
        todo!();
    }
    async fn join(self) {
        todo!();
    }
}
