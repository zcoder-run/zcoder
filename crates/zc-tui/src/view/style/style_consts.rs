#![allow(unused)]

use ratatui::style::{Color, Style};

// -- Scale - Txt Colors
pub const TXT_100: Color = Color::Indexed(255);
pub const TXT_200: Color = Color::Indexed(254);
pub const TXT_300: Color = Color::Indexed(253);
pub const TXT_500: Color = Color::Indexed(252);
pub const TXT_600: Color = Color::Indexed(248);
pub const TXT_700: Color = Color::Indexed(244);
pub const TXT_800: Color = Color::Indexed(242);
pub const TXT_900: Color = Color::Indexed(240);

pub const BKG_APP: Color = Color::Indexed(0);
pub const BKG_PANEL: Color = Color::Indexed(234);
pub const BKG_INPUT: Color = BKG_APP;

pub const BDR_DIVIDER: Color = Color::Indexed(236);

pub const TXT_HIGHLIGHT: Color = TXT_100;
pub const TXT_PRIME: Color = TXT_500;
pub const TXT_SECOND: Color = TXT_600;
pub const TXT_MUTED: Color = TXT_700;
pub const TXT_DIM: Color = TXT_900;
pub const TXT_ERROR: Color = Color::Indexed(196);

pub const STL_BKG: Style = Style::new().bg(BKG_APP);
pub const STL_ANSWER: Style = Style::new().fg(TXT_SECOND).bg(BKG_PANEL);

// -- INPUT
pub const STL_INPUT: Style = Style::new().fg(TXT_HIGHLIGHT).bg(BKG_INPUT);
pub const STL_INPUT_WAITING: Style = Style::new().fg(TXT_MUTED).bg(BKG_INPUT);
pub const STL_INPUT_BORDER: Style = Style::new().fg(BDR_DIVIDER).bg(BKG_INPUT);

// -- Status
pub const STL_STATUS_READY: Style = Style::new().fg(TXT_MUTED).bg(BKG_APP);
pub const STL_STATUS_WAITING: Style = Style::new().fg(TXT_MUTED).bg(BKG_APP);
pub const STL_STATUS_ERR: Style = Style::new().fg(TXT_ERROR).bg(BKG_APP);

// -- Footer
pub const STL_FOOTER: Style = Style::new().fg(TXT_DIM).bg(BKG_APP);

pub const STL_SYS_STAT_LBL: Style = Style::new().fg(TXT_MUTED).bg(BKG_PANEL);
pub const STL_SYS_STAT_VAL: Style = Style::new().fg(TXT_SECOND).bg(BKG_APP);
