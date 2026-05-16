use ratatui::style::{Color, Style};

pub const CLR_BKG_BLACK: Color = Color::Indexed(0);
pub const CLR_BKG_ANSWER: Color = Color::Indexed(236);

pub const CLR_TXT_ANSWER: Color = Color::Indexed(252);
pub const CLR_TXT_MUTED: Color = Color::Indexed(244);
pub const CLR_TXT_READY: Color = Color::Green;
pub const CLR_TXT_WAITING: Color = Color::Yellow;
pub const CLR_TXT_ERR: Color = Color::Red;

pub const STL_BKG: Style = Style::new().bg(CLR_BKG_BLACK);
pub const STL_ANSWER: Style = Style::new().fg(CLR_TXT_ANSWER).bg(CLR_BKG_ANSWER);
pub const STL_INPUT: Style = Style::new();
pub const STL_INPUT_WAITING: Style = Style::new().fg(CLR_TXT_MUTED);
pub const STL_STATUS_READY: Style = Style::new().fg(CLR_TXT_READY);
pub const STL_STATUS_WAITING: Style = Style::new().fg(CLR_TXT_WAITING);
pub const STL_STATUS_ERR: Style = Style::new().fg(CLR_TXT_ERR);
pub const STL_FOOTER: Style = Style::new().fg(CLR_TXT_MUTED);
