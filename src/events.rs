use ratatui::crossterm::event::Event;

use crate::record::Record;

pub enum TuiEvent {
    Key(Event),
    NewRecord(Record),
}
