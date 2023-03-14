use crate::sampler::GetShredResponse;
use crate::tinydancer::{ClientService, TinyDancer, Cluster};
use async_trait::async_trait;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{any::Any, thread::Thread};
use std::{fmt, thread::JoinHandle};
use thiserror::Error;
use tui::layout::{Rect, Corner};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::Direction,
    layout::{Constraint, Layout},
    widgets::{
        Axis, BarChart, Block, Borders, Cell, Chart, Dataset, Gauge, LineGauge, List, ListItem,
        ListState, Paragraph, Row, Sparkline, Table, TableState, Tabs, Wrap,
    },
    Frame, Terminal,
};



pub struct App {
    pub title: String,
    pub tabs: TabsState,
    pub slot_list: SlotList,
    pub verification_stats_list: VerificationStatList,
    pub per_request_sample_stat_list: PerRequestSampleStatsList,
    //pub peers_list: StatefulList<(String, usize)>,
    //pub full_nodes_list: StatefulList<(String, usize)>,
    pub should_quit: bool,

}
impl App{
   pub fn new(title: String, 
    slot_list: Vec<usize>,
    r_list: Vec<(usize, usize, usize, usize)>,
    v_list: Vec<(usize, usize, usize)>,
    //peers_list: Vec<(String, usize)>,
    //full_nodes_list: Vec<(String, usize)>
   ) -> App{
    App{
        title,
        tabs: TabsState::new(vec!["Sampler".to_string()]),
        slot_list: SlotList::new("SLOTS".to_string(), slot_list),
        per_request_sample_stat_list: PerRequestSampleStatsList::new("Request Stats".to_string(), r_list),
        verification_stats_list: VerificationStatList::new("Verified Stats".to_string(), v_list),
        //peers_list: StatefulList::with_items(peers_list),
        //full_nodes_list: StatefulList::with_items(full_nodes_list),
        should_quit: true,
     }
    }
    pub fn on_up(&mut self) {
        self.slot_list.state.previous();
    }
    pub fn on_down(&mut self) {
        self.slot_list.state.next();
    }
    pub fn on_right(&mut self) {
        self.tabs.next();
    }
    pub fn on_left(&mut self) {
        self.tabs.previous();
    }
    pub fn on_tick(&mut self) {
         self.slot_list.state.on_tick();
         self.per_request_sample_stat_list.state.on_tick();
         self.verification_stats_list.state.on_tick();
    }
    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => {
                self.should_quit = true;
            }
            _ => {}
        }
    }
}

pub struct SlotList {
    title: String,
    state: StatefulList<usize>
}
impl SlotList{
    pub fn new(title: String, state: Vec<usize>) -> SlotList{
        SlotList { 
            title, 
            state: StatefulList::with_items(state),
        }
    }
}
pub struct PerRequestSampleStatsList{
    title: String,
    state: StatefulList<(usize, usize, usize, usize)>,
}
impl PerRequestSampleStatsList{
    pub fn new(title: String, state: Vec<(usize, usize, usize, usize)>) -> PerRequestSampleStatsList{
        PerRequestSampleStatsList{
            title,
            state: StatefulList::with_items(state)
        }
    }
}
pub struct VerificationStatList{
    title: String,
    state: StatefulList<(usize, usize, usize)>,
}
impl VerificationStatList{
    pub fn new(title: String, state: Vec<(usize, usize, usize)>) -> VerificationStatList{
        VerificationStatList{
            title,
            state: StatefulList::with_items(state)
        }
    }
}
pub struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}
impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
    }
   pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
   pub fn unselect(&mut self) {
        self.state.select(None);
    }
    pub fn on_tick(&mut self) {
        let item = self.items.remove(0);
        self.items.push(item);
    }
}

pub struct TabsState {
    pub titles: Vec<String>,
    pub index: usize,
}
impl TabsState {
    pub fn new(titles: Vec<String>) -> TabsState {
        TabsState { titles, index: 0 }
    }
    pub fn next(&mut self) {
        self.index = self.index + 1
    }

    pub fn previous(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
        else {
            self.index = self.titles.len() - 1;
        }
    }
}

// main draw function
pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Length(3),Constraint::Min(0)].as_ref())
        .split(f.size());
    
    let titles =  app
    .tabs
    .titles
    .iter()
    .map(|t| Spans::from(Span::styled(t, Style::default().fg(Color::Yellow)))).collect();
    
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.title.clone()),
        )
        .highlight_style(Style::default().bg(Color::White).fg(Color::Green))
        .select(app.tabs.index);
    // let block = Block::default().borders(Borders::ALL).title(Span::styled(
    //     "Footer",
    //     Style::default()
    //         .fg(Color::Magenta)
    //         .add_modifier(Modifier::BOLD),
    // ));
   f.render_widget(tabs, chunks[0]);
    match app.tabs.index{
        0 => draw_first_tab(f, app, chunks[1]),
       // 1 => draw_second_tab(f, app, chunks[1]),
        _ => {}
    }

}

fn draw_first_tab<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Length(40),
                Constraint::Length(40),
            ]
            .as_ref(),
        )
        .split(area);

    //ui(f, app, chunks[0]);
    draw_list_one(f, app, chunks[0]);
    draw_list_two(f, app, chunks[1]);
    draw_list_three(f, app, chunks[2]);
}
    
// }
fn draw_second_tab<B>(f: &mut Frame<B>, app: &mut App, area: Rect)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Length(20),
                Constraint::Length(3),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .split(area);
     //todo: Draw network usage metrics here...
     draw_list_one(f, app, chunks[0]);
     draw_list_two(f, app, chunks[0]);
     draw_list_three(f, app, chunks[1]);

}



// draws list of slots
fn draw_list_one<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    // let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    // let normal_style = Style::default().bg(Color::LightBlue);
    let slots: Vec<ListItem> = app
        .slot_list
        .state
        .items
        .iter()
        .map(|i| {
            let mut lines = vec![Spans::from(Span::styled("Slot".to_string(), Style::default().add_modifier(Modifier::BOLD)))];
            
                lines.push(Spans::from(Span::styled(
                    // dummy values in list that would be replaced by slot numbers
                    i.to_string(),
                    Style::default().add_modifier(Modifier::ITALIC),
                )));
        
            ListItem::new(lines).style(Style::default().fg(Color::LightGreen).bg(Color::Black))
        })
        .collect();
    let list = List::new(slots)
            .block(Block::default().borders(Borders::ALL).title("SLOTS"))
            .highlight_style(Style::default().bg(Color::Cyan).add_modifier(Modifier::BOLD))
            .highlight_symbol(">>");
    f.render_stateful_widget(list, area, &mut app.slot_list.state.state);
    
    
    
}

fn draw_list_two<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect) {
    let r_list: Vec<ListItem> = app
        .per_request_sample_stat_list
        .state
        .items
        .iter()
        .map(|i|{
            let slot_span = Spans::from(vec![
                Span::styled(format!("{:<8}", "Slot: "),Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(i.0.to_string(), Style::default().fg(Color::LightBlue))
            ]);
            let total_sampled_span = Spans::from(vec![
                Span::styled(format!("{:<8}", "TotalSampled:"),Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(i.1.to_string(), Style::default().fg(Color::LightBlue))
            ]); 
            let data_shred_span = Spans::from(vec![
                Span::styled(format!("{:<8}", "DataShreds:"),Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(i.2.to_string(), Style::default().fg(Color::LightBlue))
            ]);
            let coding_shred_span = Spans::from(vec![
                Span::styled(format!("{:<8}", "CodingShreds:"),Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(i.3.to_string(), Style::default().fg(Color::LightBlue))
            ]);
            ListItem::new(vec![
                Spans::from("-".repeat(area.width as usize)),
                slot_span,
                total_sampled_span,
                data_shred_span,
                coding_shred_span,
                Spans::from("-".repeat(area.width as usize))
            ]).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        })
        .collect();
     
    let r_list = List::new(r_list)
                            .block(Block::default().borders(Borders::ALL).title("PER REQUEST SAMPLE STATS"))
                            .start_corner(Corner::BottomRight)
                            .highlight_style(Style::default().bg(Color::Cyan).add_modifier(Modifier::BOLD))
                            .highlight_symbol(">>");
    f.render_stateful_widget(r_list, area, &mut app.per_request_sample_stat_list.state.state);
}

fn draw_list_three<B: Backend>(f: &mut Frame<B>, app: &mut App, area: Rect){
    let ve_list: Vec<ListItem> = app
        .verification_stats_list
        .state
        .items
        .iter()
        .map(|i|{

            let slot_span = Spans::from(vec![
                Span::styled(format!("{:<8}", "Slot: "),Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(i.0.to_string(), Style::default().fg(Color::LightBlue))
            ]);

            let verified_span = Spans::from(vec![
                Span::styled(format!("{:<8}", "Successfully Verified: "),Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(i.1.to_string(), Style::default().fg(Color::LightBlue))
            ]); 
            let failed_span = Spans::from(vec![
                Span::styled(format!("{:<8}", "Failed: "),Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(i.2.to_string(), Style::default().fg(Color::LightBlue))
            ]); 
            ListItem::new(vec![
                Spans::from("-".repeat(area.width as usize)),
                slot_span,
                verified_span,
                failed_span,
                Spans::from("-".repeat(area.width as usize))
            ]).style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        })
        .collect();
     
    let v_list = List::new(ve_list)
                            .block(Block::default().borders(Borders::ALL).title("VERIFICATION STATS"))
                            .start_corner(Corner::BottomRight)
                            .highlight_style(Style::default().bg(Color::Cyan).add_modifier(Modifier::BOLD))
                            .highlight_symbol(">>");
    f.render_stateful_widget(v_list, area, &mut app.verification_stats_list.state.state);
                
}


