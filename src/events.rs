use std::process::ChildStdin;

use ratatui::crossterm::event::Event;

use crate::record::Record;

pub enum TuiEvent {
    // From the keyboard handler
    Key(Event),

    // From the RecordList
    NewRecord(Record),

    // From the state handler
    RefreshScreen,
    Pause,
    Resume,
}
