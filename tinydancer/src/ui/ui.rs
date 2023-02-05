use crate::tinydancer::{ClientService, TinyDancer};
use async_trait::async_trait;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{any::Any, thread::Thread};
use std::{fmt, thread::JoinHandle};
use thiserror::Error;
use tui::{
    backend::{Backend, CrosstermBackend},
    widgets::{
        Axis, BarChart, Block, Borders, Cell, Chart, Dataset, Gauge, LineGauge, List, ListItem,
        Paragraph, Row, Sparkline, Table, TableState, Tabs, Wrap,
    },
    Frame, Terminal,
};

pub struct UiService {
    //pub views: Vec<String>, //placeholder
    pub ui_service_handle: JoinHandle<()>, // pub table: TableState,  // placeholder view
}

pub struct App {
    pub table: TableState,
    pub items: Vec<Vec<String>>,
}
impl App {}
pub struct UiConfig {}

#[async_trait]
impl ClientService<UiConfig> for UiService {
    type ServiceError = ThreadJoinError;
    fn new(config: UiConfig) -> Self {
        let ui_service_handle = std::thread::spawn(|| loop {
            println!("rendering ui");
            std::thread::sleep(std::time::Duration::from_secs(2));
        });
        Self { ui_service_handle }
    }
    async fn join(self) -> std::result::Result<(), Self::ServiceError> {
        match self.ui_service_handle.join() {
            Ok(_) => Ok(()),
            Err(error) => Err(ThreadJoinError { error }),
        }
    }
}

#[derive(Debug, Error)]
pub struct ThreadJoinError {
    error: Box<dyn Any + Send>,
}

// impl ThreadJoinError {
//     fn new<E: Any + Send>(msg: Box<E>) -> ThreadJoinError {
//         ThreadJoinError { error: msg }
//     }
// }

impl fmt::Display for ThreadJoinError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.error)
    }
}

// impl Error for ThreadJoinError {
//     fn description(&self) -> &str {
//         &self.error.into()
//     }
// }
