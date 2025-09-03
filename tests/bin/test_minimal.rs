use crossterm::event::{self, Event};
use ratatui::{text::Text, Frame};

fn main() {
    let mut terminal = ratatui::init();
    loop {
        terminal.draw(draw).expect("failed to draw frame");
        if matches!(event::read().expect("failed to read event"), Event::Key(_)) {
            break;
        }
    }
    ratatui::restore();
}

fn draw(frame: &mut Frame) {
    let text = Text::raw("Hello World! Press any key to exit");
    frame.render_widget(text, frame.area());
}