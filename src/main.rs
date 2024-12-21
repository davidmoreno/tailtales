use std::{thread, time::Duration};

mod record;
mod tuichrome;

fn main() {
    // get a filename from the command line
    let filename = std::env::args().nth(1).expect("need a filename");
    println!("reading from file: {}", filename);

    let mut tui_chrome = tuichrome::TuiChrome::new().expect("could not create TuiChrome");
    tui_chrome.state.records.readfile(&filename);
    tui_chrome.draw().expect("could not draw TuiChrome");
    thread::sleep(Duration::from_secs(5));
}
