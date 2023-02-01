use async_trait::async_trait;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    widgets::{
        Axis, BarChart, Block, Borders, Cell, Chart, Dataset, Gauge, LineGauge, List, ListItem,
        Paragraph, Row, Sparkline, Table, TableState, Tabs, Wrap,
    },
    Frame, Terminal,
};

use crate::tinydancer::{GenericService, TinyDancer};

pub struct UiService {
    //pub views: Vec<String>, //placeholder
    pub ui_service_handle: tokio::task::JoinHandle<()>, // pub table: TableState,  // placeholder view
}

pub struct App {
    pub table: TableState,
    pub items: Vec<Vec<String>>,
}
impl App {}
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
