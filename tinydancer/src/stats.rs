//todo
use tokio::task::JoinHandle;
pub struct StatsService {
    stat_handle: JoinHandle<()>,
}

pub struct SampleStats {}
