use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use crate::sampler::GetShredResponse;
use async_trait::async_trait;
use std::{any::Any, thread::Thread};
use std::{fmt, thread::JoinHandle};
use crate::tinydancer::{ClientService, TinyDancer, Cluster};
use crossbeam::channel::{Receiver, Sender};
use thiserror::Error;
use crate::stats::{PerRequestSampleStats, PerRequestVerificationStats, SlotUpdateStats};
use tokio::time::sleep;
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use crate::ui::{App};

use super::draw;

pub struct UiConfig {
    cluster: Cluster
}
pub struct UiService {
    //pub views: Vec<String>, //placeholder
    s_stats: Receiver<SlotUpdateStats>,
    r_stats: Receiver<PerRequestSampleStats>,
    v_stats: Receiver<PerRequestVerificationStats>,
    pub ui_service_handle: JoinHandle<()>, // pub table: TableState,  // placeholder view
}
// #[async_trait]
// impl ClientService<UiConfig> for UiService {
//     type ServiceError = ThreadJoinError;
//    fn new(config: UiConfig) -> Self {
//         let ui_service_handle = std::thread::spawn(||  {
//             std::thread::spawn(|| start_ui_loop(s_stats, self.r_stats, self.v_stats));
//             //start_ui_loop(config.s_stats, config.r_stats, config.v_stats);
//             std::thread::sleep(std::time::Duration::from_secs(2));
//         });
//         let s_stats = Receiver::from(SlotUpdateStats::);
//         Self { ui_service_handle }
//     }
//     async fn join(self) -> std::result::Result<(), Self::ServiceError> {
//         match self.ui_service_handle.join() {
//             Ok(_) => Ok(()),
//             Err(error) => Err(ThreadJoinError { error }),
//         }
//     }
// }


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




pub fn display( 
    slot_list: Vec<usize>,
    r_list: Vec<(usize, usize, usize, usize)>,
    v_list: Vec<(usize, usize, usize)>,
) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new("Sampler Statistics".to_string(), slot_list, r_list, v_list);
    let res = run_app(&mut terminal, app, Duration::from_millis(1000));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
   // let events = Events::new(Duration::from_millis(200));
    let mut last_tick =  Instant::now();
    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => app.on_key(c),
                    KeyCode::Left => app.on_left(),
                    KeyCode::Up => app.on_up(),
                    KeyCode::Right => app.on_right(),
                    KeyCode::Down => app.on_down(),
                    
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }
        if app.should_quit {
            return Ok(());
        }
    }
}

pub async fn start_ui_loop(
   s_stats: Receiver<SlotUpdateStats>,
   r_stats: Receiver<PerRequestSampleStats>,
   v_stats: Receiver<PerRequestVerificationStats>,
){
  
    loop{
        
        let (sx,rx, vx  )= (s_stats.recv(),r_stats.recv(), v_stats.recv());
        if sx.is_ok() && rx.is_ok() && vx.is_ok(){
            let mut s_vec = vec![];
            let mut r_vec = vec![];
            let mut v_vec = vec![];
            s_vec.push( sx.unwrap().slots);
            r_vec.push((rx.unwrap().slot as usize, rx.unwrap().total_sampled, rx.unwrap().num_data_shreds, rx.unwrap().num_coding_shreds));
            v_vec.push((vx.unwrap().slot as usize, vx.unwrap().num_verified, vx.unwrap().num_failed));
            display(s_vec, r_vec, v_vec).expect("TOTALLY FAILED");
        }
        else{
            break;
        }
    }
}