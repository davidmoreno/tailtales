use ratatui::crossterm::event::Event;

pub enum TuiEvent {
    Key(Event),
}
